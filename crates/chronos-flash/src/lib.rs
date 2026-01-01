//! # ChronosFlash - Negative-Latency Pre-Execution Oracle
//!
//! The world's first negative-latency DEX that executes swaps
//! BEFORE the user even submits them.
//!
//! ## How It Works
//!
//! 1. AI swarm watches mempools across ALL 103+ chains + Bitcoin + Lightning
//! 2. Evolution Core breeds optimal cross-chain routes in <50ms
//! 3. Pre-sign and pre-broadcast atomic bundles with checkpoint rollback
//! 4. User's swap confirms 100-400ms BEFORE they click "approve"
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    CHRONOSFLASH ORACLE                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐       │
//! │  │ MEMPOOL       │  │ INTENT        │  │ QUANTUM       │       │
//! │  │ SCANNER       │→ │ PREDICTOR     │→ │ ROUTER        │       │
//! │  │ (103 chains)  │  │ (AI Swarm)    │  │ (Evolution)   │       │
//! │  └───────────────┘  └───────────────┘  └───────────────┘       │
//! │          │                  │                  │                │
//! │          ▼                  ▼                  ▼                │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │              PRE-EXECUTION ENGINE                       │   │
//! │  │  • Pre-sign atomic bundles                              │   │
//! │  │  • Checkpoint rollback                                  │   │
//! │  │  • Multi-chain finality tracking                        │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │                              │                                  │
//! │                              ▼                                  │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │              TIME-WARP EXECUTOR                         │   │
//! │  │  • Execute BEFORE user submits                          │   │
//! │  │  • Reality distortion: 100-400ms advantage              │   │
//! │  │  • Atomic cross-chain settlement                        │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod config;
pub mod error;
pub mod intent;
pub mod mempool;
pub mod oracle;
pub mod predictor;
pub mod router;
pub mod timewarp;
pub mod types;

pub use config::ChronosConfig;
pub use error::{ChronosError, ChronosResult};
pub use intent::{IntentDetector, IntentType, PredictedIntent, SwapIntent};
pub use mempool::{MempoolScanner, PendingTx};
pub use oracle::{ChronosOracle, ChronosOracleBuilder};
pub use predictor::IntentPredictor;
pub use router::QuantumRouter;
pub use timewarp::{Signer, TimeWarpEngine};
pub use types::*;

/// ChronosFlash version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum chains supported
pub const MAX_CHAINS: usize = 150;

/// Target pre-execution latency (ms)
pub const TARGET_LATENCY_MS: u64 = 50;

/// Maximum time-warp advantage (ms)
pub const MAX_TIMEWARP_MS: u64 = 400;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::config::ChronosConfig;
    pub use crate::error::{ChronosError, ChronosResult};
    pub use crate::intent::{IntentDetector, IntentType, SwapIntent};
    pub use crate::oracle::{ChronosOracle, ChronosOracleBuilder};
    pub use crate::predictor::IntentPredictor;
    pub use crate::router::QuantumRouter;
    pub use crate::timewarp::{Signer, TimeWarpEngine};
    pub use crate::types::*;
}
