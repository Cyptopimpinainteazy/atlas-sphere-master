//! Core types used throughout the Atlas SDK.

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::H256;

/// Account ID type (32 bytes, compatible with Substrate SS58)
pub type AccountId = sp_core::crypto::AccountId32;

/// Balance type (128-bit unsigned integer)
pub type Balance = u128;

/// Nonce type (64-bit unsigned integer)
pub type Nonce = u64;

/// Block number type
pub type BlockNumber = u64;

/// Asset ID type
pub type AssetId = H256;

/// Transaction hash type
pub type TxHash = H256;

/// Chain ID type
pub type ChainId = u64;

/// Gas/compute units type
pub type Gas = u64;

// ============================================================================
// Comit Transaction Types
// ============================================================================

/// A Comit transaction payload for atomic cross-VM execution.
#[derive(Clone, Debug, Default, Encode, Decode, Serialize, Deserialize)]
pub struct ComitPayload {
    /// EVM payload bytes (optional)
    pub evm_payload: Option<Vec<u8>>,

    /// SVM payload bytes (optional)
    pub svm_payload: Option<Vec<u8>>,

    /// Nonce for replay protection
    pub nonce: Nonce,

    /// Prepare root commitment (hash of inputs)
    pub prepare_root: H256,

    /// EVM gas limit
    pub evm_gas_limit: Gas,

    /// SVM compute unit limit
    pub svm_compute_limit: Gas,

    /// Fee in native tokens
    pub fee: Balance,

    /// Deadline block number (0 = no deadline)
    pub deadline: BlockNumber,
}

impl ComitPayload {
    /// Check if this is an EVM-only comit
    pub fn is_evm_only(&self) -> bool {
        self.evm_payload.is_some() && self.svm_payload.is_none()
    }

    /// Check if this is an SVM-only comit
    pub fn is_svm_only(&self) -> bool {
        self.svm_payload.is_some() && self.evm_payload.is_none()
    }

    /// Check if this is a dual-VM comit
    pub fn is_dual_vm(&self) -> bool {
        self.evm_payload.is_some() && self.svm_payload.is_some()
    }

    /// Get total payload size
    pub fn payload_size(&self) -> usize {
        let evm_size = self.evm_payload.as_ref().map(|p| p.len()).unwrap_or(0);
        let svm_size = self.svm_payload.as_ref().map(|p| p.len()).unwrap_or(0);
        evm_size + svm_size
    }
}

/// Result of a Comit transaction execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComitResult {
    /// Transaction hash
    pub tx_hash: TxHash,

    /// Block number included in
    pub block_number: BlockNumber,

    /// Block hash
    pub block_hash: H256,

    /// Whether execution was successful
    pub success: bool,

    /// EVM execution receipt (if applicable)
    pub evm_receipt: Option<EvmReceipt>,

    /// SVM execution receipt (if applicable)
    pub svm_receipt: Option<SvmReceipt>,

    /// Total gas used
    pub total_gas_used: Gas,

    /// Actual fee paid
    pub fee_paid: Balance,

    /// Events emitted
    pub events: Vec<ComitEvent>,
}

// ============================================================================
// VM-Specific Types
// ============================================================================

/// EVM execution receipt.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvmReceipt {
    /// Success flag
    pub success: bool,

    /// Gas used
    pub gas_used: Gas,

    /// Return data
    pub return_data: Vec<u8>,

    /// Logs emitted
    pub logs: Vec<EvmLog>,

    /// Contract address (if deployment)
    pub contract_address: Option<[u8; 20]>,

    /// Revert reason (if failed)
    pub revert_reason: Option<String>,
}

/// EVM log entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvmLog {
    /// Contract address
    pub address: [u8; 20],

    /// Log topics
    pub topics: Vec<H256>,

    /// Log data
    pub data: Vec<u8>,
}

/// SVM execution receipt.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SvmReceipt {
    /// Success flag
    pub success: bool,

    /// Compute units used
    pub compute_units_used: Gas,

    /// Return data
    pub return_data: Vec<u8>,

    /// Program logs
    pub logs: Vec<String>,

    /// Error message (if failed)
    pub error: Option<String>,
}

