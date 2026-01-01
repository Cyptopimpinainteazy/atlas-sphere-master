//! Route optimization for Apotheosis Transaction

use crate::{
    error::{ApotheosisError, ApotheosisResult},
    types::*,
};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Route optimizer using Dijkstra's algorithm with multi-factor cost
pub struct RouteOptimizer {
    /// Known bridges between chains
    bridges: Vec<BridgeInfo>,
    /// Chain latencies (average confirmation time in seconds)
    chain_latencies: HashMap<ChainId, u64>,
    /// Gas price multipliers
    gas_multipliers: HashMap<ChainId, f64>,
}

/// Information about a bridge
#[derive(Debug, Clone)]
pub struct BridgeInfo {
    /// Bridge name
    pub name: String,
    /// Source chain
    pub from: ChainId,
    /// Destination chain
    pub to: ChainId,
    /// Base fee (USD)
    pub base_fee: f64,
    /// Fee percentage
    pub fee_percent: f64,
    /// Average time (seconds)
    pub avg_time: u64,
    /// Reliability score (0-100)
    pub reliability: u8,
    /// Liquidity (USD)
    pub liquidity: f64,
    /// Supported asset types
    pub supported_assets: Vec<AssetTypeClass>,
}

/// Classification of asset types for bridge compatibility
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetTypeClass {
    Native,
    ERC20,
    ERC721,
    ERC1155,
    SPLToken,
    All,
}

/// Route node for pathfinding
#[derive(Debug, Clone)]
struct RouteNode {
    chain_id: ChainId,
    cost: u64,
    hops: Vec<RouteHop>,
}

impl PartialEq for RouteNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for RouteNode {}

impl Ord for RouteNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost) // Reverse for min-heap
    }
}

impl PartialOrd for RouteNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl RouteOptimizer {
    /// Create a new route optimizer with default configuration
    pub fn new() -> Self {
        let mut optimizer = Self {
            bridges: Vec::new(),
            chain_latencies: HashMap::new(),
            gas_multipliers: HashMap::new(),
        };

        optimizer.initialize_defaults();
        optimizer
    }

    /// Initialize with default bridges and settings
    fn initialize_defaults(&mut self) {
        // Default chain latencies (seconds)
        self.chain_latencies.insert(ChainId::ATLAS, 6);
        self.chain_latencies.insert(ChainId::ETHEREUM, 12);
        self.chain_latencies.insert(ChainId::BSC, 3);
        self.chain_latencies.insert(ChainId::POLYGON, 2);
        self.chain_latencies.insert(ChainId::ARBITRUM, 1);
        self.chain_latencies.insert(ChainId::OPTIMISM, 2);
        self.chain_latencies.insert(ChainId::AVALANCHE, 2);
        self.chain_latencies.insert(ChainId::BASE, 2);
        self.chain_latencies.insert(ChainId::SOLANA, 0); // 400ms

        // Gas multipliers (relative to ETH mainnet = 1.0)
        self.gas_multipliers.insert(ChainId::ATLAS, 0.001);
        self.gas_multipliers.insert(ChainId::ETHEREUM, 1.0);
        self.gas_multipliers.insert(ChainId::BSC, 0.05);
        self.gas_multipliers.insert(ChainId::POLYGON, 0.01);
        self.gas_multipliers.insert(ChainId::ARBITRUM, 0.05);
        self.gas_multipliers.insert(ChainId::OPTIMISM, 0.05);
        self.gas_multipliers.insert(ChainId::AVALANCHE, 0.1);
        self.gas_multipliers.insert(ChainId::BASE, 0.05);
        self.gas_multipliers.insert(ChainId::SOLANA, 0.001);

        // Default bridges
        self.add_default_bridges();
    }

