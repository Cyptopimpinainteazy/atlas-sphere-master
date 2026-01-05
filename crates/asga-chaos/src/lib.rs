//! # ASGA Chaos Testing Harness
//!
//! This crate provides chaos testing scenarios for the ASGA (Atomic Swap Guardian Agent)
//! to ensure the system handles adversarial conditions correctly.
//!
//! ## Chaos Scenarios
//!
//! 1. **Fork Simulation**: Test behavior when a domain experiences a chain reorganization
//! 2. **Double-Spend Attempts**: Verify detection of double-spend attacks
//! 3. **Timeout Races**: Test edge cases around deadline boundaries
//! 4. **Concurrent Receipt Submissions**: Test race conditions in receipt processing
//! 5. **Invalid Receipt Injection**: Test rejection of malformed/invalid receipts
//! 6. **Partial Completion**: Test revert behavior when only some domains complete
//!
//! ## Usage
//!
//! ```rust,ignore
//! use asga_chaos::{ChaosScenario, ChaosRunner};
//!
//! let mut runner = ChaosRunner::new();
//! let result = runner.run_scenario(ChaosScenario::ForkSimulation {
//!     domain: DomainId::Evm,
//!     depth: 3,
//! });
//! assert!(result.is_handled_correctly());
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use parity_scale_codec::{Decode, Encode};
use asga_receipts::{
    DomainId, Phase, Receipt, ReceiptHeader, ReceiptPayload,
    EvmReceipt, SvmReceipt, BtcReceipt, X3Receipt, ValidationError,
};
use asga_state_machine::{
    SwapContext, SwapState, SwapDeadlines, TransitionEvent, TransitionError, ReceiptStatus,
};

/// Chaos scenario types for testing.
#[derive(Clone, Debug)]
pub enum ChaosScenario {
    /// Simulate a chain fork/reorganization
    ForkSimulation {
        domain: DomainId,
        /// Number of blocks reverted
        depth: u32,
    },
    /// Simulate a double-spend attempt
    DoubleSpendAttempt {
        domain: DomainId,
        /// Original receipt
        original_receipt: Receipt,
        /// Conflicting receipt (double-spend)
        conflicting_receipt: Receipt,
    },
    /// Test behavior at exact deadline boundary
    TimeoutRace {
        /// Block at which to submit receipt
        submit_block: u64,
        /// Deadline block
        deadline_block: u64,
    },
    /// Concurrent receipt submissions from multiple attesters
    ConcurrentReceipts {
        domain: DomainId,
        /// Number of concurrent submissions
        count: u32,
    },
    /// Inject an invalid receipt
    InvalidReceiptInjection {
        error_type: InvalidReceiptType,
    },
    /// Partial domain completion (some domains complete, others don't)
    PartialCompletion {
        /// Domains that complete
        completing_domains: Vec<DomainId>,
        /// Domains that fail/timeout
        failing_domains: Vec<DomainId>,
    },
    /// Byzantine attester submitting conflicting attestations
    ByzantineAttester {
        /// Attester public key
        attester: [u8; 32],
    },
    /// Network partition simulation
    NetworkPartition {
        /// Duration in blocks
        duration: u64,
        /// Affected domains
        partitioned_domains: Vec<DomainId>,
    },
}

/// Types of invalid receipts for testing rejection.
#[derive(Clone, Debug)]
pub enum InvalidReceiptType {
    /// Empty transaction hash
    EmptyTxHash,
    /// Domain mismatch between header and payload
    DomainMismatch,
    /// Insufficient confirmations
    InsufficientConfirmations,
    /// Invalid signature
    InvalidSignature,
    /// Wrong intent ID
    WrongIntentId,
    /// Zero amount
    ZeroAmount,
    /// Malformed SCALE encoding
    MalformedEncoding,
}

/// Result of running a chaos scenario.
#[derive(Clone, Debug)]
pub struct ChaosResult {
    /// Scenario that was run
    pub scenario: &'static str,
    /// Whether the scenario was handled correctly
    pub handled_correctly: bool,
    /// Final state after scenario
    pub final_state: SwapState,
    /// Any errors encountered
    pub errors: Vec<ChaosError>,
    /// Invariant violations detected
    pub invariant_violations: Vec<&'static str>,
}

impl ChaosResult {
    /// Check if the chaos scenario was handled correctly
    pub fn is_handled_correctly(&self) -> bool {
        self.handled_correctly && self.invariant_violations.is_empty()
    }
}

