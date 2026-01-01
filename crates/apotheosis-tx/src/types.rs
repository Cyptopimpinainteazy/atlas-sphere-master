//! Type definitions for Apotheosis Transaction

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Chain identifier with metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub u64);

impl ChainId {
    // Well-known chain IDs
    pub const ATLAS: Self = Self(0);
    pub const ETHEREUM: Self = Self(1);
    pub const BSC: Self = Self(56);
    pub const POLYGON: Self = Self(137);
    pub const ARBITRUM: Self = Self(42161);
    pub const OPTIMISM: Self = Self(10);
    pub const AVALANCHE: Self = Self(43114);
    pub const BASE: Self = Self(8453);
    pub const SOLANA: Self = Self(1399811149); // Solana mainnet identifier
    pub const COSMOS: Self = Self(118); // Cosmos Hub

    pub fn is_evm(&self) -> bool {
        matches!(
            self.0,
            1 | 56 | 137 | 42161 | 10 | 43114 | 8453 | 250 | 324 | 59144
        )
    }

    pub fn is_svm(&self) -> bool {
        self.0 == 1399811149
    }

    pub fn is_cosmos(&self) -> bool {
        matches!(self.0, 118 | 119 | 120 | 121 | 122)
    }
}

/// Asset type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    /// Native chain token (ETH, SOL, ATOM)
    Native,
    /// ERC-20 or equivalent fungible token
    FungibleToken(String),
    /// NFT (ERC-721 equivalent)
    NonFungible { contract: String, token_id: String },
    /// Semi-fungible (ERC-1155 equivalent)
    SemiFungible {
        contract: String,
        token_id: String,
        amount: u128,
    },
    /// DeFi position (LP tokens, staked assets)
    Position {
        protocol: String,
        position_id: String,
    },
    /// Wrapped/bridged asset
    Wrapped {
        original_chain: ChainId,
        asset: Box<AssetType>,
    },
}

/// Asset to be migrated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationAsset {
    /// Chain where asset currently resides
    pub source_chain: ChainId,
    /// Type of asset
    pub asset_type: AssetType,
    /// Amount (for fungible assets)
    pub amount: Option<u128>,
    /// Contract address (if applicable)
    pub contract: Option<String>,
    /// Estimated USD value
    pub estimated_value_usd: f64,
    /// Priority (higher = migrate first)
    pub priority: u8,
}

/// Source chain with all assets to migrate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceChain {
    /// Chain identifier
    pub chain_id: ChainId,
    /// Account/address on this chain
    pub address: String,
    /// Assets to migrate from this chain
    pub assets: Vec<MigrationAsset>,
    /// Total estimated gas cost
    pub estimated_gas: u128,
    /// Is this chain currently reachable?
    pub is_reachable: bool,
}

/// Destination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    /// Target chain
    pub chain_id: ChainId,
    /// Target address
    pub address: String,
    /// Whether to create new account if needed
    pub create_if_needed: bool,
    /// Preferred token for gas fees
    pub gas_token: Option<AssetType>,
}

/// Migration route for a single asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRoute {
    /// Source asset
    pub asset: MigrationAsset,
    /// Hops to reach destination
    pub hops: Vec<RouteHop>,
    /// Total estimated cost
    pub total_cost: f64,
    /// Estimated time (seconds)
    pub estimated_time: u64,
    /// Risk score (0-100)
    pub risk_score: u8,
}

/// Single hop in a migration route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHop {
    /// Hop type
    pub hop_type: HopType,
    /// Source chain
    pub from_chain: ChainId,
    /// Target chain
    pub to_chain: ChainId,
    /// Protocol used (bridge name, DEX name)
    pub protocol: String,
    /// Estimated gas on source chain
    pub gas_cost: u128,
    /// Estimated time (seconds)
    pub estimated_time: u64,
}

/// Type of route hop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HopType {
    /// Cross-chain bridge
    Bridge,
    /// DEX swap
    Swap,
    /// Unwrap wrapped asset
    Unwrap,
    /// Wrap to bridgeable form
    Wrap,
    /// Exit DeFi position
    ExitPosition,
    /// Direct transfer (same chain)
    Transfer,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Building transaction
    Building,
    /// Awaiting signatures
    AwaitingSignatures,
    /// Executing on chains
    Executing,
    /// Waiting for confirmations
    Confirming,
    /// Successfully completed
    Completed,
    /// Failed (with rollback)
    Failed,
    /// Partially completed (some chains failed)
    PartiallyCompleted,
    /// Rolled back
    RolledBack,
}

/// Chain execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainExecutionStatus {
    /// Chain
    pub chain_id: ChainId,
    /// Status
    pub status: TransactionStatus,
    /// Transaction hash(es)
    pub tx_hashes: Vec<String>,
    /// Error if failed
    pub error: Option<String>,
    /// Block number if confirmed
    pub block_number: Option<u64>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Complete Apotheosis transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApotheosisTransaction {
    /// Unique transaction ID
    pub id: String,
    /// Source chains and assets
    pub sources: Vec<SourceChain>,
    /// Destination
    pub destination: Destination,
    /// Calculated routes
    pub routes: Vec<MigrationRoute>,
    /// Overall status
    pub status: TransactionStatus,
    /// Per-chain status
    pub chain_statuses: Vec<ChainExecutionStatus>,
    /// Total estimated cost (USD)
    pub total_cost_usd: f64,
    /// Total estimated time (seconds)
    pub total_estimated_time: u64,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Started execution timestamp
    pub started_at: Option<DateTime<Utc>>,
    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

impl ApotheosisTransaction {
    /// Get total assets being migrated
    pub fn total_assets(&self) -> usize {
        self.sources.iter().map(|s| s.assets.len()).sum()
    }

    /// Get total chains involved
    pub fn total_chains(&self) -> usize {
        self.sources.len()
    }

    /// Get total value being migrated (USD)
    pub fn total_value_usd(&self) -> f64 {
        self.sources
            .iter()
            .flat_map(|s| &s.assets)
            .map(|a| a.estimated_value_usd)
            .sum()
    }

    /// Check if transaction is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Completed | TransactionStatus::PartiallyCompleted
        )
    }

    /// Check if transaction failed
    pub fn is_failed(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Failed | TransactionStatus::RolledBack
        )
    }
}

/// Statistics about the apotheosis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApotheosisStats {
    /// Total chains migrated from
    pub chains_migrated: usize,
    /// Total assets migrated
    pub assets_migrated: usize,
    /// Total value migrated (USD)
    pub total_value_usd: f64,
    /// Total gas spent (USD)
    pub total_gas_usd: f64,
    /// Time taken (seconds)
    pub time_taken: u64,
    /// Success rate (0.0-1.0)
    pub success_rate: f64,
}
