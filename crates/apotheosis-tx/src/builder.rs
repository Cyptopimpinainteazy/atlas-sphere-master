//! Apotheosis Transaction Builder

use crate::{
    error::{ApotheosisError, ApotheosisResult},
    types::*,
    MAX_ASSETS_PER_CHAIN, MAX_SOURCE_CHAINS,
};
use chrono::Utc;
use rand::Rng;

/// Builder for constructing Apotheosis transactions
#[derive(Debug, Default)]
pub struct ApotheosisBuilder {
    /// Source chains to migrate from
    sources: Vec<SourceChain>,
    /// Destination configuration
    destination: Option<Destination>,
    /// Custom routes (if not auto-calculated)
    custom_routes: Option<Vec<MigrationRoute>>,
    /// Minimum acceptable value after fees
    min_received_value: Option<f64>,
    /// Maximum acceptable gas (USD)
    max_gas_cost: Option<f64>,
    /// Deadline for completion
    deadline_seconds: Option<u64>,
    /// Whether to use optimal routing
    optimize_routes: bool,
    /// Risk tolerance (0-100)
    risk_tolerance: u8,
}

impl ApotheosisBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            destination: None,
            custom_routes: None,
            min_received_value: None,
            max_gas_cost: None,
            deadline_seconds: None,
            optimize_routes: true,
            risk_tolerance: 50,
        }
    }

    /// Add a source chain
    pub fn from_chain(mut self, chain: SourceChain) -> Self {
        self.sources.push(chain);
        self
    }

    /// Add multiple source chains
    pub fn from_chains(mut self, chains: Vec<SourceChain>) -> Self {
        self.sources.extend(chains);
        self
    }

    /// Sweep all assets from a chain
    pub fn sweep_chain(mut self, chain_id: ChainId, address: impl Into<String>) -> Self {
        self.sources.push(SourceChain {
            chain_id,
            address: address.into(),
            assets: Vec::new(), // Will be populated by discovery
            estimated_gas: 0,
            is_reachable: true,
        });
        self
    }

    /// Set destination
    pub fn to(mut self, chain_id: ChainId, address: impl Into<String>) -> Self {
        self.destination = Some(Destination {
            chain_id,
            address: address.into(),
            create_if_needed: true,
            gas_token: None,
        });
        self
    }

    /// Set destination with full config
    pub fn to_destination(mut self, destination: Destination) -> Self {
        self.destination = Some(destination);
        self
    }

    /// Set minimum value to receive after fees
    pub fn min_value(mut self, min_usd: f64) -> Self {
        self.min_received_value = Some(min_usd);
        self
    }

    /// Set maximum acceptable gas cost
    pub fn max_gas(mut self, max_usd: f64) -> Self {
        self.max_gas_cost = Some(max_usd);
        self
    }

    /// Set deadline for completion
    pub fn deadline(mut self, seconds: u64) -> Self {
        self.deadline_seconds = Some(seconds);
        self
    }

    /// Enable/disable route optimization
    pub fn optimize(mut self, optimize: bool) -> Self {
        self.optimize_routes = optimize;
        self
    }

    /// Set risk tolerance (0 = no risk, 100 = max risk)
    pub fn risk_tolerance(mut self, tolerance: u8) -> Self {
        self.risk_tolerance = tolerance.min(100);
        self
    }

    /// Use custom routes instead of auto-calculated
    pub fn with_routes(mut self, routes: Vec<MigrationRoute>) -> Self {
        self.custom_routes = Some(routes);
        self
    }

    /// Build the Apotheosis transaction
    pub fn build(self) -> ApotheosisResult<ApotheosisTransaction> {
        // Validate destination - clone to avoid move
        let destination = match self.destination.clone() {
            Some(d) => d,
            None => return Err(ApotheosisError::NoDestination),
        };

        // Validate sources
        if self.sources.is_empty() {
            return Err(ApotheosisError::NoSourceChains);
        }

        if self.sources.len() > MAX_SOURCE_CHAINS {
            return Err(ApotheosisError::MaxChainsExceeded {
                count: self.sources.len(),
                max: MAX_SOURCE_CHAINS,
            });
        }

        // Validate assets per chain
        for source in &self.sources {
            if source.assets.len() > MAX_ASSETS_PER_CHAIN {
                return Err(ApotheosisError::MaxAssetsExceeded {
                    chain_id: source.chain_id.0,
                    count: source.assets.len(),
                    max: MAX_ASSETS_PER_CHAIN,
                });
            }
        }

        // Generate or use custom routes
        let routes = if let Some(custom) = self.custom_routes {
            custom
        } else {
            self.calculate_routes(&destination)?
        };

        // Calculate totals
        let total_cost_usd: f64 = routes.iter().map(|r| r.total_cost).sum();
        let total_estimated_time: u64 = routes.iter().map(|r| r.estimated_time).max().unwrap_or(0);

        // Generate transaction ID
        let id = generate_transaction_id();

        Ok(ApotheosisTransaction {
            id,
            sources: self.sources,
            destination,
            routes,
            status: TransactionStatus::Building,
            chain_statuses: Vec::new(),
            total_cost_usd,
            total_estimated_time,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        })
    }

    /// Calculate optimal routes for all assets
    fn calculate_routes(&self, destination: &Destination) -> ApotheosisResult<Vec<MigrationRoute>> {
        let mut routes = Vec::new();

        for source in &self.sources {
            for asset in &source.assets {
                let route =
                    self.calculate_route_for_asset(asset, source.chain_id, destination.chain_id)?;
                routes.push(route);
            }
        }

        // Sort by priority
        routes.sort_by(|a, b| b.asset.priority.cmp(&a.asset.priority));

        Ok(routes)
    }

    /// Calculate route for a single asset
    fn calculate_route_for_asset(
        &self,
        asset: &MigrationAsset,
        from: ChainId,
        to: ChainId,
    ) -> ApotheosisResult<MigrationRoute> {
        let mut hops = Vec::new();

        // Determine routing based on asset type and chains
        if from == to {
            // Same chain - direct transfer
            hops.push(RouteHop {
                hop_type: HopType::Transfer,
                from_chain: from,
                to_chain: to,
                protocol: "direct".to_string(),
                gas_cost: 21000,
                estimated_time: 15,
            });
        } else if from.is_evm() && to.is_evm() {
            // EVM to EVM - use bridge
            hops.push(RouteHop {
                hop_type: HopType::Bridge,
                from_chain: from,
                to_chain: to,
                protocol: self.select_bridge(from, to),
                gas_cost: 150000,
                estimated_time: 900, // 15 minutes
            });
        } else if from.is_svm() && to.is_evm() {
            // Solana to EVM - Wormhole
            hops.push(RouteHop {
                hop_type: HopType::Bridge,
                from_chain: from,
                to_chain: to,
                protocol: "wormhole".to_string(),
                gas_cost: 5000,       // Lamports
                estimated_time: 1200, // 20 minutes
            });
        } else if from.is_evm() && to.is_svm() {
            // EVM to Solana - Wormhole
            hops.push(RouteHop {
                hop_type: HopType::Bridge,
                from_chain: from,
                to_chain: to,
                protocol: "wormhole".to_string(),
                gas_cost: 200000,
                estimated_time: 1200,
            });
        } else if to == ChainId::ATLAS {
            // Any chain to Atlas - use Atlas Bridge
            hops.push(RouteHop {
                hop_type: HopType::Bridge,
                from_chain: from,
                to_chain: to,
                protocol: "atlas-bridge".to_string(),
                gas_cost: 100000,
                estimated_time: 60, // 1 minute
            });
        } else {
            // Multi-hop through Atlas
            hops.push(RouteHop {
                hop_type: HopType::Bridge,
                from_chain: from,
                to_chain: ChainId::ATLAS,
                protocol: "atlas-bridge".to_string(),
                gas_cost: 100000,
                estimated_time: 60,
            });
            hops.push(RouteHop {
                hop_type: HopType::Bridge,
                from_chain: ChainId::ATLAS,
                to_chain: to,
                protocol: "atlas-bridge".to_string(),
                gas_cost: 50000, // Atlas gas
                estimated_time: 60,
            });
        }

        // Handle position exits
        if let AssetType::Position { .. } = &asset.asset_type {
            hops.insert(
                0,
                RouteHop {
                    hop_type: HopType::ExitPosition,
                    from_chain: from,
                    to_chain: from,
                    protocol: "auto".to_string(),
                    gas_cost: 300000,
                    estimated_time: 30,
                },
            );
        }

        let total_cost = hops.iter().map(|h| h.gas_cost as f64 * 0.00001).sum();
        let estimated_time = hops.iter().map(|h| h.estimated_time).sum();

        Ok(MigrationRoute {
            asset: asset.clone(),
            hops,
            total_cost,
            estimated_time,
            risk_score: self.calculate_risk_score(&asset.asset_type, from, to),
        })
    }

    /// Select best bridge for route
    fn select_bridge(&self, from: ChainId, to: ChainId) -> String {
        // Priority order based on security and speed
        let bridges = vec![
            (
                "across",
                vec![
                    (ChainId::ETHEREUM, ChainId::ARBITRUM),
                    (ChainId::ETHEREUM, ChainId::OPTIMISM),
                ],
            ),
            (
                "stargate",
                vec![
                    (ChainId::ETHEREUM, ChainId::BSC),
                    (ChainId::ETHEREUM, ChainId::POLYGON),
                ],
            ),
            (
                "hop",
                vec![
                    (ChainId::ETHEREUM, ChainId::POLYGON),
                    (ChainId::ARBITRUM, ChainId::OPTIMISM),
                ],
            ),
        ];

        for (bridge, routes) in &bridges {
            if routes
                .iter()
                .any(|(f, t)| (*f == from && *t == to) || (*f == to && *t == from))
            {
                return bridge.to_string();
            }
        }

        // Default to atlas-bridge for Atlas routes, or layerzero for others
        if from == ChainId::ATLAS || to == ChainId::ATLAS {
            "atlas-bridge".to_string()
        } else {
            "layerzero".to_string()
        }
    }

    /// Calculate risk score for a route
    fn calculate_risk_score(&self, asset_type: &AssetType, from: ChainId, to: ChainId) -> u8 {
        let mut risk = 10u8; // Base risk

        // Asset type risk
        risk += match asset_type {
            AssetType::Native => 5,
            AssetType::FungibleToken(_) => 10,
            AssetType::NonFungible { .. } => 20,
            AssetType::SemiFungible { .. } => 15,
            AssetType::Position { .. } => 30,
            AssetType::Wrapped { .. } => 25,
        };

        // Cross-VM risk
        if from.is_evm() != to.is_evm() {
            risk += 15;
        }

        // Distance risk (more hops = more risk)
        if from != to && from != ChainId::ATLAS && to != ChainId::ATLAS {
            risk += 10;
        }

        risk.min(100)
    }
}

/// Generate a unique transaction ID
fn generate_transaction_id() -> String {
    let mut rng = rand::thread_rng();
    let random: u64 = rng.gen();
    let timestamp = Utc::now().timestamp_millis() as u64;
    format!("APO-{:016X}-{:08X}", timestamp, random as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_validation() {
        // No destination
        let result = ApotheosisBuilder::new()
            .sweep_chain(ChainId::ETHEREUM, "0x1234")
            .build();
        assert!(matches!(result, Err(ApotheosisError::NoDestination)));

        // No sources
        let result = ApotheosisBuilder::new()
            .to(ChainId::ATLAS, "5Atlas...")
            .build();
        assert!(matches!(result, Err(ApotheosisError::NoSourceChains)));
    }

    #[test]
    fn test_successful_build() {
        let result = ApotheosisBuilder::new()
            .sweep_chain(ChainId::ETHEREUM, "0x1234")
            .to(ChainId::ATLAS, "5Atlas...")
            .build();

        assert!(result.is_ok());
        let tx = result.unwrap();
        assert_eq!(tx.status, TransactionStatus::Building);
        assert!(tx.id.starts_with("APO-"));
    }
}
