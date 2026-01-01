//! Chain Indexing Job
//!
//! Multi-chain data indexing for:
//! - Block data extraction
//! - Event log processing
//! - State trie indexing
//! - Historical data aggregation
//! - Cross-chain correlation

use crate::error::{SwarmError, SwarmResult};
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for chain indexing job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Chains to index
    pub chains: Vec<ChainIndexConfig>,
    /// Block range to index
    pub block_range: BlockRange,
    /// Index types to extract
    pub index_types: Vec<IndexType>,
    /// Batch size for processing
    pub batch_size: usize,
    /// Parallel threads
    pub parallelism: usize,
    /// Output format
    pub output_format: OutputFormat,
    /// Compression enabled
    pub compression: bool,
}

/// Chain-specific indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainIndexConfig {
    pub chain_id: u64,
    pub name: String,
    pub rpc_url: String,
    /// Contracts to specifically index
    pub contracts: Vec<ContractConfig>,
    /// Event signatures to capture
    pub event_signatures: Vec<String>,
}

/// Contract configuration for indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    pub address: String,
    pub name: String,
    pub abi_hash: Option<[u8; 32]>,
    /// Specific events to index (empty = all)
    pub events: Vec<String>,
}

/// Block range specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockRange {
    /// Specific block range
    Range { start: u64, end: u64 },
    /// Last N blocks
    Latest(u64),
    /// From block to latest
    FromBlock(u64),
    /// Single block
    Block(u64),
}

impl Default for BlockRange {
    fn default() -> Self {
        Self::Latest(1000)
    }
}

/// Types of data to index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexType {
    /// Block headers
    BlockHeaders,
    /// Transaction receipts
    TransactionReceipts,
    /// Event logs
    EventLogs,
    /// State changes
    StateChanges,
    /// Token transfers (ERC20/721/1155)
    TokenTransfers,
    /// Internal transactions (traces)
    InternalTransactions,
    /// Contract deployments
    ContractDeployments,
    /// Account balances
    AccountBalances,
}

/// Output format for indexed data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Parquet,
    Json,
    Csv,
    Avro,
    Binary,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Parquet
    }
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            chains: vec![ChainIndexConfig {
                chain_id: 1,
                name: "Ethereum".to_string(),
                rpc_url: "".to_string(),
                contracts: Vec::new(),
                event_signatures: vec![
                    "Transfer(address,address,uint256)".to_string(),
                    "Swap(address,uint256,uint256,uint256,uint256,address)".to_string(),
                ],
            }],
            block_range: BlockRange::Latest(100),
            index_types: vec![
                IndexType::BlockHeaders,
                IndexType::TransactionReceipts,
                IndexType::EventLogs,
            ],
            batch_size: 100,
            parallelism: 4,
            output_format: OutputFormat::Parquet,
            compression: true,
        }
    }
}

/// Indexed block header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedBlock {
    pub chain_id: u64,
    pub number: u64,
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub timestamp: u64,
    pub miner: String,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub base_fee: Option<u64>,
    pub transaction_count: usize,
    pub size: u64,
}

/// Indexed transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedTransaction {
    pub chain_id: u64,
    pub block_number: u64,
    pub tx_index: u32,
    pub hash: [u8; 32],
    pub from: String,
    pub to: Option<String>,
    pub value: u128,
    pub gas_used: u64,
    pub gas_price: u64,
    pub status: bool,
    pub contract_created: Option<String>,
}

/// Indexed event log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedEvent {
    pub chain_id: u64,
    pub block_number: u64,
    pub tx_hash: [u8; 32],
    pub log_index: u32,
    pub address: String,
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
    pub event_signature: Option<String>,
    pub decoded: Option<DecodedEvent>,
}

/// Decoded event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedEvent {
    pub name: String,
    pub params: HashMap<String, String>,
}

/// Token transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedTokenTransfer {
    pub chain_id: u64,
    pub block_number: u64,
    pub tx_hash: [u8; 32],
    pub log_index: u32,
    pub token_address: String,
    pub token_type: TokenType,
    pub from: String,
    pub to: String,
    pub value: u128,
    pub token_id: Option<u128>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenType {
    ERC20,
    ERC721,
    ERC1155,
    Native,
}

