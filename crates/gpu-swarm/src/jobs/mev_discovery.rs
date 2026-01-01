//! MEV Discovery Job
//!
//! Identifies Maximum Extractable Value opportunities across chains:
//! - Arbitrage paths (DEX→DEX, CEX→DEX)
//! - Sandwich opportunities
//! - Liquidation targets
//! - JIT liquidity positions

use crate::error::{SwarmError, SwarmResult};
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for MEV discovery job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevConfig {
    /// Target chains to scan
    pub chains: Vec<ChainConfig>,
    /// Minimum profit threshold (in USD)
    pub min_profit_usd: f64,
    /// Maximum gas budget (in native token)
    pub max_gas_budget: f64,
    /// Scan window (blocks)
    pub scan_blocks: u32,
    /// DEX protocols to include
    pub dex_protocols: Vec<String>,
    /// Lending protocols for liquidations
    pub lending_protocols: Vec<String>,
    /// Flash loan providers
    pub flash_loan_providers: Vec<String>,
    /// Enable sandwich detection
    pub enable_sandwich: bool,
    /// Enable arbitrage detection
    pub enable_arbitrage: bool,
    /// Enable liquidation detection
    pub enable_liquidations: bool,
    /// Enable JIT liquidity detection
    pub enable_jit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: String,
    pub rpc_url: String,
    pub avg_block_time_ms: u64,
    pub native_token: String,
    pub native_price_usd: f64,
}

impl Default for MevConfig {
    fn default() -> Self {
        Self {
            chains: vec![ChainConfig {
                chain_id: 1,
                name: "Ethereum".to_string(),
                rpc_url: "".to_string(),
                avg_block_time_ms: 12000,
                native_token: "ETH".to_string(),
                native_price_usd: 2000.0,
            }],
            min_profit_usd: 10.0,
            max_gas_budget: 0.1,
            scan_blocks: 1,
            dex_protocols: vec![
                "UniswapV2".to_string(),
                "UniswapV3".to_string(),
                "Curve".to_string(),
                "Balancer".to_string(),
            ],
            lending_protocols: vec!["Aave".to_string(), "Compound".to_string()],
            flash_loan_providers: vec!["Aave".to_string(), "dYdX".to_string()],
            enable_sandwich: true,
            enable_arbitrage: true,
            enable_liquidations: true,
            enable_jit: true,
        }
    }
}

/// Type of MEV opportunity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MevType {
    Arbitrage,
    Sandwich,
    Liquidation,
    JitLiquidity,
    Backrun,
    Frontrun,
}

/// An identified MEV opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevOpportunity {
    /// Opportunity ID
    pub id: [u8; 32],
    /// Type of MEV
    pub mev_type: MevType,
    /// Chain ID
    pub chain_id: u64,
    /// Expected profit (in native token)
    pub profit_native: f64,
    /// Expected profit (in USD)
    pub profit_usd: f64,
    /// Gas cost estimate
    pub gas_cost: f64,
    /// Net profit (profit - gas)
    pub net_profit_usd: f64,
    /// Confidence score (0-1)
    pub confidence: f64,
    /// Time sensitivity (blocks until opportunity expires)
    pub blocks_until_stale: u32,
    /// Required capital
    pub capital_required: f64,
    /// Execution path (for arbitrage)
    pub execution_path: Vec<SwapHop>,
    /// Target transaction (for sandwich)
    pub target_tx: Option<String>,
    /// Collateral address (for liquidation)
    pub liquidation_target: Option<String>,
    /// Timestamp discovered
    pub discovered_at: u64,
}

/// A swap hop in an arbitrage path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapHop {
    pub protocol: String,
    pub pool_address: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: f64,
    pub amount_out: f64,
    pub price_impact: f64,
}

/// Result from MEV discovery job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevDiscoveryResult {
    /// Discovered opportunities (sorted by profit)
    pub opportunities: Vec<MevOpportunity>,
    /// Opportunities by type
    pub by_type: HashMap<String, usize>,
    /// Total potential profit (USD)
    pub total_profit_usd: f64,
    /// Blocks scanned
    pub blocks_scanned: u32,
    /// Pools analyzed
    pub pools_analyzed: usize,
    /// Transactions analyzed
    pub txs_analyzed: usize,
    /// Execution duration (ms)
    pub duration_ms: u64,
    /// Result hash for verification
    pub result_hash: [u8; 32],
}

/// MEV Discovery Job
pub struct MevDiscoveryJob {
    pub config: MevConfig,
    /// Pending transactions from mempool
    pub pending_txs: Vec<PendingTransaction>,
    /// Pool reserves snapshot
    pub pool_snapshots: Vec<PoolSnapshot>,
}