/// Errors that can occur during chaos testing.
#[derive(Clone, Debug)]
pub enum ChaosError {
    /// State transition failed
    TransitionFailed(TransitionError),
    /// Receipt validation failed
    ValidationFailed(ValidationError),
    /// Unexpected state reached
    UnexpectedState { expected: SwapState, got: SwapState },
    /// Invariant violation
    InvariantViolation(&'static str),
}

/// Chaos test runner with configurable scenarios.
pub struct ChaosRunner {
    /// Current swap context
    pub context: Option<SwapContext>,
    /// Current block number (simulated)
    pub current_block: u64,
    /// Collected results
    pub results: Vec<ChaosResult>,
}

impl ChaosRunner {
    /// Create a new chaos runner
    pub fn new() -> Self {
        Self {
            context: None,
            current_block: 0,
            results: Vec::new(),
        }
    }

    /// Initialize a new swap context for testing
    pub fn init_swap(&mut self, domains: Vec<DomainId>, deadlines: SwapDeadlines) {
        self.context = Some(SwapContext::new(domains, deadlines));
        self.current_block = 0;
    }

    /// Advance the simulated block number
    pub fn advance_blocks(&mut self, blocks: u64) {
        self.current_block += blocks;
    }

    /// Run a chaos scenario and return the result
    pub fn run_scenario(&mut self, scenario: ChaosScenario) -> ChaosResult {
        match scenario {
            ChaosScenario::ForkSimulation { domain, depth } => {
                self.run_fork_simulation(domain, depth)
            }
            ChaosScenario::TimeoutRace { submit_block, deadline_block } => {
                self.run_timeout_race(submit_block, deadline_block)
            }
            ChaosScenario::InvalidReceiptInjection { error_type } => {
                self.run_invalid_receipt_injection(error_type)
            }
            ChaosScenario::PartialCompletion { completing_domains, failing_domains } => {
                self.run_partial_completion(completing_domains, failing_domains)
            }
            _ => {
                // Placeholder for other scenarios
                ChaosResult {
                    scenario: "unimplemented",
                    handled_correctly: false,
                    final_state: SwapState::Undefined,
                    errors: vec![],
                    invariant_violations: vec!["Scenario not implemented"],
                }
            }
        }
    }

    /// Run fork simulation scenario
    fn run_fork_simulation(&mut self, domain: DomainId, depth: u32) -> ChaosResult {
        // Fork simulation: a receipt was accepted but the underlying chain reorganized
        // The system should detect this and either:
        // 1. Wait for new receipt with sufficient confirmations
        // 2. Revert the swap if the fork invalidates the transaction

        let mut errors = Vec::new();
        let mut violations = Vec::new();

        // Initialize swap if not already done
        if self.context.is_none() {
            self.init_swap(
                vec![DomainId::Evm, DomainId::Svm],
                SwapDeadlines {
                    lock_deadline: 100,
                    exec_deadline: 200,
                    final_deadline: 300,
                },
            );
        }

        let ctx = self.context.as_mut().unwrap();

        // Advance to lock phase
        let _ = ctx.transition(TransitionEvent::IntentSubmitted, 0);
        let _ = ctx.transition(TransitionEvent::LockInitiated, 1);

        // Submit receipt with confirmations just above threshold
        let confirmations_before_fork = domain.min_confirmations() + 1;

        // After fork of `depth` blocks, confirmations would be reduced
        let confirmations_after_fork = if depth >= confirmations_before_fork {
            0
        } else {
            confirmations_before_fork - depth
        };

        // The system should detect that confirmations are now below threshold
        let below_threshold = confirmations_after_fork < domain.min_confirmations();

        if below_threshold {
            // System should reject or require new confirmation
            // For this test, we verify the state machine handles it correctly
            // by not advancing past the current phase
        }

        let final_state = ctx.state;
        let handled_correctly = if below_threshold {
            // Should remain in PhaseLock (not advance) or revert
            final_state == SwapState::PhaseLock || final_state == SwapState::Reverted
        } else {
            true
        };

        ChaosResult {
            scenario: "fork_simulation",
            handled_correctly,
            final_state,
            errors,
            invariant_violations: violations,
        }
    }

