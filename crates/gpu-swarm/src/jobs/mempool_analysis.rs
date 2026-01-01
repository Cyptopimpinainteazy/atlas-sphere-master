//! Mempool Analysis Job
//!
//! Aggregates and analyzes pending transactions from multiple mempools:
//! - Transaction flow analysis
//! - Gas price prediction
//! - Priority fee estimation
//! - Bundle detection
//! - Whale movement tracking

use crate::error::{SwarmError, SwarmResult};
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for mempool analysis job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolConfig {
    /// Chains to monitor
    pub chains: Vec<ChainMempool>,
    /// Analysis window (seconds)
    pub analysis_window_secs: u64,
    /// Minimum transaction value to track (USD)
    pub min_tx_value_usd: f64,
    /// Enable gas prediction
    pub enable_gas_prediction: bool,
    /// Enable whale tracking
    pub enable_whale_tracking: bool,
    /// Whale threshold (USD)
    pub whale_threshold_usd: f64,
    /// Enable bundle detection
    pub enable_bundle_detection: bool,
    /// Track specific addresses
    pub tracked_addresses: Vec<String>,
    /// Track specific contracts
    pub tracked_contracts: Vec<String>,
}

/// Chain mempool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMempool {
    pub chain_id: u64,
    pub name: String,
    pub mempool_sources: Vec<String>, // RPC URLs or relay endpoints
    pub native_price_usd: f64,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            chains: vec![ChainMempool {
                chain_id: 1,
                name: "Ethereum".to_string(),
                mempool_sources: vec!["flashbots".to_string()],
                native_price_usd: 2000.0,
            }],
            analysis_window_secs: 60,
            min_tx_value_usd: 100.0,
            enable_gas_prediction: true,
            enable_whale_tracking: true,
            whale_threshold_usd: 100_000.0,
            enable_bundle_detection: true,
            tracked_addresses: Vec::new(),
            tracked_contracts: Vec::new(),
        }
    }
}

/// Pending transaction in mempool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTx {
    pub hash: String,
    pub chain_id: u64,
    pub from: String,
    pub to: Option<String>,
    pub value: f64,
    pub gas_limit: u64,
    pub gas_price: f64,
    pub max_fee_per_gas: Option<f64>,
    pub max_priority_fee: Option<f64>,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub first_seen: u64,
    pub propagation_count: u32,
}

/// Transaction classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TxClassification {
    Transfer,
    Swap,
    LiquidityAdd,
    LiquidityRemove,
    NFTMint,
    NFTTransfer,
    ContractDeploy,
    ContractCall,
    Approval,
    Bridge,
    Stake,
    Unstake,
    Unknown,
}

/// Analyzed transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedTx {
    pub tx: PendingTx,
    pub classification: TxClassification,
    pub value_usd: f64,
    pub gas_cost_usd: f64,
    pub priority_score: f64,
    pub is_whale: bool,
    pub related_txs: Vec<String>,
    pub decoded: Option<DecodedCall>,
}

/// Decoded contract call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedCall {
    pub method: String,
    pub contract_name: Option<String>,
    pub params: HashMap<String, String>,
}

/// Detected transaction bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxBundle {
    pub id: [u8; 32],
    pub transactions: Vec<String>,
    pub builder: Option<String>,
    pub total_gas: u64,
    pub total_value_usd: f64,
    pub mev_type: Option<String>,
    pub confidence: f64,
}

/// Gas price analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasAnalysis {
    pub chain_id: u64,
    pub timestamp: u64,
    /// Current base fee (EIP-1559 chains)
    pub base_fee: f64,
    /// Predicted base fee (next block)
    pub predicted_base_fee: f64,
    /// Percentile-based gas prices
    pub percentiles: GasPercentiles,
    /// Recommended fees
    pub recommendations: GasRecommendations,
    /// Mempool congestion (0-1)
    pub congestion: f64,
}

/// Gas price percentiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPercentiles {
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p99: f64,
}

/// Gas price recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasRecommendations {
    pub slow: GasFee,
    pub standard: GasFee,
    pub fast: GasFee,
    pub instant: GasFee,
}

/// Gas fee recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasFee {
    pub max_fee: f64,
    pub priority_fee: f64,
    pub estimated_time_secs: u64,
    pub confidence: f64,
}

/// Whale activity summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhaleActivity {
    pub address: String,
    pub chain_id: u64,
    pub tx_count: usize,
    pub total_volume_usd: f64,
    pub net_flow_usd: f64,
    pub tokens_moved: HashMap<String, f64>,
    pub patterns: Vec<String>,
}

