//! X3 Static Verifier
//!
//! Performs static analysis on X3 MIR and bytecode to verify:
//! - Gas bounds (per-function and per-contract limits)
//! - Forbidden operation detection
//! - Determinism checking (no floating point, no timestamps, etc.)
//! - Atomic window verification
//! - Type safety at MIR level
//!
//! # Usage
//! ```ignore
//! use x3_verifier::{Verifier, SafetyRules, VerificationReport};
//!
//! let rules = SafetyRules::load("contract-safety.yaml")?;
//! let verifier = Verifier::new(rules);
//! let report = verifier.verify_mir(&mir)?;
//! if !report.passed() {
//!     for error in report.errors() {
//!         eprintln!("Error: {}", error);
//!     }
//! }
//! ```

mod error;
mod gas;
mod rules;
mod verifier;

pub use error::{VerifierError, VerifierResult};
pub use gas::{FunctionGas, GasAnalyzer, GasReport};
pub use rules::{GasCost, OpcodeClass, SafetyRules};
pub use verifier::{Severity, VerificationError, VerificationReport, Verifier};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        GasAnalyzer, GasReport, SafetyRules, Severity, VerificationError, VerificationReport,
        Verifier,
    };
}
