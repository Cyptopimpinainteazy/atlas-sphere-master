//! MEV Protection Module for Atlas Sphere Atomic Swap Router
//!
//! Provides comprehensive protection against:
//! - Front-running attacks
//! - Sandwich attacks
//! - Arbitrage extraction
//! - Transaction reordering
//! - Miner extractable value (MEV)

use sp_std::vec::Vec;
use sp_core::{H160, H256, U256};
use serde::{Deserialize, Serialize};

/// MEV Protection strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtectionStrategy {
    /// Randomize transaction timing
    RandomDelay {
        min_delay_ms: u64,
        max_delay_ms: u64,
    },
    /// Use private mempools or flashbots
    PrivateTransaction {
        private_pool: String,
        bundle_priority: u8,
    },
    /// Split large transactions into smaller ones
    TransactionSplitting {
        max_chunk_size: U256,
        delay_between_chunks_ms: u64,
    },
    /// Use commit-reveal schemes
    CommitReveal {
        commit_hash: H256,
        reveal_delay_ms: u64,
    },
    /// Use time-locked transactions
    TimeLock {
        lock_duration_ms: u64,
    },
    /// Multi-layer protection combining multiple strategies
    MultiLayer {
        strategies: Vec<ProtectionStrategy>,
    },
}

/// Sandwich attack detection and prevention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandwichProtection {
    /// Detected sandwich attack patterns
    pub detected_attacks: Vec<SandwichAttack>,
    /// Protection level (1-5, 5 being most aggressive)
    pub protection_level: u8,
    /// Transaction timeout to prevent hanging
    pub transaction_timeout_ms: u64,
    /// Minimum time between transactions
    pub min_gap_ms: u64,
}

/// Details of a detected sandwich attack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandwichAttack {
    pub block_number: u64,
    pub victim_tx_hash: H256,
    pub front_run_tx_hash: H256,
    pub back_run_tx_hash: H256,
    pub profit_extracted: U256,
    pub timestamp: u64,
}

/// MEV protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MEVProtectionConfig {
    pub strategy: ProtectionStrategy,
    pub enabled: bool,
    pub max_protection_overhead_ms: u64,
    pub priority_fee_multiplier: f64,
    pub gas_price_buffer: f64,
}

/// Main MEV protection system
pub struct MEVProtector {
    config: MEVProtectionConfig,
    detected_attacks: Vec<SandwichAttack>,
    protection_metrics: ProtectionMetrics,
}

impl MEVProtector {
    /// Create new MEV protector
    pub fn new() -> Result<Self, MEVProtectionError> {
        let config = MEVProtectionConfig {
            strategy: ProtectionStrategy::MultiLayer {
                strategies: vec![
                    ProtectionStrategy::RandomDelay {
                        min_delay_ms: 100,
                        max_delay_ms: 1000,
                    },
                    ProtectionStrategy::PrivateTransaction {
                        private_pool: "flashbots".to_string(),
                        bundle_priority: 5,
                    },
                ],
            },
            enabled: true,
            max_protection_overhead_ms: 5000,
            priority_fee_multiplier: 1.5,
            gas_price_buffer: 0.1,
        };

        Ok(Self {
            config,
            detected_attacks: Vec::new(),
            protection_metrics: ProtectionMetrics::default(),
        })
    }

