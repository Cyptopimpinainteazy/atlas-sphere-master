//! Chain-specific adapter implementations
//!
//! Each chain module provides a concrete adapter implementation
//! for interacting with that specific external chain.
//!
//! # Architecture
//! - Individual adapters (base, arbitrum, etc.) for chain-specific logic
//! - Universal adapter for ANY EVM chain via registry
//! - Registry contains 100+ chains with metadata

pub mod arbitrum;
pub mod avalanche;
pub mod base;
pub mod bnb;
pub mod polygon;
pub mod registry;
pub mod universal;

// Re-export specific adapters
pub use arbitrum::ArbitrumAdapter;
pub use avalanche::AvalancheAdapter;
pub use base::BaseAdapter;
pub use bnb::BnbAdapter;
pub use polygon::PolygonAdapter;

// Re-export universal adapter and registry
pub use registry::{all_chain_ids, chain_count, get_chain, ChainInfo, ALL_CHAINS};
pub use universal::{adapter_for, create_all_universal_adapters, UniversalEvmAdapter};

use crate::adapter::{ChainAdapter, ChainConfig};
use crate::ChainType;
use sp_std::boxed::Box;

/// Create an adapter for the specified chain type
pub fn create_adapter(chain: ChainType, config: ChainConfig) -> Box<dyn ChainAdapter> {
    match chain {
        ChainType::Base => Box::new(BaseAdapter::new(config)),
        ChainType::Arbitrum => Box::new(ArbitrumAdapter::new(config)),
        ChainType::Polygon => Box::new(PolygonAdapter::new(config)),
        ChainType::Avalanche => Box::new(AvalancheAdapter::new(config)),
        ChainType::Bnb => Box::new(BnbAdapter::new(config)),
        ChainType::AtlasSphere => {
            // For Atlas Sphere, we use a pass-through adapter
            // since it's the local chain
            Box::new(base::BaseAdapter::new(config))
        }
    }
}

/// Create adapters for all supported external chains
pub fn create_all_adapters() -> Vec<Box<dyn ChainAdapter>> {
    vec![
        Box::new(BaseAdapter::new(ChainConfig::for_chain(ChainType::Base))),
        Box::new(ArbitrumAdapter::new(ChainConfig::for_chain(
            ChainType::Arbitrum,
        ))),
        Box::new(PolygonAdapter::new(ChainConfig::for_chain(
            ChainType::Polygon,
        ))),
        Box::new(AvalancheAdapter::new(ChainConfig::for_chain(
            ChainType::Avalanche,
        ))),
        Box::new(BnbAdapter::new(ChainConfig::for_chain(ChainType::Bnb))),
    ]
}
