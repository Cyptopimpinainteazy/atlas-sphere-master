//! Atlas Sphere X3 VM Integration
//!
//! This crate provides the bridge between the Atlas Kernel pallet and the X3
//! virtual machine. It enables execution of X3 bytecode alongside EVM and SVM
//! in atomic cross-VM transactions.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        Atlas Kernel Pallet                          │
//! │                                                                     │
//! │  ┌───────────────┐ ┌───────────────┐ ┌───────────────────────────┐ │
//! │  │ EVM Adapter   │ │ SVM Adapter   │ │ X3 Adapter (this crate)   │ │
//! │  └───────┬───────┘ └───────┬───────┘ └─────────────┬─────────────┘ │
//! └──────────│─────────────────│───────────────────────│───────────────┘
//!            │                 │                       │
//!            ▼                 ▼                       ▼
//! ┌──────────────────┐ ┌──────────────┐ ┌────────────────────────────┐
//! │ Frontier EVM     │ │ solana-rbpf  │ │ X3 VM (x3-vm crate)        │
//! │ (pallet-evm)     │ │              │ │ • Bytecode verification    │
//! └──────────────────┘ └──────────────┘ │ • Deterministic execution  │
//!                                       │ • Hostcall bridge          │
//!                                       └────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Bytecode Execution**: Execute verified X3BC modules with gas metering
//! - **Hostcall Bridge**: Connect X3 hostcalls to Substrate storage/events
//! - **Cross-VM State**: Bridge X3 state changes to canonical ledger
//! - **Gas Translation**: Convert X3 gas units to Substrate weight
//!
//! # Example
//!
//! ```ignore
//! use atlas_x3_integration::{X3Executor, X3ExecutorConfig};
//!
//! // Execute X3 bytecode
//! let config = X3ExecutorConfig::default();
//! let receipt = X3Executor::execute(&bytecode, &[], config)?;
//!
//! // Check execution result
//! assert!(receipt.success);
//! println!("Gas used: {}", receipt.gas_used);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec, vec::Vec};

pub mod error;
pub mod executor;
pub mod hostcalls;
pub mod types;

#[cfg(feature = "compile")]
pub mod compiler_bridge;

pub use error::{X3IntegrationError, X3Result};
pub use executor::{X3Executor, X3ExecutorConfig};
pub use types::{X3ExecutionReceipt, X3StateChange};

#[cfg(feature = "std")]
pub use hostcalls::SubstrateHostcalls;