// ============================================================================
// Event Types
// ============================================================================

/// Event emitted by Comit execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ComitEvent {
    /// Comit submitted
    ComitSubmitted {
        comit_id: H256,
        origin: AccountId,
        evm_payload_size: u32,
        svm_payload_size: u32,
    },

    /// Comit executed successfully
    ComitExecuted {
        comit_id: H256,
        evm_gas_used: Gas,
        svm_compute_used: Gas,
        final_root: H256,
    },

    /// Comit failed
    ComitFailed { comit_id: H256, reason: String },

    /// State change applied
    StateChange {
        asset_id: AssetId,
        account: AccountId,
        delta: i128,
    },

    /// Fee paid
    FeePaid { from: AccountId, amount: Balance },
}

// ============================================================================
// Account Types
// ============================================================================

/// Account information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account ID
    pub account: AccountId,

    /// Native balance
    pub native_balance: Balance,

    /// Current nonce
    pub nonce: Nonce,

    /// Whether account is authorized for Comit
    pub is_authorized: bool,

    /// Asset balances
    pub asset_balances: Vec<(AssetId, Balance)>,
}

/// Asset metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetMetadata {
    /// Asset ID
    pub id: AssetId,

    /// Symbol (e.g., "ETH", "SOL")
    pub symbol: String,

    /// Decimal places
    pub decimals: u8,

    /// Total supply
    pub total_supply: Option<Balance>,
}

// ============================================================================
// Block Types
// ============================================================================

/// Block header information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block number
    pub number: BlockNumber,

    /// Block hash
    pub hash: H256,

    /// Parent hash
    pub parent_hash: H256,

    /// State root
    pub state_root: H256,

    /// Extrinsics root
    pub extrinsics_root: H256,

    /// Timestamp
    pub timestamp: u64,
}

/// Full block information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,

    /// Extrinsics in this block
    pub extrinsics: Vec<ExtrinsicInfo>,
}

/// Extrinsic (transaction) information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtrinsicInfo {
    /// Extrinsic index in block
    pub index: u32,

    /// Extrinsic hash
    pub hash: TxHash,

    /// Whether successful
    pub success: bool,

    /// Signer (if signed)
    pub signer: Option<AccountId>,

    /// Pallet name
    pub pallet: String,

    /// Call name
    pub call: String,
}

// ============================================================================
// Query Response Types
// ============================================================================

/// Response for balance queries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    /// Free balance
    pub free: Balance,

    /// Reserved balance
    pub reserved: Balance,

    /// Frozen balance
    pub frozen: Balance,
}

/// Response for storage queries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageResponse {
    /// Storage key
    pub key: Vec<u8>,

    /// Storage value (SCALE-encoded)
    pub value: Option<Vec<u8>>,

    /// Block hash at which queried
    pub at: H256,
}

// ============================================================================
// Subscription Types
// ============================================================================

/// Subscription message types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SubscriptionMessage {
    /// New block finalized
    NewBlock(BlockHeader),

    /// Comit event
    ComitEvent(ComitEvent),

    /// Storage change
    StorageChange {
        key: Vec<u8>,
        value: Option<Vec<u8>>,
    },
}

/// Subscription handle for managing subscriptions.
#[derive(Debug)]
pub struct SubscriptionHandle {
    /// Subscription ID
    pub id: String,

    /// Whether subscription is active
    pub active: bool,
}

// ============================================================================
// Network Types
// ============================================================================

/// Network status information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkStatus {
    /// Chain name
    pub chain: String,

    /// Chain ID
    pub chain_id: ChainId,

    /// Node name
    pub node_name: String,

    /// Node version
    pub node_version: String,

    /// Current best block
    pub best_block: BlockNumber,

    /// Finalized block
    pub finalized_block: BlockNumber,

    /// Number of connected peers
    pub peer_count: u32,

    /// Whether node is syncing
    pub is_syncing: bool,
}

/// Runtime version information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeVersion {
    /// Spec name
    pub spec_name: String,

    /// Impl name
    pub impl_name: String,

    /// Spec version
    pub spec_version: u32,

    /// Impl version
    pub impl_version: u32,

    /// Transaction version
    pub transaction_version: u32,
}