    /// Add default bridge configurations
    fn add_default_bridges(&mut self) {
        // Atlas Bridge (to/from all chains)
        let atlas_chains = vec![
            ChainId::ETHEREUM,
            ChainId::BSC,
            ChainId::POLYGON,
            ChainId::ARBITRUM,
            ChainId::OPTIMISM,
            ChainId::AVALANCHE,
            ChainId::BASE,
            ChainId::SOLANA,
        ];

        for chain in &atlas_chains {
            self.bridges.push(BridgeInfo {
                name: "atlas-bridge".to_string(),
                from: *chain,
                to: ChainId::ATLAS,
                base_fee: 0.1,
                fee_percent: 0.01,
                avg_time: 60,
                reliability: 99,
                liquidity: 100_000_000.0,
                supported_assets: vec![AssetTypeClass::All],
            });
            self.bridges.push(BridgeInfo {
                name: "atlas-bridge".to_string(),
                from: ChainId::ATLAS,
                to: *chain,
                base_fee: 0.1,
                fee_percent: 0.01,
                avg_time: 60,
                reliability: 99,
                liquidity: 100_000_000.0,
                supported_assets: vec![AssetTypeClass::All],
            });
        }

        // Wormhole
        self.bridges.push(BridgeInfo {
            name: "wormhole".to_string(),
            from: ChainId::SOLANA,
            to: ChainId::ETHEREUM,
            base_fee: 5.0,
            fee_percent: 0.1,
            avg_time: 1200,
            reliability: 95,
            liquidity: 500_000_000.0,
            supported_assets: vec![AssetTypeClass::Native, AssetTypeClass::SPLToken],
        });
        self.bridges.push(BridgeInfo {
            name: "wormhole".to_string(),
            from: ChainId::ETHEREUM,
            to: ChainId::SOLANA,
            base_fee: 10.0,
            fee_percent: 0.1,
            avg_time: 1200,
            reliability: 95,
            liquidity: 500_000_000.0,
            supported_assets: vec![AssetTypeClass::Native, AssetTypeClass::ERC20],
        });

        // Across Protocol
        self.bridges.push(BridgeInfo {
            name: "across".to_string(),
            from: ChainId::ETHEREUM,
            to: ChainId::ARBITRUM,
            base_fee: 2.0,
            fee_percent: 0.05,
            avg_time: 120,
            reliability: 98,
            liquidity: 200_000_000.0,
            supported_assets: vec![AssetTypeClass::Native, AssetTypeClass::ERC20],
        });
        self.bridges.push(BridgeInfo {
            name: "across".to_string(),
            from: ChainId::ETHEREUM,
            to: ChainId::OPTIMISM,
            base_fee: 2.0,
            fee_percent: 0.05,
            avg_time: 120,
            reliability: 98,
            liquidity: 200_000_000.0,
            supported_assets: vec![AssetTypeClass::Native, AssetTypeClass::ERC20],
        });

        // Stargate
        self.bridges.push(BridgeInfo {
            name: "stargate".to_string(),
            from: ChainId::ETHEREUM,
            to: ChainId::BSC,
            base_fee: 3.0,
            fee_percent: 0.06,
            avg_time: 300,
            reliability: 97,
            liquidity: 300_000_000.0,
            supported_assets: vec![AssetTypeClass::Native, AssetTypeClass::ERC20],
        });
        self.bridges.push(BridgeInfo {
            name: "stargate".to_string(),
            from: ChainId::ETHEREUM,
            to: ChainId::POLYGON,
            base_fee: 3.0,
            fee_percent: 0.06,
            avg_time: 300,
            reliability: 97,
            liquidity: 300_000_000.0,
            supported_assets: vec![AssetTypeClass::Native, AssetTypeClass::ERC20],
        });
    }

    /// Add a custom bridge
    pub fn add_bridge(&mut self, bridge: BridgeInfo) {
        self.bridges.push(bridge);
    }

    /// Find optimal route from source to destination
    pub fn find_optimal_route(
        &self,
        from: ChainId,
        to: ChainId,
        asset: &MigrationAsset,
    ) -> ApotheosisResult<Vec<RouteHop>> {
        if from == to {
            return Ok(vec![RouteHop {
                hop_type: HopType::Transfer,
                from_chain: from,
                to_chain: to,
                protocol: "direct".to_string(),
                gas_cost: 21000,
                estimated_time: self.chain_latencies.get(&from).copied().unwrap_or(60),
            }]);
        }

        // Dijkstra's algorithm
        let mut heap = BinaryHeap::new();
        let mut best_costs: HashMap<ChainId, u64> = HashMap::new();

        heap.push(RouteNode {
            chain_id: from,
            cost: 0,
            hops: Vec::new(),
        });
        best_costs.insert(from, 0);

        while let Some(current) = heap.pop() {
            if current.chain_id == to {
                return Ok(current.hops);
            }

            if current.cost > *best_costs.get(&current.chain_id).unwrap_or(&u64::MAX) {
                continue;
            }

            // Explore bridges from current chain
            let asset_class = self.classify_asset(&asset.asset_type);

            for bridge in &self.bridges {
                if bridge.from != current.chain_id {
                    continue;
                }

                // Check asset compatibility
                if !bridge.supported_assets.contains(&asset_class)
                    && !bridge.supported_assets.contains(&AssetTypeClass::All)
                {
                    continue;
                }

                let bridge_cost = self.calculate_bridge_cost(bridge, asset);
                let new_cost = current.cost + bridge_cost;

                if new_cost < *best_costs.get(&bridge.to).unwrap_or(&u64::MAX) {
                    best_costs.insert(bridge.to, new_cost);

                    let mut new_hops = current.hops.clone();
                    new_hops.push(RouteHop {
                        hop_type: HopType::Bridge,
                        from_chain: bridge.from,
                        to_chain: bridge.to,
                        protocol: bridge.name.clone(),
                        gas_cost: (bridge.base_fee * 1000.0) as u128,
                        estimated_time: bridge.avg_time,
                    });

                    heap.push(RouteNode {
                        chain_id: bridge.to,
                        cost: new_cost,
                        hops: new_hops,
                    });
                }
            }
        }

        Err(ApotheosisError::RouteNotFound {
            from: from.0,
            to: to.0,
        })
    }

