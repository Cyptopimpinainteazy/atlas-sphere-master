//! ChronosFlash core types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for intents
pub type IntentId = Uuid;

/// Unique identifier for routes
pub type RouteId = Uuid;

/// Unique identifier for checkpoints
pub type CheckpointId = Uuid;

/// Chain identifier
pub type ChainId = u64;

/// Address type (32 bytes)
pub type Address = [u8; 32];

/// Hash type (32 bytes)
pub type Hash = [u8; 32];

/// Transaction hash
pub type TxHash = [u8; 32];

/// Balance in smallest units
pub type Balance = u128;

/// Price in fixed-point (18 decimals)
pub type Price = u128;

/// Timestamp in milliseconds
pub type Timestamp = u64;

/// Gas amount
pub type Gas = u64;

/// Basis points (1/100th of a percent)
pub type BasisPoints = u32;

/// Token info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub chain_id: ChainId,
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
}

/// Swap direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapDirection {
    ExactIn,
    ExactOut,
}

/// Trade route hop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHop {
    pub chain_id: ChainId,
    pub protocol: String,
    pub pool_address: Address,
    pub token_in: Token,
    pub token_out: Token,
    pub amount_in: Balance,
    pub expected_out: Balance,
    pub gas_estimate: Gas,
}

/// Complete trade route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRoute {
    pub id: RouteId,
    pub hops: Vec<RouteHop>,
    pub total_input: Balance,
    pub expected_output: Balance,
    pub minimum_output: Balance,
    pub total_gas: Gas,
    pub slippage_bps: BasisPoints,
    pub computed_at: Timestamp,
    pub expires_at: Timestamp,
}

/// Execution checkpoint for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub route_id: RouteId,
    pub chain_id: ChainId,
    pub block_number: u64,
    pub state_root: Hash,
    pub created_at: Timestamp,
}

/// Pre-signed transaction bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreSignedBundle {
    pub id: Uuid,
    pub route: TradeRoute,
    pub checkpoints: Vec<Checkpoint>,
    pub signatures: Vec<Signature>,
    pub created_at: Timestamp,
    pub valid_until: Timestamp,
}

/// Cryptographic signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub chain_id: ChainId,
    pub signer: Address,
    pub signature: Vec<u8>,
    pub recovery_id: u8,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub route_id: RouteId,
    pub success: bool,
    pub actual_output: Balance,
    pub gas_used: Gas,
    pub tx_hashes: Vec<TxHash>,
    pub latency_ms: u64,
    pub time_advantage_ms: i64, // Negative = executed before user submitted
    pub executed_at: Timestamp,
}

/// Chain status for mempool scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatus {
    pub chain_id: ChainId,
    pub name: String,
    pub is_connected: bool,
    pub current_block: u64,
    pub pending_txs: usize,
    pub avg_block_time_ms: u64,
    pub last_updated: Timestamp,
}

/// Mempool statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MempoolStats {
    pub total_pending: usize,
    pub swap_intents_detected: usize,
    pub routes_precomputed: usize,
    pub bundles_presigned: usize,
    pub successful_timewarps: usize,
    pub avg_time_advantage_ms: f64,
    pub chains_monitored: usize,
}

/// Oracle performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OracleMetrics {
    pub intents_predicted: u64,
    pub routes_computed: u64,
    pub bundles_executed: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub avg_time_advantage_ms: f64,
    pub total_volume: Balance,
    pub total_gas_saved: Gas,
}
