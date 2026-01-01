//! # Atlas Sphere Rust SDK
//!
//! A comprehensive Rust SDK for interacting with the Atlas Sphere blockchain,
//! supporting both EVM and SVM virtual machines.
//!
//! ## Features
//!
//! - **Dual-VM Support**: Native interaction with both EVM and SVM payloads
//! - **Comit Transactions**: Build and submit atomic cross-VM transactions
//! - **RPC Client**: Full JSON-RPC and WebSocket client
//! - **Type Safety**: Strongly typed interfaces matching runtime types
//! - **Async First**: Built on tokio for high-performance async operations
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use atlas_sdk::{AtlasClient, ComitBuilder, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Connect to Atlas Sphere node
//!     let client = AtlasClient::connect("http://localhost:9944").await?;
//!
//!     // Build a Comit transaction
//!     let comit = ComitBuilder::new()
//!         .with_evm_payload(&[0x60, 0x80, 0x60, 0x40])
//!         .with_fee(1_000_000)
//!         .build()?;
//!
//!     // Submit to chain (note: requires signer to be configured)
//!     // let result = client.submit_comit(comit).await?;
//!     // println!("Comit submitted: {:?}", result.tx_hash);
//!
//!     Ok(())
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod client;
pub mod comit;
pub mod error;
pub mod evm;
pub mod rpc;
pub mod svm;
pub mod types;
pub mod utils;

// Re-exports for convenience
pub use client::{AtlasClient, Signer, Sr25519Signer};
pub use comit::ComitBuilder;
pub use error::{AtlasError, Result};
pub use types::*;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default WebSocket endpoint
pub const DEFAULT_WS_ENDPOINT: &str = "ws://localhost:9944";

/// Default HTTP endpoint
pub const DEFAULT_HTTP_ENDPOINT: &str = "http://localhost:9944";

/// Testnet WebSocket endpoint
pub const TESTNET_WS_ENDPOINT: &str = "ws://rpc.testnet.atlas-sphere.io:9944";

/// Testnet HTTP endpoint
pub const TESTNET_HTTP_ENDPOINT: &str = "http://rpc.testnet.atlas-sphere.io:9944";

/// Mainnet HTTP endpoint
pub const MAINNET_HTTP_ENDPOINT: &str = "http://rpc.atlas-sphere.io:9944";

/// Mainnet WebSocket endpoint
pub const MAINNET_WS_ENDPOINT: &str = "ws://rpc.atlas-sphere.io:9944";

/// Native asset symbol
pub const NATIVE_ASSET_SYMBOL: &str = "ATLAS";

/// Native asset decimals
pub const NATIVE_ASSET_DECIMALS: u8 = 18;

/// Maximum EVM payload size (16 KB)
pub const MAX_EVM_PAYLOAD_SIZE: usize = 16 * 1024;

/// Maximum SVM payload size (16 KB)
pub const MAX_SVM_PAYLOAD_SIZE: usize = 16 * 1024;

/// Maximum combined payload size (32 KB)
pub const MAX_COMBINED_PAYLOAD_SIZE: usize = 32 * 1024;

/// Maximum single payload size (for validation)
pub const MAX_PAYLOAD_SIZE: usize = MAX_EVM_PAYLOAD_SIZE;

/// Chain ID for Atlas Sphere mainnet
pub const MAINNET_CHAIN_ID: u64 = 650_000;

/// Chain ID for Atlas Sphere testnet
pub const TESTNET_CHAIN_ID: u64 = 650_001;

/// Default gas price in native units
pub const DEFAULT_GAS_PRICE: u128 = 1_000_000_000; // 1 gwei equivalent