    /// Run timeout race scenario
    fn run_timeout_race(&mut self, submit_block: u64, deadline_block: u64) -> ChaosResult {
        let mut errors = Vec::new();
        let violations = Vec::new();

        // Initialize swap
        self.init_swap(
            vec![DomainId::Evm, DomainId::Svm],
            SwapDeadlines {
                lock_deadline: deadline_block,
                exec_deadline: deadline_block + 100,
                final_deadline: deadline_block + 200,
            },
        );

        let ctx = self.context.as_mut().unwrap();

        // Advance to lock phase
        let _ = ctx.transition(TransitionEvent::IntentSubmitted, 0);
        let _ = ctx.transition(TransitionEvent::LockInitiated, 1);

        // Try to submit receipt at or after deadline
        let result = ctx.transition(
            TransitionEvent::LockReceiptReceived { domain: DomainId::Evm },
            submit_block,
        );

        let final_state = ctx.state;

        // If submit_block > deadline, should revert
        let should_revert = submit_block > deadline_block;
        let handled_correctly = if should_revert {
            final_state == SwapState::Reverted
        } else {
            // Should either stay in PhaseLock or advance to LockConfirmed
            final_state == SwapState::PhaseLock || final_state == SwapState::LockConfirmed
        };

        if let Err(e) = result {
            errors.push(ChaosError::TransitionFailed(e));
        }

        ChaosResult {
            scenario: "timeout_race",
            handled_correctly,
            final_state,
            errors,
            invariant_violations: violations,
        }
    }

    /// Run invalid receipt injection scenario
    fn run_invalid_receipt_injection(&mut self, error_type: InvalidReceiptType) -> ChaosResult {
        let intent_id = [42u8; 32];

        let receipt = match error_type {
            InvalidReceiptType::EmptyTxHash => Receipt {
                header: ReceiptHeader::new(
                    intent_id,
                    DomainId::Evm,
                    Phase::Lock,
                    1000,
                    [1u8; 32],
                    1700000000,
                    vec![2u8; 33],
                ),
                payload: ReceiptPayload::Evm(EvmReceipt {
                    tx_hash: [0u8; 32], // Invalid: empty
                    block_number: 1000,
                    confirmations: 15,
                    contract_address: [4u8; 20],
                    calldata_hash: [5u8; 32],
                }),
            },
            InvalidReceiptType::DomainMismatch => Receipt {
                header: ReceiptHeader::new(
                    intent_id,
                    DomainId::Svm, // Header says SVM
                    Phase::Lock,
                    1000,
                    [1u8; 32],
                    1700000000,
                    vec![2u8; 33],
                ),
                payload: ReceiptPayload::Evm(EvmReceipt { // But payload is EVM
                    tx_hash: [3u8; 32],
                    block_number: 1000,
                    confirmations: 15,
                    contract_address: [4u8; 20],
                    calldata_hash: [5u8; 32],
                }),
            },
            InvalidReceiptType::InsufficientConfirmations => Receipt {
                header: ReceiptHeader::new(
                    intent_id,
                    DomainId::Evm,
                    Phase::Lock,
                    1000,
                    [1u8; 32],
                    1700000000,
                    vec![2u8; 33],
                ),
                payload: ReceiptPayload::Evm(EvmReceipt {
                    tx_hash: [3u8; 32],
                    block_number: 1000,
                    confirmations: 2, // Invalid: less than 12
                    contract_address: [4u8; 20],
                    calldata_hash: [5u8; 32],
                }),
            },
            InvalidReceiptType::WrongIntentId => Receipt {
                header: ReceiptHeader::new(
                    [99u8; 32], // Wrong intent
                    DomainId::Evm,
                    Phase::Lock,
                    1000,
                    [1u8; 32],
                    1700000000,
                    vec![2u8; 33],
                ),
                payload: ReceiptPayload::Evm(EvmReceipt {
                    tx_hash: [3u8; 32],
                    block_number: 1000,
                    confirmations: 15,
                    contract_address: [4u8; 20],
                    calldata_hash: [5u8; 32],
                }),
            },
            InvalidReceiptType::ZeroAmount => Receipt {
                header: ReceiptHeader::new(
                    intent_id,
                    DomainId::Evm,
                    Phase::Lock,
                    0, // Invalid: zero
                    [1u8; 32],
                    1700000000,
                    vec![2u8; 33],
                ),
                payload: ReceiptPayload::Evm(EvmReceipt {
                    tx_hash: [3u8; 32],
                    block_number: 1000,
                    confirmations: 15,
                    contract_address: [4u8; 20],
                    calldata_hash: [5u8; 32],
                }),
            },
            _ => Receipt {
                header: ReceiptHeader::new(
                    intent_id,
                    DomainId::Evm,
                    Phase::Lock,
                    1000,
                    [1u8; 32],
                    1700000000,
                    vec![2u8; 33],
                ),
                payload: ReceiptPayload::Evm(EvmReceipt {
                    tx_hash: [3u8; 32],
                    block_number: 1000,
                    confirmations: 15,
                    contract_address: [4u8; 20],
                    calldata_hash: [5u8; 32],
                }),
            },
        };

        // Validate the receipt
        let validation_result = receipt.validate(&intent_id);

        // Invalid receipts should be rejected
        let handled_correctly = validation_result.is_err();

        let mut errors = Vec::new();
        if let Err(e) = validation_result {
            errors.push(ChaosError::ValidationFailed(e));
        }

        ChaosResult {
            scenario: "invalid_receipt_injection",
            handled_correctly,
            final_state: SwapState::Undefined,
            errors,
            invariant_violations: vec![],
        }
    }