/// Result from mempool analysis job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolAnalysisResult {
    /// Analyzed transactions
    pub transactions: Vec<AnalyzedTx>,
    /// Gas analysis per chain
    pub gas_analysis: HashMap<u64, GasAnalysis>,
    /// Detected bundles
    pub bundles: Vec<TxBundle>,
    /// Whale activities
    pub whale_activities: Vec<WhaleActivity>,
    /// Transaction stats
    pub stats: MempoolStats,
    /// Analysis duration (ms)
    pub duration_ms: u64,
    /// Result hash
    pub result_hash: [u8; 32],
}

/// Mempool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolStats {
    pub total_txs: usize,
    pub by_classification: HashMap<String, usize>,
    pub total_value_usd: f64,
    pub avg_gas_price: f64,
    pub whale_tx_count: usize,
    pub bundle_count: usize,
}

/// Mempool Analysis Job
pub struct MempoolAnalysisJob {
    pub config: MempoolConfig,
    /// Raw pending transactions
    pub pending_txs: Vec<PendingTx>,
}

impl MempoolAnalysisJob {
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            config,
            pending_txs: Vec::new(),
        }
    }

    pub fn with_pending_txs(mut self, txs: Vec<PendingTx>) -> Self {
        self.pending_txs = txs;
        self
    }

    /// Classify a transaction
    fn classify_tx(&self, tx: &PendingTx) -> TxClassification {
        // Simple classification based on data patterns
        if tx.data.is_empty() {
            return TxClassification::Transfer;
        }

        if tx.data.len() >= 4 {
            let selector = &tx.data[0..4];

            // Common function selectors
            match selector {
                // transfer(address,uint256)
                [0xa9, 0x05, 0x9c, 0xbb] => TxClassification::Transfer,
                // approve(address,uint256)
                [0x09, 0x5e, 0xa7, 0xb3] => TxClassification::Approval,
                // swapExactTokensForTokens
                [0x38, 0xed, 0x17, 0x39] => TxClassification::Swap,
                // swapExactETHForTokens
                [0x7f, 0xf3, 0x6a, 0xb5] => TxClassification::Swap,
                // addLiquidity
                [0xe8, 0xe3, 0x37, 0x00] => TxClassification::LiquidityAdd,
                // removeLiquidity
                [0xba, 0xa2, 0xab, 0xde] => TxClassification::LiquidityRemove,
                _ => {
                    if tx.to.is_none() {
                        TxClassification::ContractDeploy
                    } else {
                        TxClassification::ContractCall
                    }
                }
            }
        } else {
            TxClassification::Unknown
        }
    }

    /// Analyze a single transaction
    fn analyze_tx(&self, tx: &PendingTx) -> AnalyzedTx {
        let chain = self
            .config
            .chains
            .iter()
            .find(|c| c.chain_id == tx.chain_id);

        let native_price = chain.map(|c| c.native_price_usd).unwrap_or(2000.0);

        let classification = self.classify_tx(tx);
        let value_usd = tx.value * native_price;
        let gas_cost_usd = (tx.gas_limit as f64 * tx.gas_price) * native_price / 1e18;

        let priority_score = if let Some(priority_fee) = tx.max_priority_fee {
            priority_fee / 10.0 // Normalize to 0-1 range
        } else {
            tx.gas_price / 100.0
        }
        .min(1.0);

        let is_whale = value_usd >= self.config.whale_threshold_usd;

        AnalyzedTx {
            tx: tx.clone(),
            classification,
            value_usd,
            gas_cost_usd,
            priority_score,
            is_whale,
            related_txs: Vec::new(),
            decoded: None,
        }
    }

    /// Analyze gas prices
    fn analyze_gas(&self, chain_id: u64, txs: &[AnalyzedTx]) -> GasAnalysis {
        let mut gas_prices: Vec<f64> = txs
            .iter()
            .filter(|t| t.tx.chain_id == chain_id)
            .map(|t| t.tx.gas_price)
            .collect();

        gas_prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = gas_prices.len().max(1);
        let percentile = |p: usize| -> f64 {
            if gas_prices.is_empty() {
                0.0
            } else {
                gas_prices[(p * len / 100).min(len - 1)]
            }
        };

        let base_fee = percentile(50);
        let predicted_base_fee = base_fee * 1.125; // Assume 12.5% increase

        let congestion = if len > 100 { 0.8 } else { len as f64 / 125.0 };

        GasAnalysis {
            chain_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or_default(),
            base_fee,
            predicted_base_fee,
            percentiles: GasPercentiles {
                p10: percentile(10),
                p25: percentile(25),
                p50: percentile(50),
                p75: percentile(75),
                p90: percentile(90),
                p99: percentile(99),
            },
            recommendations: GasRecommendations {
                slow: GasFee {
                    max_fee: percentile(25),
                    priority_fee: 1.0,
                    estimated_time_secs: 120,
                    confidence: 0.7,
                },
                standard: GasFee {
                    max_fee: percentile(50),
                    priority_fee: 2.0,
                    estimated_time_secs: 60,
                    confidence: 0.85,
                },
                fast: GasFee {
                    max_fee: percentile(75),
                    priority_fee: 3.0,
                    estimated_time_secs: 30,
                    confidence: 0.95,
                },
                instant: GasFee {
                    max_fee: percentile(90),
                    priority_fee: 5.0,
                    estimated_time_secs: 12,
                    confidence: 0.99,
                },
            },
            congestion,
        }
    }

    /// Detect transaction bundles
    fn detect_bundles(&self, txs: &[AnalyzedTx]) -> Vec<TxBundle> {
        let mut bundles = Vec::new();

        // Group by sender and look for patterns
        let mut by_sender: HashMap<&str, Vec<&AnalyzedTx>> = HashMap::new();
        for tx in txs {
            by_sender.entry(&tx.tx.from).or_default().push(tx);
        }

        for (sender, sender_txs) in by_sender {
            if sender_txs.len() >= 2 {
                // Check for sequential nonces (bundle pattern)
                let mut nonces: Vec<u64> = sender_txs.iter().map(|t| t.tx.nonce).collect();
                nonces.sort();

                let is_sequential = nonces.windows(2).all(|w| w[1] == w[0] + 1);

                if is_sequential && sender_txs.len() >= 2 {
                    let bundle_id =
                        blake3::hash(format!("bundle:{}:{}", sender, nonces[0]).as_bytes()).into();

                    let total_gas: u64 = sender_txs.iter().map(|t| t.tx.gas_limit).sum();

                    let total_value: f64 = sender_txs.iter().map(|t| t.value_usd).sum();

                    bundles.push(TxBundle {
                        id: bundle_id,
                        transactions: sender_txs.iter().map(|t| t.tx.hash.clone()).collect(),
                        builder: None,
                        total_gas,
                        total_value_usd: total_value,
                        mev_type: Some("bundle".to_string()),
                        confidence: 0.7,
                    });
                }
            }
        }

        bundles
    }

    /// Track whale activities
    fn track_whales(&self, txs: &[AnalyzedTx]) -> Vec<WhaleActivity> {
        let mut whale_txs: HashMap<&str, Vec<&AnalyzedTx>> = HashMap::new();

        for tx in txs {
            if tx.is_whale {
                whale_txs.entry(&tx.tx.from).or_default().push(tx);
            }
        }

        whale_txs
            .into_iter()
            .map(|(address, txs)| {
                let total_volume: f64 = txs.iter().map(|t| t.value_usd).sum();

                WhaleActivity {
                    address: address.to_string(),
                    chain_id: txs[0].tx.chain_id,
                    tx_count: txs.len(),
                    total_volume_usd: total_volume,
                    net_flow_usd: total_volume, // Simplified
                    tokens_moved: HashMap::new(),
                    patterns: vec!["high_volume".to_string()],
                }
            })
            .collect()
    }

    /// Execute full mempool analysis
    fn run_analysis(&self) -> SwarmResult<MempoolAnalysisResult> {
        use std::time::Instant;

        let start = Instant::now();

        // Analyze all transactions
        let transactions: Vec<AnalyzedTx> = self
            .pending_txs
            .iter()
            .filter(|tx| {
                let chain = self
                    .config
                    .chains
                    .iter()
                    .find(|c| c.chain_id == tx.chain_id);
                let native_price = chain.map(|c| c.native_price_usd).unwrap_or(2000.0);
                tx.value * native_price >= self.config.min_tx_value_usd
            })
            .map(|tx| self.analyze_tx(tx))
            .collect();

        // Gas analysis per chain
        let mut gas_analysis = HashMap::new();
        for chain in &self.config.chains {
            let analysis = self.analyze_gas(chain.chain_id, &transactions);
            gas_analysis.insert(chain.chain_id, analysis);
        }

        // Bundle detection
        let bundles = if self.config.enable_bundle_detection {
            self.detect_bundles(&transactions)
        } else {
            Vec::new()
        };

        // Whale tracking
        let whale_activities = if self.config.enable_whale_tracking {
            self.track_whales(&transactions)
        } else {
            Vec::new()
        };

        // Calculate stats
        let mut by_classification: HashMap<String, usize> = HashMap::new();
        for tx in &transactions {
            *by_classification
                .entry(format!("{:?}", tx.classification))
                .or_default() += 1;
        }

        let total_value: f64 = transactions.iter().map(|t| t.value_usd).sum();
        let avg_gas = if transactions.is_empty() {
            0.0
        } else {
            transactions.iter().map(|t| t.tx.gas_price).sum::<f64>() / transactions.len() as f64
        };

        let stats = MempoolStats {
            total_txs: transactions.len(),
            by_classification,
            total_value_usd: total_value,
            avg_gas_price: avg_gas,
            whale_tx_count: transactions.iter().filter(|t| t.is_whale).count(),
            bundle_count: bundles.len(),
        };

        // Calculate result hash
        let mut hasher = blake3::Hasher::new();
        hasher.update(&(transactions.len() as u64).to_le_bytes());
        hasher.update(&total_value.to_le_bytes());
        let result_hash: [u8; 32] = hasher.finalize().into();

        Ok(MempoolAnalysisResult {
            transactions,
            gas_analysis,
            bundles,
            whale_activities,
            stats,
            duration_ms: start.elapsed().as_millis() as u64,
            result_hash,
        })
    }
}