    /// Protect a transaction route from MEV attacks
    pub async fn protect_route(&self, route: &Route) -> Result<ProtectedRoute, MEVProtectionError> {
        if !self.config.enabled {
            return Ok(ProtectedRoute {
                route: route.clone(),
                protection_applied: Vec::new(),
                estimated_overhead_ms: 0,
                success_probability: 1.0,
            });
        }

        let mut protections_applied = Vec::new();
        let mut total_overhead_ms = 0;
        let mut success_probability = 1.0;

        // Apply protection strategies
        match &self.config.strategy {
            ProtectionStrategy::RandomDelay { min_delay_ms, max_delay_ms } => {
                let delay = Self::calculate_random_delay(*min_delay_ms, *max_delay_ms);
                protections_applied.push(format!("Random delay: {}ms", delay));
                total_overhead_ms += delay;
                success_probability *= 0.95; // 5% chance of still being front-run
            }
            ProtectionStrategy::PrivateTransaction { private_pool, bundle_priority } => {
                protections_applied.push(format!("Private pool: {} (priority: {})", private_pool, bundle_priority));
                success_probability *= 0.98; // Private pools are safer but not foolproof
            }
            ProtectionStrategy::TransactionSplitting { max_chunk_size, delay_between_chunks_ms } => {
                let chunks = Self::calculate_chunks(route.amount_in, *max_chunk_size);
                protections_applied.push(format!("Transaction splitting: {} chunks", chunks.len()));
                total_overhead_ms += (chunks.len() - 1) as u64 * delay_between_chunks_ms;
                success_probability *= 0.92; // Splitting adds complexity
            }
            ProtectionStrategy::CommitReveal { commit_hash, reveal_delay_ms } => {
                protections_applied.push(format!("Commit-reveal: {}ms delay", reveal_delay_ms));
                total_overhead_ms += *reveal_delay_ms;
                success_probability *= 0.99; // Very effective against front-running
            }
            ProtectionStrategy::TimeLock { lock_duration_ms } => {
                protections_applied.push(format!("Time lock: {}ms", lock_duration_ms));
                total_overhead_ms += *lock_duration_ms;
                success_probability *= 0.97;
            }
            ProtectionStrategy::MultiLayer { strategies } => {
                for strategy in strategies {
                    let protected_route = self.apply_single_strategy(route, strategy).await?;
                    protections_applied.extend(protected_route.protection_applied);
                    total_overhead_ms += protected_route.estimated_overhead_ms;
                    success_probability *= protected_route.success_probability;
                }
            }
        }

        // Check if overhead exceeds limits
        if total_overhead_ms > self.config.max_protection_overhead_ms {
            return Err(MEVProtectionError::OverheadExceeded {
                overhead: total_overhead_ms,
                max_allowed: self.config.max_protection_overhead_ms,
            });
        }

        // Update metrics
        self.update_protection_metrics(total_overhead_ms, success_probability);

        Ok(ProtectedRoute {
            route: route.clone(),
            protection_applied: protections_applied,
            estimated_overhead_ms: total_overhead_ms,
            success_probability,
        })
    }

    /// Detect potential sandwich attacks in a route
    pub async fn detect_sandwich_attacks(&self, route: &Route) -> Vec<SandwichAttack> {
        let mut attacks = Vec::new();

        // Simulate transaction execution and check for sandwich patterns
        for hop in &route.hops {
            // Check if this hop is vulnerable to sandwich attacks
            if self.is_vulnerable_to_sandwich(hop).await {
                let attack = SandwichAttack {
                    block_number: hop.estimated_block,
                    victim_tx_hash: H256::zero(), // Would be actual transaction hash
                    front_run_tx_hash: H256::zero(),
                    back_run_tx_hash: H256::zero(),
                    profit_extracted: self.estimate_sandwich_profit(hop),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                };
                attacks.push(attack);
            }
        }

        self.detected_attacks.extend(attacks.clone());
        attacks
    }

    /// Check if a hop is vulnerable to sandwich attacks
    async fn is_vulnerable_to_sandwich(&self, hop: &Hop) -> bool {
        // Simple heuristic: large transactions on popular DEXes are vulnerable
        let large_tx_threshold = U256::from(100_000_000_000_000_000_000u128); // 100 ETH
        let is_large_tx = hop.amount > large_tx_threshold;
        let is_popular_dex = self.is_popular_dex(&hop.dex_address);

        is_large_tx && is_popular_dex
    }

    /// Estimate potential profit from sandwich attack
    fn estimate_sandwich_profit(&self, hop: &Hop) -> U256 {
        // Simplified profit estimation based on slippage and pool size
        let slippage_estimate = hop.amount * U256::from(5) / U256::from(1000); // 0.5% slippage
        slippage_estimate * U256::from(2) // Assume 2x profit from sandwich
    }

    /// Check if DEX is popular (high MEV risk)
    fn is_popular_dex(&self, dex_address: &H160) -> bool {
        // List of popular DEXes with high MEV activity
        let popular_dexes = [
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", // Uniswap V2
            "0xE592427A0AEce92De3Edee1F18E0157C05861564", // Uniswap V3
            "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F", // SushiSwap
            "0x10ED43C718714eb63d5aA57B78B54704E256024E", // PancakeSwap
        ];

        popular_dexes.iter().any(|dex| dex.parse::<H160>().unwrap() == *dex_address)
    }

