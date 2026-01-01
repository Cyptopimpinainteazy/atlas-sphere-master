//! External Chain Adapters for Atlas Sphere
//!
//! This crate provides adapters for connecting Atlas Sphere to external EVM-compatible chains.
//! Each adapter translates foreign chain messaging formats into X3 messages the Atlas Kernel understands.
//!
//! # Supported Chains
//! 103 EVM chains via Universal Registry! Including:
//! - Tier 1: Ethereum, Base, Arbitrum, Polygon, Avalanche, BSC, Optimism, Fantom, Cronos, Klaytn
//! - Tier 2: zkSync, Aurora, Harmony, Celo, Metis, Gnosis, Moonbeam, Boba, and more
//! - Tier 3: Scroll, Taiko, Palm, and 80+ more chains
//!
//! # Cross-Chain Swaps
//! Use the `router` module for atomic cross-chain swaps via Comit transactions.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod adapter;
pub mod assets;
pub mod chains;
pub mod error;
pub mod router;
pub mod settlement;

pub use adapter::{ChainAdapter, ChainConfig, ChainMessage, CrossChainTransfer, TransferStatus};
pub use assets::{AssetMetadata, AssetRegistry, MirroredAsset, TokenMapping};
pub use chains::{create_adapter, create_all_adapters};
pub use error::ExternalChainError;
pub use router::{SwapRouter, SwapRoute, AtomicSwapBundle, QuoteResult, quote_swap, find_best_route, build_atomic_swap};
pub use settlement::{SettlementConfig, SettlementProof, SettlementVerifier, ProofType};

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::{boxed::Box, vec::Vec};

/// Re-export chain IDs for convenience
pub mod chain_ids {
    pub const BASE: u64 = 8453;
    pub const ARBITRUM: u64 = 42161;
    pub const POLYGON: u64 = 137;
    pub const AVALANCHE: u64 = 43114;
    pub const BNB: u64 = 56;
    pub const ATLAS_SPHERE: u64 = 42; // Our chain
}

/// Chain type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode, TypeInfo)]
pub enum ChainType {
    Base,
    Arbitrum,
    Polygon,
    Avalanche,
    Bnb,
    AtlasSphere,
}

impl ChainType {
    /// Get chain ID
    pub fn chain_id(&self) -> u64 {
        match self {
            ChainType::Base => chain_ids::BASE,
            ChainType::Arbitrum => chain_ids::ARBITRUM,
            ChainType::Polygon => chain_ids::POLYGON,
            ChainType::Avalanche => chain_ids::AVALANCHE,
            ChainType::Bnb => chain_ids::BNB,
            ChainType::AtlasSphere => chain_ids::ATLAS_SPHERE,
        }
    }

    /// Get chain name
    pub fn name(&self) -> &'static str {
        match self {
            ChainType::Base => "Base",
            ChainType::Arbitrum => "Arbitrum One",
            ChainType::Polygon => "Polygon PoS",
            ChainType::Avalanche => "Avalanche C-Chain",
            ChainType::Bnb => "BNB Smart Chain",
            ChainType::AtlasSphere => "Atlas Sphere",
        }
    }

    /// Get native token symbol
    pub fn native_token(&self) -> &'static str {
        match self {
            ChainType::Base => "ETH",
            ChainType::Arbitrum => "ETH",
            ChainType::Polygon => "MATIC",
            ChainType::Avalanche => "AVAX",
            ChainType::Bnb => "BNB",
            ChainType::AtlasSphere => "ATLAS",
        }
    }

    /// Get default RPC endpoint
    pub fn default_rpc(&self) -> &'static str {
        match self {
            ChainType::Base => "https://mainnet.base.org",
            ChainType::Arbitrum => "https://arb1.arbitrum.io/rpc",
            ChainType::Polygon => "https://polygon-rpc.com",
            ChainType::Avalanche => "https://api.avax.network/ext/bc/C/rpc",
            ChainType::Bnb => "https://bsc-dataseed.binance.org",
            ChainType::AtlasSphere => "http://127.0.0.1:9944",
        }
    }

    /// Get block explorer URL
    pub fn explorer(&self) -> &'static str {
        match self {
            ChainType::Base => "https://basescan.org",
            ChainType::Arbitrum => "https://arbiscan.io",
            ChainType::Polygon => "https://polygonscan.com",
            ChainType::Avalanche => "https://snowtrace.io",
            ChainType::Bnb => "https://bscscan.com",
            ChainType::AtlasSphere => "http://explorer.x3",
        }
    }

    /// Is this an L2 chain?
    pub fn is_l2(&self) -> bool {
        matches!(self, ChainType::Base | ChainType::Arbitrum)
    }

    /// Average block time in seconds
    pub fn block_time_secs(&self) -> u64 {
        match self {
            ChainType::Base => 2,
            ChainType::Arbitrum => 1, // Sub-second but ~1s for finality
            ChainType::Polygon => 2,
            ChainType::Avalanche => 2,
            ChainType::Bnb => 3,
            ChainType::AtlasSphere => 6,
        }
    }
}