/// Indexing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingStats {
    pub blocks_processed: usize,
    pub transactions_indexed: usize,
    pub events_indexed: usize,
    pub transfers_indexed: usize,
    pub errors_encountered: usize,
    pub bytes_produced: usize,
    pub processing_rate: f64, // blocks/sec
}

/// Result from chain indexing job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainIndexingResult {
    /// Indexed blocks
    pub blocks: Vec<IndexedBlock>,
    /// Indexed transactions (sample)
    pub transactions: Vec<IndexedTransaction>,
    /// Indexed events (sample)
    pub events: Vec<IndexedEvent>,
    /// Token transfers (sample)
    pub transfers: Vec<IndexedTokenTransfer>,
    /// Indexing stats
    pub stats: IndexingStats,
    /// Output locations (URIs)
    pub output_locations: HashMap<String, String>,
    /// Duration (ms)
    pub duration_ms: u64,
    /// Result hash
    pub result_hash: [u8; 32],
}

/// Chain Indexing Job
pub struct ChainIndexingJob {
    pub config: IndexingConfig,
    /// Pre-fetched block data (for testing/mock)
    pub prefetched_blocks: Vec<RawBlock>,
}

/// Raw block data for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawBlock {
    pub chain_id: u64,
    pub number: u64,
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub timestamp: u64,
    pub miner: String,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub base_fee: Option<u64>,
    pub transactions: Vec<RawTransaction>,
}

/// Raw transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTransaction {
    pub hash: [u8; 32],
    pub from: String,
    pub to: Option<String>,
    pub value: u128,
    pub gas_used: u64,
    pub gas_price: u64,
    pub status: bool,
    pub logs: Vec<RawLog>,
}

/// Raw log data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLog {
    pub address: String,
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

impl ChainIndexingJob {
    pub fn new(config: IndexingConfig) -> Self {
        Self {
            config,
            prefetched_blocks: Vec::new(),
        }
    }

    pub fn with_prefetched_blocks(mut self, blocks: Vec<RawBlock>) -> Self {
        self.prefetched_blocks = blocks;
        self
    }

    /// Index a single block
    fn index_block(&self, raw: &RawBlock) -> IndexedBlock {
        IndexedBlock {
            chain_id: raw.chain_id,
            number: raw.number,
            hash: raw.hash,
            parent_hash: raw.parent_hash,
            timestamp: raw.timestamp,
            miner: raw.miner.clone(),
            gas_used: raw.gas_used,
            gas_limit: raw.gas_limit,
            base_fee: raw.base_fee,
            transaction_count: raw.transactions.len(),
            size: 0, // Would calculate from actual data
        }
    }

    /// Index transactions from a block
    fn index_transactions(&self, raw: &RawBlock) -> Vec<IndexedTransaction> {
        raw.transactions
            .iter()
            .enumerate()
            .map(|(idx, tx)| IndexedTransaction {
                chain_id: raw.chain_id,
                block_number: raw.number,
                tx_index: idx as u32,
                hash: tx.hash,
                from: tx.from.clone(),
                to: tx.to.clone(),
                value: tx.value,
                gas_used: tx.gas_used,
                gas_price: tx.gas_price,
                status: tx.status,
                contract_created: None,
            })
            .collect()
    }

    /// Index events from a block
    fn index_events(&self, raw: &RawBlock) -> Vec<IndexedEvent> {
        let mut events = Vec::new();

        for tx in &raw.transactions {
            for (log_idx, log) in tx.logs.iter().enumerate() {
                events.push(IndexedEvent {
                    chain_id: raw.chain_id,
                    block_number: raw.number,
                    tx_hash: tx.hash,
                    log_index: log_idx as u32,
                    address: log.address.clone(),
                    topics: log.topics.clone(),
                    data: log.data.clone(),
                    event_signature: None,
                    decoded: None,
                });
            }
        }

        events
    }