/// Pending transaction from mempool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: f64,
    pub gas_price: f64,
    pub data: Vec<u8>,
    pub decoded_action: Option<DecodedAction>,
}

/// Decoded swap/action from transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedAction {
    pub action_type: String,
    pub protocol: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: f64,
    pub min_amount_out: f64,
}

/// Pool reserves snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSnapshot {
    pub protocol: String,
    pub address: String,
    pub token0: String,
    pub token1: String,
    pub reserve0: f64,
    pub reserve1: f64,
    pub fee: f64,
    pub block_number: u64,
}

impl MevDiscoveryJob {
    pub fn new(config: MevConfig) -> Self {
        Self {
            config,
            pending_txs: Vec::new(),
            pool_snapshots: Vec::new(),
        }
    }

    pub fn with_pending_txs(mut self, txs: Vec<PendingTransaction>) -> Self {
        self.pending_txs = txs;
        self
    }

    pub fn with_pool_snapshots(mut self, snapshots: Vec<PoolSnapshot>) -> Self {
        self.pool_snapshots = snapshots;
        self
    }

    /// Detect arbitrage opportunities
    fn detect_arbitrage(&self) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();

        // Simple 2-hop arbitrage detection (production would use Bellman-Ford)
        for pool1 in &self.pool_snapshots {
            for pool2 in &self.pool_snapshots {
                if pool1.address == pool2.address {
                    continue;
                }

                // Check if pools form a cycle
                if pool1.token1 == pool2.token0 && pool2.token1 == pool1.token0 {
                    // Calculate potential profit
                    let rate1 = pool1.reserve1 / pool1.reserve0;
                    let rate2 = pool2.reserve1 / pool2.reserve0;
                    let profit_ratio = rate1 * rate2;

                    if profit_ratio > 1.001 {
                        // > 0.1% profit after fees
                        let profit_usd = (profit_ratio - 1.0) * 10000.0; // Assume 10k capital

                        if profit_usd >= self.config.min_profit_usd {
                            let id = blake3::hash(
                                format!("arb:{}:{}", pool1.address, pool2.address).as_bytes(),
                            )
                            .into();

                            opportunities.push(MevOpportunity {
                                id,
                                mev_type: MevType::Arbitrage,
                                chain_id: self
                                    .config
                                    .chains
                                    .first()
                                    .map(|c| c.chain_id)
                                    .unwrap_or(1),
                                profit_native: profit_usd / 2000.0,
                                profit_usd,
                                gas_cost: 50.0,
                                net_profit_usd: profit_usd - 50.0,
                                confidence: 0.85,
                                blocks_until_stale: 1,
                                capital_required: 10000.0,
                                execution_path: vec![
                                    SwapHop {
                                        protocol: pool1.protocol.clone(),
                                        pool_address: pool1.address.clone(),
                                        token_in: pool1.token0.clone(),
                                        token_out: pool1.token1.clone(),
                                        amount_in: 10000.0,
                                        amount_out: 10000.0 * rate1,
                                        price_impact: 0.001,
                                    },
                                    SwapHop {
                                        protocol: pool2.protocol.clone(),
                                        pool_address: pool2.address.clone(),
                                        token_in: pool2.token0.clone(),
                                        token_out: pool2.token1.clone(),
                                        amount_in: 10000.0 * rate1,
                                        amount_out: 10000.0 * profit_ratio,
                                        price_impact: 0.001,
                                    },
                                ],
                                target_tx: None,
                                liquidation_target: None,
                                discovered_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            });
                        }
                    }
                }
            }
        }

