//! Database models for indexed data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Indexed block.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Block {
    pub number: i64,
    pub hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub timestamp: DateTime<Utc>,
    pub author: Option<String>,
    pub extrinsic_count: i32,
    pub event_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Indexed extrinsic (transaction).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Extrinsic {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: i32,
    pub hash: String,
    pub pallet: String,
    pub call: String,
    pub signer: Option<String>,
    pub success: bool,
    pub fee: Option<String>,
    pub raw_data: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}

/// Indexed event.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: Option<i32>,
    pub event_index: i32,
    pub pallet: String,
    pub variant: String,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Indexed Comit transaction.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ComitTransaction {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: i32,
    pub comit_hash: String,
    pub origin: String,
    pub evm_payload_size: i32,
    pub svm_payload_size: i32,
    pub evm_gas_used: Option<i64>,
    pub svm_compute_used: Option<i64>,
    pub fee_paid: String,
    pub success: bool,
    pub evm_success: Option<bool>,
    pub svm_success: Option<bool>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Indexed account.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub native_balance: String,
    pub nonce: i64,
    pub is_authorized: bool,
    pub first_seen_block: i64,
    pub last_seen_block: i64,
    pub total_transactions: i64,
    pub updated_at: DateTime<Utc>,
}

/// Indexed asset balance.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AssetBalance {
    pub id: i64,
    pub account: String,
    pub asset_id: String,
    pub balance: String,
    pub updated_at: DateTime<Utc>,
}

/// Indexed EVM log.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EvmLog {
    pub id: i64,
    pub block_number: i64,
    pub transaction_index: i32,
    pub log_index: i32,
    pub contract_address: String,
    pub topic0: Option<String>,
    pub topic1: Option<String>,
    pub topic2: Option<String>,
    pub topic3: Option<String>,
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

/// Indexed SVM instruction.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SvmInstruction {
    pub id: i64,
    pub block_number: i64,
    pub transaction_index: i32,
    pub instruction_index: i32,
    pub program_id: String,
    pub accounts: serde_json::Value,
    pub data: Vec<u8>,
    pub success: bool,
    pub created_at: DateTime<Utc>,
}

/// Indexer state tracking.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct IndexerState {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// New record types (for insertion)
// ============================================================================

/// New block to insert.
#[derive(Debug, Clone)]
pub struct NewBlock {
    pub number: i64,
    pub hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub timestamp: DateTime<Utc>,
    pub author: Option<String>,
    pub extrinsic_count: i32,
    pub event_count: i32,
}

/// New extrinsic to insert.
#[derive(Debug, Clone)]
pub struct NewExtrinsic {
    pub block_number: i64,
    pub extrinsic_index: i32,
    pub hash: String,
    pub pallet: String,
    pub call: String,
    pub signer: Option<String>,
    pub success: bool,
    pub fee: Option<String>,
    pub raw_data: Option<Vec<u8>>,
}

/// New event to insert.
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub block_number: i64,
    pub extrinsic_index: Option<i32>,
    pub event_index: i32,
    pub pallet: String,
    pub variant: String,
    pub data: serde_json::Value,
}

/// New Comit transaction to insert.
#[derive(Debug, Clone)]
pub struct NewComitTransaction {
    pub block_number: i64,
    pub extrinsic_index: i32,
    pub comit_hash: String,
    pub origin: String,
    pub evm_payload_size: i32,
    pub svm_payload_size: i32,
    pub evm_gas_used: Option<i64>,
    pub svm_compute_used: Option<i64>,
    pub fee_paid: String,
    pub success: bool,
    pub evm_success: Option<bool>,
    pub svm_success: Option<bool>,
    pub error_message: Option<String>,
}

/// New EVM log to insert.
#[derive(Debug, Clone)]
pub struct NewEvmLog {
    pub block_number: i64,
    pub transaction_index: i32,
    pub log_index: i32,
    pub contract_address: String,
    pub topic0: Option<String>,
    pub topic1: Option<String>,
    pub topic2: Option<String>,
    pub topic3: Option<String>,
    pub data: Vec<u8>,
}