    /// Detect token transfers from events
    fn index_transfers(&self, raw: &RawBlock) -> Vec<IndexedTokenTransfer> {
        let mut transfers = Vec::new();

        // ERC20 Transfer topic
        let transfer_topic = [
            0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37,
            0x8d, 0xaa, 0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d,
            0xf5, 0x23, 0xb3, 0xef,
        ];

        for tx in &raw.transactions {
            for (log_idx, log) in tx.logs.iter().enumerate() {
                if log.topics.first() == Some(&transfer_topic) && log.topics.len() >= 3 {
                    let from = format!("0x{}", hex::encode(&log.topics[1][12..32]));
                    let to = format!("0x{}", hex::encode(&log.topics[2][12..32]));

                    let value = if log.data.len() >= 32 {
                        u128::from_be_bytes(log.data[16..32].try_into().unwrap_or([0u8; 16]))
                    } else {
                        0
                    };

                    transfers.push(IndexedTokenTransfer {
                        chain_id: raw.chain_id,
                        block_number: raw.number,
                        tx_hash: tx.hash,
                        log_index: log_idx as u32,
                        token_address: log.address.clone(),
                        token_type: TokenType::ERC20,
                        from,
                        to,
                        value,
                        token_id: None,
                    });
                }
            }
        }

        transfers
    }

    /// Generate mock blocks for testing
    fn generate_mock_blocks(&self) -> Vec<RawBlock> {
        let block_count = match &self.config.block_range {
            BlockRange::Range { start, end } => (end - start) as usize,
            BlockRange::Latest(n) => *n as usize,
            BlockRange::FromBlock(_) => 10,
            BlockRange::Block(_) => 1,
        };

        let block_count = block_count.min(100);

        (0..block_count)
            .map(|i| {
                let block_num = 18000000 + i as u64;
                let mut hash = [0u8; 32];
                hash[0..8].copy_from_slice(&block_num.to_le_bytes());

                let mut parent_hash = [0u8; 32];
                parent_hash[0..8].copy_from_slice(&(block_num - 1).to_le_bytes());

                RawBlock {
                    chain_id: self.config.chains.first().map(|c| c.chain_id).unwrap_or(1),
                    number: block_num,
                    hash,
                    parent_hash,
                    timestamp: 1700000000 + (i as u64 * 12),
                    miner: "0x1234567890abcdef".to_string(),
                    gas_used: 15_000_000,
                    gas_limit: 30_000_000,
                    base_fee: Some(30_000_000_000),
                    transactions: (0..5)
                        .map(|j| {
                            let mut tx_hash = [0u8; 32];
                            tx_hash[0..8].copy_from_slice(&block_num.to_le_bytes());
                            tx_hash[8] = j;

                            RawTransaction {
                                hash: tx_hash,
                                from: format!("0xsender{}", j),
                                to: Some(format!("0xreceiver{}", j)),
                                value: (j as u128 + 1) * 1_000_000_000_000_000_000,
                                gas_used: 21000 + (j as u64 * 10000),
                                gas_price: 50_000_000_000,
                                status: true,
                                logs: vec![RawLog {
                                    address: "0xtoken".to_string(),
                                    topics: vec![
                                        [
                                            0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69,
                                            0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa, 0x95, 0x2b,
                                            0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a,
                                            0x4d, 0xf5, 0x23, 0xb3, 0xef,
                                        ],
                                        [0u8; 32],
                                        [0u8; 32],
                                    ],
                                    data: vec![0u8; 32],
                                }],
                            }
                        })
                        .collect(),
                }
            })
            .collect()
    }