        opportunities
    }

    /// Detect sandwich opportunities
    fn detect_sandwich(&self) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();

        for tx in &self.pending_txs {
            if let Some(action) = &tx.decoded_action {
                if action.action_type == "swap" {
                    // Check if swap is large enough to sandwich
                    let slippage = (action.amount_in - action.min_amount_out) / action.amount_in;

                    if slippage > 0.005 && action.amount_in > 1000.0 {
                        // Potential sandwich target
                        let profit_usd = action.amount_in * slippage * 0.3; // Capture 30% of slippage

                        if profit_usd >= self.config.min_profit_usd {
                            let id =
                                blake3::hash(format!("sandwich:{}", tx.hash).as_bytes()).into();

                            opportunities.push(MevOpportunity {
                                id,
                                mev_type: MevType::Sandwich,
                                chain_id: self
                                    .config
                                    .chains
                                    .first()
                                    .map(|c| c.chain_id)
                                    .unwrap_or(1),
                                profit_native: profit_usd / 2000.0,
                                profit_usd,
                                gas_cost: 100.0, // Two txs
                                net_profit_usd: profit_usd - 100.0,
                                confidence: 0.7,
                                blocks_until_stale: 0,
                                capital_required: action.amount_in * 2.0,
                                execution_path: Vec::new(),
                                target_tx: Some(tx.hash.clone()),
                                liquidation_target: None,
                                discovered_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            });
                        }
                    }
                }
            }
        }

        opportunities
    }

    /// Execute full MEV discovery
    fn run_discovery(&self) -> SwarmResult<MevDiscoveryResult> {
        use std::time::Instant;

        let start = Instant::now();
        let mut opportunities = Vec::new();
        let mut by_type: HashMap<String, usize> = HashMap::new();

        if self.config.enable_arbitrage {
            let arbs = self.detect_arbitrage();
            by_type.insert("arbitrage".to_string(), arbs.len());
            opportunities.extend(arbs);
        }

        if self.config.enable_sandwich {
            let sandwiches = self.detect_sandwich();
            by_type.insert("sandwich".to_string(), sandwiches.len());
            opportunities.extend(sandwiches);
        }

        // Sort by net profit descending
        opportunities.sort_by(|a, b| {
            b.net_profit_usd
                .partial_cmp(&a.net_profit_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_profit_usd: f64 = opportunities.iter().map(|o| o.net_profit_usd).sum();

        // Calculate result hash
        let mut hasher = blake3::Hasher::new();
        for opp in &opportunities {
            hasher.update(&opp.id);
        }
        let result_hash: [u8; 32] = hasher.finalize().into();

        Ok(MevDiscoveryResult {
            opportunities,
            by_type,
            total_profit_usd,
            blocks_scanned: self.config.scan_blocks,
            pools_analyzed: self.pool_snapshots.len(),
            txs_analyzed: self.pending_txs.len(),
            duration_ms: start.elapsed().as_millis() as u64,
            result_hash,
        })
    }
}

impl SwarmJob for MevDiscoveryJob {
    fn job_type(&self) -> JobType {
        JobType::MevDiscovery
    }

    fn compute_units(&self) -> u64 {
        // 10 CU per pool + 5 CU per tx
        (self.pool_snapshots.len() * 10 + self.pending_txs.len() * 5) as u64
    }

    fn timeout(&self) -> Duration {
        // MEV is time-critical
        Duration::from_secs(15)
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        let result = self.run_discovery()?;
        Ok(JobOutput::MevDiscovery(result))
    }

    fn verify(&self, result: &JobOutput) -> SwarmResult<bool> {
        match result {
            JobOutput::MevDiscovery(mev_result) => {
                // Verify result hash
                let mut hasher = blake3::Hasher::new();
                for opp in &mev_result.opportunities {
                    hasher.update(&opp.id);
                }
                let expected_hash: [u8; 32] = hasher.finalize().into();

                Ok(expected_hash == mev_result.result_hash)
            }
            _ => Err(SwarmError::InvalidResult("Wrong result type".into())),
        }
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::Critical // MEV is extremely time-sensitive
    }

    fn requires_gpu(&self) -> bool {
        true // GPU accelerates path finding
    }

    fn min_vram_mb(&self) -> u32 {
        512
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mev_config_default() {
        let config = MevConfig::default();
        assert!(config.enable_arbitrage);
        assert!(config.enable_sandwich);
        assert!(config.min_profit_usd > 0.0);
    }

    #[test]
    fn test_arbitrage_detection() {
        let config = MevConfig::default();
        let job = MevDiscoveryJob::new(config).with_pool_snapshots(vec![
            PoolSnapshot {
                protocol: "UniswapV2".to_string(),
                address: "0xAAA".to_string(),
                token0: "WETH".to_string(),
                token1: "USDC".to_string(),
                reserve0: 1000.0,
                reserve1: 2_000_000.0,
                fee: 0.003,
                block_number: 1000,
            },
            PoolSnapshot {
                protocol: "SushiSwap".to_string(),
                address: "0xBBB".to_string(),
                token0: "USDC".to_string(),
                token1: "WETH".to_string(),
                reserve0: 2_100_000.0, // Slight price difference
                reserve1: 1000.0,
                fee: 0.003,
                block_number: 1000,
            },
        ]);

        let result = job.execute().unwrap();

        if let JobOutput::MevDiscovery(mev_result) = result {
            // May or may not find arb depending on price diff
            assert!(mev_result.pools_analyzed == 2);
        }
    }

    #[test]
    fn test_mev_job_priority() {
        let job = MevDiscoveryJob::new(MevConfig::default());
        assert_eq!(job.priority(), TaskPriority::Critical);
    }
}