impl From<u64> for ChainType {
    fn from(chain_id: u64) -> Self {
        match chain_id {
            chain_ids::BASE => ChainType::Base,
            chain_ids::ARBITRUM => ChainType::Arbitrum,
            chain_ids::POLYGON => ChainType::Polygon,
            chain_ids::AVALANCHE => ChainType::Avalanche,
            chain_ids::BNB => ChainType::Bnb,
            _ => ChainType::AtlasSphere,
        }
    }
}

/// Multi-chain manager for coordinating operations across all chains
pub struct MultiChainManager {
    adapters: Vec<Box<dyn ChainAdapter>>,
    asset_registry: AssetRegistry,
}

impl MultiChainManager {
    /// Create new multi-chain manager with all supported chains
    pub fn new() -> Self {
        Self {
            adapters: create_all_adapters(),
            asset_registry: AssetRegistry::new(),
        }
    }

    /// Get adapter for a specific chain
    pub fn get_adapter(&self, chain: ChainType) -> Option<&dyn ChainAdapter> {
        self.adapters
            .iter()
            .find(|a| a.chain_type() == chain)
            .map(|a| a.as_ref())
    }

    /// Get all adapters
    pub fn adapters(&self) -> &[Box<dyn ChainAdapter>] {
        &self.adapters
    }

    /// Get asset registry
    pub fn asset_registry(&self) -> &AssetRegistry {
        &self.asset_registry
    }

    /// Get mutable asset registry
    pub fn asset_registry_mut(&mut self) -> &mut AssetRegistry {
        &mut self.asset_registry
    }

    /// Get all supported chain types
    pub fn supported_chains(&self) -> Vec<ChainType> {
        self.adapters.iter().map(|a| a.chain_type()).collect()
    }

    /// Check connectivity to all chains
    pub async fn check_all_connectivity(&self) -> Vec<(ChainType, bool)> {
        let mut results = Vec::new();
        for adapter in &self.adapters {
            let connected = adapter.is_connected().await;
            results.push((adapter.chain_type(), connected));
        }
        results
    }
}

impl Default for MultiChainManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_ids() {
        assert_eq!(ChainType::Base.chain_id(), 8453);
        assert_eq!(ChainType::Arbitrum.chain_id(), 42161);
        assert_eq!(ChainType::Polygon.chain_id(), 137);
        assert_eq!(ChainType::Avalanche.chain_id(), 43114);
        assert_eq!(ChainType::Bnb.chain_id(), 56);
    }

    #[test]
    fn test_chain_from_id() {
        assert_eq!(ChainType::from(8453), ChainType::Base);
        assert_eq!(ChainType::from(42161), ChainType::Arbitrum);
        assert_eq!(ChainType::from(137), ChainType::Polygon);
    }

    #[test]
    fn test_native_tokens() {
        assert_eq!(ChainType::Base.native_token(), "ETH");
        assert_eq!(ChainType::Polygon.native_token(), "MATIC");
        assert_eq!(ChainType::Bnb.native_token(), "BNB");
    }
}