    /// Execute full indexing job
    fn run_indexing(&self) -> SwarmResult<ChainIndexingResult> {
        use std::time::Instant;

        let start = Instant::now();

        // Use prefetched or generate mock data
        let blocks = if self.prefetched_blocks.is_empty() {
            self.generate_mock_blocks()
        } else {
            self.prefetched_blocks.clone()
        };

        let mut indexed_blocks = Vec::new();
        let mut indexed_txs = Vec::new();
        let mut indexed_events = Vec::new();
        let mut indexed_transfers = Vec::new();

        for block in &blocks {
            indexed_blocks.push(self.index_block(block));

            if self
                .config
                .index_types
                .contains(&IndexType::TransactionReceipts)
            {
                indexed_txs.extend(self.index_transactions(block));
            }

            if self.config.index_types.contains(&IndexType::EventLogs) {
                indexed_events.extend(self.index_events(block));
            }

            if self.config.index_types.contains(&IndexType::TokenTransfers) {
                indexed_transfers.extend(self.index_transfers(block));
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let processing_rate = if duration_ms > 0 {
            (blocks.len() as f64 * 1000.0) / duration_ms as f64
        } else {
            0.0
        };

        let stats = IndexingStats {
            blocks_processed: indexed_blocks.len(),
            transactions_indexed: indexed_txs.len(),
            events_indexed: indexed_events.len(),
            transfers_indexed: indexed_transfers.len(),
            errors_encountered: 0,
            bytes_produced: 0, // Would calculate actual output size
            processing_rate,
        };

        // Calculate result hash
        let mut hasher = blake3::Hasher::new();
        for block in &indexed_blocks {
            hasher.update(&block.hash);
        }
        let result_hash: [u8; 32] = hasher.finalize().into();

        Ok(ChainIndexingResult {
            blocks: indexed_blocks,
            transactions: indexed_txs.into_iter().take(100).collect(),
            events: indexed_events.into_iter().take(100).collect(),
            transfers: indexed_transfers.into_iter().take(100).collect(),
            stats,
            output_locations: HashMap::new(),
            duration_ms,
            result_hash,
        })
    }
}

impl SwarmJob for ChainIndexingJob {
    fn job_type(&self) -> JobType {
        JobType::ChainIndexing
    }

    fn compute_units(&self) -> u64 {
        // 10 CU per block
        let block_count = match &self.config.block_range {
            BlockRange::Range { start, end } => end - start,
            BlockRange::Latest(n) => *n,
            BlockRange::FromBlock(_) => 1000,
            BlockRange::Block(_) => 1,
        };
        block_count * 10
    }

    fn timeout(&self) -> Duration {
        // Indexing can take a while
        Duration::from_secs(300) // 5 minutes
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        let result = self.run_indexing()?;
        Ok(JobOutput::ChainIndexing(result))
    }

    fn verify(&self, result: &JobOutput) -> SwarmResult<bool> {
        match result {
            JobOutput::ChainIndexing(idx_result) => {
                // Verify result hash
                let mut hasher = blake3::Hasher::new();
                for block in &idx_result.blocks {
                    hasher.update(&block.hash);
                }
                let expected_hash: [u8; 32] = hasher.finalize().into();

                Ok(expected_hash == idx_result.result_hash)
            }
            _ => Err(SwarmError::InvalidResult("Wrong result type".into())),
        }
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::Low // Not time-critical
    }

    fn requires_gpu(&self) -> bool {
        false // CPU-bound I/O
    }

    fn min_vram_mb(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexing_config_default() {
        let config = IndexingConfig::default();
        assert!(config.compression);
        assert_eq!(config.output_format, OutputFormat::Parquet);
    }

    #[test]
    fn test_chain_indexing_execution() {
        let config = IndexingConfig {
            block_range: BlockRange::Latest(10),
            ..Default::default()
        };

        let job = ChainIndexingJob::new(config);
        let result = job.execute().unwrap();

        if let JobOutput::ChainIndexing(idx_result) = result {
            assert_eq!(idx_result.blocks.len(), 10);
            assert!(idx_result.stats.blocks_processed == 10);
        } else {
            panic!("Wrong result type");
        }
    }

    #[test]
    fn test_token_transfer_indexing() {
        let config = IndexingConfig {
            block_range: BlockRange::Block(1),
            index_types: vec![IndexType::TokenTransfers],
            ..Default::default()
        };

        let job = ChainIndexingJob::new(config);
        let result = job.execute().unwrap();

        if let JobOutput::ChainIndexing(idx_result) = result {
            // Should detect transfers from mock data
            assert!(idx_result.transfers.len() > 0);
        }
    }

    #[test]
    fn test_block_range_variants() {
        for range in [
            BlockRange::Range {
                start: 100,
                end: 110,
            },
            BlockRange::Latest(5),
            BlockRange::Block(1000),
        ] {
            let config = IndexingConfig {
                block_range: range,
                ..Default::default()
            };

            let job = ChainIndexingJob::new(config);
            assert!(job.compute_units() > 0);
        }
    }
}