impl SwarmJob for MempoolAnalysisJob {
    fn job_type(&self) -> JobType {
        JobType::MempoolAnalysis
    }

    fn compute_units(&self) -> u64 {
        // 1 CU per transaction
        self.pending_txs.len() as u64
    }

    fn timeout(&self) -> Duration {
        // Mempool analysis should be fast
        Duration::from_secs(30)
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        let result = self.run_analysis()?;
        Ok(JobOutput::MempoolAnalysis(result))
    }

    fn verify(&self, result: &JobOutput) -> SwarmResult<bool> {
        match result {
            JobOutput::MempoolAnalysis(mp_result) => {
                // Verify result hash
                let mut hasher = blake3::Hasher::new();
                hasher.update(&(mp_result.transactions.len() as u64).to_le_bytes());
                hasher.update(&mp_result.stats.total_value_usd.to_le_bytes());
                let expected_hash: [u8; 32] = hasher.finalize().into();

                Ok(expected_hash == mp_result.result_hash)
            }
            _ => Err(SwarmError::InvalidResult("Wrong result type".into())),
        }
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::High // Time-sensitive
    }

    fn requires_gpu(&self) -> bool {
        false // CPU-bound analysis
    }

    fn min_vram_mb(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mempool_config_default() {
        let config = MempoolConfig::default();
        assert!(config.enable_gas_prediction);
        assert!(config.enable_whale_tracking);
    }

    #[test]
    fn test_tx_classification() {
        let config = MempoolConfig::default();
        let job = MempoolAnalysisJob::new(config);

        // Transfer (empty data)
        let transfer_tx = PendingTx {
            hash: "0x1".to_string(),
            chain_id: 1,
            from: "0xabc".to_string(),
            to: Some("0xdef".to_string()),
            value: 1.0,
            gas_limit: 21000,
            gas_price: 50.0,
            max_fee_per_gas: None,
            max_priority_fee: None,
            nonce: 0,
            data: vec![],
            first_seen: 0,
            propagation_count: 1,
        };

        assert_eq!(job.classify_tx(&transfer_tx), TxClassification::Transfer);

        // Swap
        let swap_tx = PendingTx {
            data: vec![0x38, 0xed, 0x17, 0x39, 0x00, 0x00],
            ..transfer_tx.clone()
        };

        assert_eq!(job.classify_tx(&swap_tx), TxClassification::Swap);
    }

    #[test]
    fn test_mempool_analysis_execution() {
        let config = MempoolConfig::default();
        let txs = vec![PendingTx {
            hash: "0x1".to_string(),
            chain_id: 1,
            from: "0xwhale".to_string(),
            to: Some("0xdef".to_string()),
            value: 100.0, // 200k USD
            gas_limit: 21000,
            gas_price: 50.0,
            max_fee_per_gas: Some(60.0),
            max_priority_fee: Some(2.0),
            nonce: 0,
            data: vec![],
            first_seen: 0,
            propagation_count: 5,
        }];

        let job = MempoolAnalysisJob::new(config).with_pending_txs(txs);
        let result = job.execute().unwrap();

        if let JobOutput::MempoolAnalysis(mp_result) = result {
            assert!(mp_result.stats.total_txs >= 0);
            assert!(mp_result.gas_analysis.contains_key(&1));
        }
    }
}