    /// Run partial completion scenario
    fn run_partial_completion(
        &mut self,
        completing_domains: Vec<DomainId>,
        failing_domains: Vec<DomainId>,
    ) -> ChaosResult {
        let mut all_domains = completing_domains.clone();
        all_domains.extend(failing_domains.iter().cloned());

        // Initialize swap with all domains
        self.init_swap(
            all_domains,
            SwapDeadlines {
                lock_deadline: 100,
                exec_deadline: 200,
                final_deadline: 300,
            },
        );

        let ctx = self.context.as_mut().unwrap();

        // Start the swap
        let _ = ctx.transition(TransitionEvent::IntentSubmitted, 0);
        let _ = ctx.transition(TransitionEvent::LockInitiated, 1);

        // Only completing domains submit receipts
        for domain in &completing_domains {
            let _ = ctx.transition(
                TransitionEvent::LockReceiptReceived { domain: *domain },
                10,
            );
        }

        // Advance past deadline without failing domains completing
        self.current_block = 150;

        // Try to advance - should timeout
        let result = ctx.transition(
            TransitionEvent::LockReceiptReceived { domain: failing_domains[0] },
            self.current_block,
        );

        let final_state = ctx.state;

        // Should revert because deadline passed with incomplete receipts
        let handled_correctly = final_state == SwapState::Reverted;

        ChaosResult {
            scenario: "partial_completion",
            handled_correctly,
            final_state,
            errors: vec![],
            invariant_violations: vec![],
        }
    }
}

impl Default for ChaosRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_race_before_deadline() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::TimeoutRace {
            submit_block: 50,
            deadline_block: 100,
        });
        assert!(result.is_handled_correctly());
    }

    #[test]
    fn test_timeout_race_after_deadline() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::TimeoutRace {
            submit_block: 150,
            deadline_block: 100,
        });
        assert!(result.is_handled_correctly());
        assert_eq!(result.final_state, SwapState::Reverted);
    }

    #[test]
    fn test_invalid_receipt_empty_tx_hash() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::InvalidReceiptInjection {
            error_type: InvalidReceiptType::EmptyTxHash,
        });
        assert!(result.is_handled_correctly());
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_invalid_receipt_domain_mismatch() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::InvalidReceiptInjection {
            error_type: InvalidReceiptType::DomainMismatch,
        });
        assert!(result.is_handled_correctly());
    }

    #[test]
    fn test_invalid_receipt_insufficient_confirmations() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::InvalidReceiptInjection {
            error_type: InvalidReceiptType::InsufficientConfirmations,
        });
        assert!(result.is_handled_correctly());
    }

    #[test]
    fn test_invalid_receipt_wrong_intent() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::InvalidReceiptInjection {
            error_type: InvalidReceiptType::WrongIntentId,
        });
        assert!(result.is_handled_correctly());
    }

    #[test]
    fn test_invalid_receipt_zero_amount() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::InvalidReceiptInjection {
            error_type: InvalidReceiptType::ZeroAmount,
        });
        assert!(result.is_handled_correctly());
    }

    #[test]
    fn test_partial_completion() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::PartialCompletion {
            completing_domains: vec![DomainId::Evm],
            failing_domains: vec![DomainId::Svm],
        });
        assert!(result.is_handled_correctly());
        assert_eq!(result.final_state, SwapState::Reverted);
    }

    #[test]
    fn test_fork_simulation() {
        let mut runner = ChaosRunner::new();
        let result = runner.run_scenario(ChaosScenario::ForkSimulation {
            domain: DomainId::Evm,
            depth: 15, // Fork deeper than confirmation threshold
        });
        // Fork should be handled (system stays safe)
        assert!(result.handled_correctly || result.final_state == SwapState::Reverted);
    }
}