    /// Calculate cost of using a bridge (normalized score)
    fn calculate_bridge_cost(&self, bridge: &BridgeInfo, asset: &MigrationAsset) -> u64 {
        let value = asset.estimated_value_usd;

        // Fee cost (USD * 100 for precision)
        let fee_cost = ((bridge.base_fee + value * bridge.fee_percent / 100.0) * 100.0) as u64;

        // Time cost (seconds / 10)
        let time_cost = bridge.avg_time / 10;

        // Reliability penalty (lower reliability = higher cost)
        let reliability_penalty = (100 - bridge.reliability as u64) * 10;

        fee_cost + time_cost + reliability_penalty
    }

    /// Classify asset type for bridge compatibility
    fn classify_asset(&self, asset_type: &AssetType) -> AssetTypeClass {
        match asset_type {
            AssetType::Native => AssetTypeClass::Native,
            AssetType::FungibleToken(_) => AssetTypeClass::ERC20,
            AssetType::NonFungible { .. } => AssetTypeClass::ERC721,
            AssetType::SemiFungible { .. } => AssetTypeClass::ERC1155,
            AssetType::Position { .. } => AssetTypeClass::ERC20, // Positions usually resolve to tokens
            AssetType::Wrapped { asset, .. } => self.classify_asset(asset),
        }
    }

    /// Estimate total cost for a complete migration
    pub fn estimate_migration_cost(&self, assets: &[MigrationAsset], destination: ChainId) -> f64 {
        let mut total_cost = 0.0;

        for asset in assets {
            if let Ok(hops) = self.find_optimal_route(asset.source_chain, destination, asset) {
                for hop in hops {
                    // Estimate gas cost in USD
                    let gas_multiplier = self
                        .gas_multipliers
                        .get(&hop.from_chain)
                        .copied()
                        .unwrap_or(1.0);

                    total_cost += (hop.gas_cost as f64) * gas_multiplier / 1_000_000.0;
                }
            }
        }

        total_cost
    }

    /// Get all available bridges from a chain
    pub fn get_bridges_from(&self, chain: ChainId) -> Vec<&BridgeInfo> {
        self.bridges.iter().filter(|b| b.from == chain).collect()
    }

    /// Get all available bridges to a chain
    pub fn get_bridges_to(&self, chain: ChainId) -> Vec<&BridgeInfo> {
        self.bridges.iter().filter(|b| b.to == chain).collect()
    }
}

impl Default for RouteOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_creation() {
        let optimizer = RouteOptimizer::new();
        assert!(!optimizer.bridges.is_empty());
    }

    #[test]
    fn test_same_chain_route() {
        let optimizer = RouteOptimizer::new();
        let asset = MigrationAsset {
            source_chain: ChainId::ETHEREUM,
            asset_type: AssetType::Native,
            amount: Some(1_000_000_000_000_000_000),
            contract: None,
            estimated_value_usd: 3000.0,
            priority: 10,
        };

        let route = optimizer.find_optimal_route(ChainId::ETHEREUM, ChainId::ETHEREUM, &asset);
        assert!(route.is_ok());
        let hops = route.unwrap();
        assert_eq!(hops.len(), 1);
        assert_eq!(hops[0].hop_type, HopType::Transfer);
    }

    #[test]
    fn test_eth_to_atlas_route() {
        let optimizer = RouteOptimizer::new();
        let asset = MigrationAsset {
            source_chain: ChainId::ETHEREUM,
            asset_type: AssetType::Native,
            amount: Some(1_000_000_000_000_000_000),
            contract: None,
            estimated_value_usd: 3000.0,
            priority: 10,
        };

        let route = optimizer.find_optimal_route(ChainId::ETHEREUM, ChainId::ATLAS, &asset);
        assert!(route.is_ok());
        let hops = route.unwrap();
        assert!(!hops.is_empty());
        assert_eq!(hops[0].protocol, "atlas-bridge");
    }
}