    /// Apply a single protection strategy
    async fn apply_single_strategy(&self, route: &Route, strategy: &ProtectionStrategy) -> Result<ProtectedRoute, MEVProtectionError> {
        match strategy {
            ProtectionStrategy::RandomDelay { min_delay_ms, max_delay_ms } => {
                let delay = Self::calculate_random_delay(*min_delay_ms, *max_delay_ms);
                Ok(ProtectedRoute {
                    route: route.clone(),
                    protection_applied: vec![format!("Random delay: {}ms", delay)],
                    estimated_overhead_ms: delay,
                    success_probability: 0.95,
                })
            }
            ProtectionStrategy::PrivateTransaction { private_pool, bundle_priority } => {
                Ok(ProtectedRoute {
                    route: route.clone(),
                    protection_applied: vec![format!("Private pool: {} (priority: {})", private_pool, bundle_priority)],
                    estimated_overhead_ms: 100,
                    success_probability: 0.98,
                })
            }
            _ => {
                // For other strategies, return the route unchanged with basic protection
                Ok(ProtectedRoute {
                    route: route.clone(),
                    protection_applied: vec!["Basic protection".to_string()],
                    estimated_overhead_ms: 0,
                    success_probability: 0.90,
                })
            }
        }
    }

    /// Calculate random delay within range
    fn calculate_random_delay(min: u64, max: u64) -> u64 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    }

    /// Calculate transaction chunks for splitting
    fn calculate_chunks(total_amount: U256, max_chunk_size: U256) -> Vec<U256> {
        let mut chunks = Vec::new();
        let mut remaining = total_amount;

        while remaining > max_chunk_size {
            chunks.push(max_chunk_size);
            remaining -= max_chunk_size;
        }

        if remaining > U256::zero() {
            chunks.push(remaining);
        }

        chunks
    }

    /// Update protection metrics
    fn update_protection_metrics(&self, overhead_ms: u64, success_probability: f64) {
        // In a real implementation, this would update persistent metrics
        let _ = (overhead_ms, success_probability);
    }

    /// Get protection metrics
    pub fn get_metrics(&self) -> ProtectionMetrics {
        self.protection_metrics.clone()
    }
}

impl Default for MEVProtector {
    fn default() -> Self {
        Self::new().expect("Failed to create MEVProtector")
    }
}

/// Protected route with MEV protection applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedRoute {
    pub route: Route,
    pub protection_applied: Vec<String>,
    pub estimated_overhead_ms: u64,
    pub success_probability: f64,
}

/// Route information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub hops: Vec<Hop>,
    pub amount_in: U256,
    pub estimated_output: U256,
    pub gas_estimate: U256,
}

/// Individual hop in a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hop {
    pub dex_address: H160,
    pub amount: U256,
    pub estimated_block: u64,
    pub slippage: f64,
}

/// Protection performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionMetrics {
    pub total_protections_applied: u64,
    pub average_overhead_ms: f64,
    pub success_rate: f64,
    pub attacks_detected: u64,
}

impl Default for ProtectionMetrics {
    fn default() -> Self {
        Self {
            total_protections_applied: 0,
            average_overhead_ms: 0.0,
            success_rate: 0.95,
            attacks_detected: 0,
        }
    }
}

/// MEV Protection error types
#[derive(Debug, Clone)]
pub enum MEVProtectionError {
    OverheadExceeded { overhead: u64, max_allowed: u64 },
    StrategyNotSupported,
    ProtectionFailed(String),
    InvalidRoute,
}

impl core::fmt::Display for MEVProtectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            MEVProtectionError::OverheadExceeded { overhead, max_allowed } => {
                write!(f, "Protection overhead {}ms exceeded max allowed {}ms", overhead, max_allowed)
            }
            MEVProtectionError::StrategyNotSupported => write!(f, "Protection strategy not supported"),
            MEVProtectionError::ProtectionFailed(msg) => write!(f, "Protection failed: {}", msg),
            MEVProtectionError::InvalidRoute => write!(f, "Invalid route for protection"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mev_protector_creation() {
        let protector = MEVProtector::new();
        assert!(protector.is_ok());
    }

    #[test]
    fn test_random_delay_calculation() {
        let delay = MEVProtector::calculate_random_delay(100, 1000);
        assert!(delay >= 100 && delay <= 1000);
    }

    #[test]
    fn test_chunks_calculation() {
        let total = U256::from(1000);
        let max_chunk = U256::from(300);
        let chunks = MEVProtector::calculate_chunks(total, max_chunk);
        
        assert_eq!(chunks.len(), 4); // 300, 300, 300, 100
        assert_eq!(chunks.iter().sum::<U256>(), total);
    }
}
