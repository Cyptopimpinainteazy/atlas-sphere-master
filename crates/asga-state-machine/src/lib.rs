//! # ASGA State Machine
//!
//! Formal state machine specification for atomic cross-chain swaps.
//! This is the authoritative model for the ASGA (Atomic Swap Guardian Agent) swap lifecycle.
//!
//! ## State Diagram
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────────┐
//! │                         ASGA STATE MACHINE                                 │
//! │                                                                            │
//! │   ┌─────────┐                                                              │
//! │   │   S0    │─────────────submit_intent()─────────────────────────┐       │
//! │   │Undefined│                                                      │       │
//! │   └─────────┘                                                      ▼       │
//! │                                                              ┌──────────┐  │
//! │                            ┌──────────timeout────────────────│    S1    │  │
//! │                            │                                 │Initialized  │
//! │                            ▼                                 └────┬─────┘  │
//! │                     ┌───────────┐                                  │        │
//! │                     │    S7     │◄───────failure──────┐           │        │
//! │                     │ Reverted  │                     │   submit_lock()    │
//! │                     └───────────┘                     │           │        │
//! │                            ▲                          │           ▼        │
//! │                            │                    ┌──────────┐               │
//! │                        timeout                  │    S2    │               │
//! │                            │                    │ PhaseLock│               │
//! │                     ┌──────┴─────┐              └────┬─────┘               │
//! │                     │    S6      │                   │                     │
//! │                     │ PhaseFinal │◄────┐     all_lock_receipts()          │
//! │                     └──────┬─────┘     │             │                     │
//! │                            │           │             ▼                     │
//! │                   all_final_receipts() │       ┌──────────┐               │
//! │                            │           │       │    S3    │               │
//! │                            ▼           │       │LockConfirm│              │
//! │                     ┌───────────┐      │       └────┬─────┘               │
//! │                     │    S8     │      │            │                     │
//! │                     │ Completed │      │      submit_exec()               │
//! │                     └───────────┘      │            │                     │
//! │                                        │            ▼                     │
//! │   ┌───────────┐                   ┌──────────┐                            │
//! │   │    S9     │◄──slash_agent()───│    S4    │                            │
//! │   │  Slashed  │                   │PhaseExec │                            │
//! │   └───────────┘                   └────┬─────┘                            │
//! │                                        │                                  │
//! │                               all_exec_receipts()                         │
//! │                                        │                                  │
//! │                                        ▼                                  │
//! │                                  ┌──────────┐                             │
//! │                                  │    S5    │──────submit_final()─────────┘
//! │                                  │ExecConfirm                             │
//! │                                  └──────────┘                             │
//! └───────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Invariants
//!
//! 1. **All-or-nothing**: Either all domains complete all phases, or all revert.
//! 2. **Time-bounded execution**: Each phase has a deadline; timeout triggers revert.
//! 3. **Receipt validity**: Only valid, attested receipts can advance state.
//! 4. **Finality safety**: Receipts from non-final blocks are not accepted.
//! 5. **Monotonic progress**: State only advances forward (except for revert).
//!
//! ## Phases
//!
//! - **Lock**: Assets are locked in escrow on each domain
//! - **Exec**: Atomic execution of the swap (release to counterparty)
//! - **Final**: Confirmation of finality across all domains

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

pub use asga_receipts::{DomainId, Phase};

/// Swap state enumeration following the formal state machine.
///
/// State transitions are strictly controlled by the ASGA arbiter.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
#[repr(u8)]
pub enum SwapState {
    /// S0: Undefined/non-existent - initial state before intent submission
    Undefined = 0,
    /// S1: Intent submitted, waiting for lock phase to begin
    Initialized = 1,
    /// S2: Lock phase in progress, waiting for lock receipts from all domains
    PhaseLock = 2,
    /// S3: All lock receipts confirmed, ready to execute
    LockConfirmed = 3,
    /// S4: Execution phase in progress, waiting for exec receipts
    PhaseExec = 4,
    /// S5: All execution receipts confirmed, ready for finalization
    ExecConfirmed = 5,
    /// S6: Finalization phase in progress, waiting for final receipts
    PhaseFinal = 6,
    /// S7: Swap reverted (timeout or failure)
    Reverted = 7,
    /// S8: Swap completed successfully
    Completed = 8,
    /// S9: Agent slashed for misbehavior
    Slashed = 9,
}

impl Default for SwapState {
    fn default() -> Self {
        Self::Undefined
    }
}

impl SwapState {
    /// Returns true if this is a terminal state (no further transitions possible)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Reverted | Self::Slashed)
    }

    /// Returns true if the swap is still in progress (can advance)
    pub fn is_active(&self) -> bool {
        !self.is_terminal() && *self != Self::Undefined
    }

    /// Returns the current phase based on state, if applicable
    pub fn current_phase(&self) -> Option<Phase> {
        match self {
            Self::PhaseLock | Self::LockConfirmed => Some(Phase::Lock),
            Self::PhaseExec | Self::ExecConfirmed => Some(Phase::Exec),
            Self::PhaseFinal => Some(Phase::Final),
            _ => None,
        }
    }
}

/// State transition events that trigger state changes.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum TransitionEvent {
    /// Intent submitted by proposer
    IntentSubmitted,
    /// Lock transaction initiated
    LockInitiated,
    /// Lock receipt received from a domain
    LockReceiptReceived { domain: DomainId },
    /// All lock receipts confirmed
    AllLocksConfirmed,
    /// Execution transaction initiated
    ExecInitiated,
    /// Execution receipt received from a domain
    ExecReceiptReceived { domain: DomainId },
    /// All execution receipts confirmed
    AllExecsConfirmed,
    /// Finalization transaction initiated
    FinalInitiated,
    /// Final receipt received from a domain
    FinalReceiptReceived { domain: DomainId },
    /// All final receipts confirmed
    AllFinalsConfirmed,
    /// Timeout occurred
    Timeout,
    /// Failure detected (invalid receipt, double-spend, etc.)
    Failure { reason: FailureReason },
    /// Slashing evidence submitted
    SlashEvidence,
}

/// Reasons for swap failure.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub enum FailureReason {
    /// Deadline exceeded without completion
    Timeout,
    /// Receipt validation failed
    InvalidReceipt,
    /// Double-spend detected
    DoubleSpend,
    /// Insufficient confirmations
    InsufficientConfirmations,
    /// Domain reported failure
    DomainFailure,
    /// Manual abort by proposer
    Aborted,
    /// Arbiter detected protocol violation
    ProtocolViolation,
}

/// Transition error - returned when a state transition is invalid.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub enum TransitionError {
    /// Current state does not allow this transition
    InvalidTransition,
    /// Missing required receipts
    MissingReceipts,
    /// Deadline has passed
    DeadlineExceeded,
    /// State is already terminal
    AlreadyTerminal,
    /// Unauthorized caller
    Unauthorized,
}

/// Transition result type.
pub type TransitionResult = Result<SwapState, TransitionError>;

/// Swap deadline configuration per phase.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub struct SwapDeadlines {
    /// Block number by which lock phase must complete
    pub lock_deadline: u64,
    /// Block number by which exec phase must complete
    pub exec_deadline: u64,
    /// Block number by which final phase must complete
    pub final_deadline: u64,
}

/// Receipt collection status for a swap.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, Default)]
pub struct ReceiptStatus {
    /// Domains that have submitted lock receipts
    pub lock_receipts: Vec<DomainId>,
    /// Domains that have submitted exec receipts
    pub exec_receipts: Vec<DomainId>,
    /// Domains that have submitted final receipts
    pub final_receipts: Vec<DomainId>,
}

impl ReceiptStatus {
    /// Check if all required domains have submitted receipts for a phase
    pub fn phase_complete(&self, phase: Phase, required_domains: &[DomainId]) -> bool {
        let receipts = match phase {
            Phase::Lock => &self.lock_receipts,
            Phase::Exec => &self.exec_receipts,
            Phase::Final => &self.final_receipts,
        };
        required_domains.iter().all(|d| receipts.contains(d))
    }

    /// Add a receipt for a domain and phase
    pub fn add_receipt(&mut self, phase: Phase, domain: DomainId) {
        let receipts = match phase {
            Phase::Lock => &mut self.lock_receipts,
            Phase::Exec => &mut self.exec_receipts,
            Phase::Final => &mut self.final_receipts,
        };
        if !receipts.contains(&domain) {
            receipts.push(domain);
        }
    }

    /// Check if a receipt already exists
    pub fn has_receipt(&self, phase: Phase, domain: DomainId) -> bool {
        let receipts = match phase {
            Phase::Lock => &self.lock_receipts,
            Phase::Exec => &self.exec_receipts,
            Phase::Final => &self.final_receipts,
        };
        receipts.contains(&domain)
    }
}

/// Complete swap context for state machine transitions.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct SwapContext {
    /// Current state
    pub state: SwapState,
    /// Required domains for this swap
    pub required_domains: Vec<DomainId>,
    /// Receipt collection status
    pub receipts: ReceiptStatus,
    /// Deadlines for each phase
    pub deadlines: SwapDeadlines,
}

impl SwapContext {
    /// Create a new swap context in Initialized state
    pub fn new(required_domains: Vec<DomainId>, deadlines: SwapDeadlines) -> Self {
        Self {
            state: SwapState::Initialized,
            required_domains,
            receipts: ReceiptStatus::default(),
            deadlines,
        }
    }

    /// Apply a transition event and return the new state
    pub fn transition(&mut self, event: TransitionEvent, current_block: u64) -> TransitionResult {
        // Check for timeout first
        if self.state.is_active() {
            if let Some(phase) = self.state.current_phase() {
                let deadline = match phase {
                    Phase::Lock => self.deadlines.lock_deadline,
                    Phase::Exec => self.deadlines.exec_deadline,
                    Phase::Final => self.deadlines.final_deadline,
                };
                if current_block > deadline {
                    self.state = SwapState::Reverted;
                    return Ok(SwapState::Reverted);
                }
            }
        }

        // Check terminal state
        if self.state.is_terminal() {
            return Err(TransitionError::AlreadyTerminal);
        }

        let new_state = match (&self.state, &event) {
            // S0 -> S1: Intent submitted
            (SwapState::Undefined, TransitionEvent::IntentSubmitted) => SwapState::Initialized,

            // S1 -> S2: Lock initiated
            (SwapState::Initialized, TransitionEvent::LockInitiated) => SwapState::PhaseLock,

            // S2: Lock receipt received (stays in S2 until all received)
            (SwapState::PhaseLock, TransitionEvent::LockReceiptReceived { domain }) => {
                self.receipts.add_receipt(Phase::Lock, *domain);
                if self.receipts.phase_complete(Phase::Lock, &self.required_domains) {
                    SwapState::LockConfirmed
                } else {
                    SwapState::PhaseLock
                }
            }

            // S3 -> S4: Exec initiated
            (SwapState::LockConfirmed, TransitionEvent::ExecInitiated) => SwapState::PhaseExec,

            // S4: Exec receipt received (stays in S4 until all received)
            (SwapState::PhaseExec, TransitionEvent::ExecReceiptReceived { domain }) => {
                self.receipts.add_receipt(Phase::Exec, *domain);
                if self.receipts.phase_complete(Phase::Exec, &self.required_domains) {
                    SwapState::ExecConfirmed
                } else {
                    SwapState::PhaseExec
                }
            }

            // S5 -> S6: Final initiated
            (SwapState::ExecConfirmed, TransitionEvent::FinalInitiated) => SwapState::PhaseFinal,

            // S6: Final receipt received (stays in S6 until all received)
            (SwapState::PhaseFinal, TransitionEvent::FinalReceiptReceived { domain }) => {
                self.receipts.add_receipt(Phase::Final, *domain);
                if self.receipts.phase_complete(Phase::Final, &self.required_domains) {
                    SwapState::Completed
                } else {
                    SwapState::PhaseFinal
                }
            }

            // Any active state + Timeout -> Reverted
            (s, TransitionEvent::Timeout) if s.is_active() => SwapState::Reverted,

            // Any active state + Failure -> Reverted
            (s, TransitionEvent::Failure { .. }) if s.is_active() => SwapState::Reverted,

            // Any state + SlashEvidence -> Slashed
            (_, TransitionEvent::SlashEvidence) => SwapState::Slashed,

            // Invalid transition
            _ => return Err(TransitionError::InvalidTransition),
        };

        self.state = new_state;
        Ok(new_state)
    }

    /// Check all ASGA invariants
    pub fn check_invariants(&self, current_block: u64) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();

        // Invariant 1: All-or-nothing
        // (Cannot check fully without all receipts, but can detect partial states)
        if self.state == SwapState::Completed {
            for domain in &self.required_domains {
                if !self.receipts.has_receipt(Phase::Final, *domain) {
                    violations.push(InvariantViolation::AllOrNothing {
                        missing_domain: *domain,
                    });
                }
            }
        }

        // Invariant 2: Time-bounded execution
        if self.state.is_active() {
            if let Some(phase) = self.state.current_phase() {
                let deadline = match phase {
                    Phase::Lock => self.deadlines.lock_deadline,
                    Phase::Exec => self.deadlines.exec_deadline,
                    Phase::Final => self.deadlines.final_deadline,
                };
                if current_block > deadline {
                    violations.push(InvariantViolation::TimeBoundExceeded {
                        phase,
                        deadline,
                        current_block,
                    });
                }
            }
        }

        // Invariant 5: Monotonic progress
        // (Encoded in transition logic - cannot violate without bug)

        violations
    }
}

/// Invariant violation types for monitoring.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum InvariantViolation {
    /// All-or-nothing invariant violated: domain missing receipt
    AllOrNothing { missing_domain: DomainId },
    /// Time-bound invariant violated: deadline exceeded
    TimeBoundExceeded {
        phase: Phase,
        deadline: u64,
        current_block: u64,
    },
    /// Receipt validity invariant violated
    InvalidReceipt { domain: DomainId, reason: ReceiptError },
    /// Finality safety invariant violated
    FinalitySafetyViolation { domain: DomainId },
}

/// Receipt error reasons for invariant violations.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub enum ReceiptError {
    /// Receipt signature invalid
    InvalidSignature,
    /// Receipt format malformed
    MalformedFormat,
    /// Receipt domain mismatch
    DomainMismatch,
    /// Receipt already submitted
    Duplicate,
    /// Receipt from non-final block
    NonFinalBlock,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> SwapContext {
        SwapContext::new(
            vec![DomainId::Evm, DomainId::Svm],
            SwapDeadlines {
                lock_deadline: 100,
                exec_deadline: 200,
                final_deadline: 300,
            },
        )
    }

    #[test]
    fn test_happy_path() {
        let mut ctx = SwapContext {
            state: SwapState::Undefined,
            required_domains: vec![DomainId::Evm, DomainId::Svm],
            receipts: ReceiptStatus::default(),
            deadlines: SwapDeadlines {
                lock_deadline: 100,
                exec_deadline: 200,
                final_deadline: 400,
            },
        };

        // S0 -> S1
        assert_eq!(ctx.transition(TransitionEvent::IntentSubmitted, 0).unwrap(), SwapState::Initialized);

        // S1 -> S2
        assert_eq!(ctx.transition(TransitionEvent::LockInitiated, 1).unwrap(), SwapState::PhaseLock);

        // S2: receive lock receipts
        assert_eq!(
            ctx.transition(TransitionEvent::LockReceiptReceived { domain: DomainId::Evm }, 10).unwrap(),
            SwapState::PhaseLock
        );
        assert_eq!(
            ctx.transition(TransitionEvent::LockReceiptReceived { domain: DomainId::Svm }, 15).unwrap(),
            SwapState::LockConfirmed
        );

        // S3 -> S4
        assert_eq!(ctx.transition(TransitionEvent::ExecInitiated, 20).unwrap(), SwapState::PhaseExec);

        // S4: receive exec receipts
        assert_eq!(
            ctx.transition(TransitionEvent::ExecReceiptReceived { domain: DomainId::Evm }, 110).unwrap(),
            SwapState::PhaseExec
        );
        assert_eq!(
            ctx.transition(TransitionEvent::ExecReceiptReceived { domain: DomainId::Svm }, 115).unwrap(),
            SwapState::ExecConfirmed
        );

        // S5 -> S6
        assert_eq!(ctx.transition(TransitionEvent::FinalInitiated, 150).unwrap(), SwapState::PhaseFinal);

        // S6: receive final receipts
        assert_eq!(
            ctx.transition(TransitionEvent::FinalReceiptReceived { domain: DomainId::Evm }, 250).unwrap(),
            SwapState::PhaseFinal
        );
        assert_eq!(
            ctx.transition(TransitionEvent::FinalReceiptReceived { domain: DomainId::Svm }, 255).unwrap(),
            SwapState::Completed
        );

        assert!(ctx.state.is_terminal());
        assert!(ctx.check_invariants(255).is_empty());
    }

    #[test]
    fn test_timeout_revert() {
        let mut ctx = make_context();

        ctx.state = SwapState::PhaseLock;

        // Deadline exceeded should revert
        let result = ctx.transition(TransitionEvent::LockReceiptReceived { domain: DomainId::Evm }, 150);
        assert_eq!(result.unwrap(), SwapState::Reverted);
    }

    #[test]
    fn test_invalid_transition() {
        let mut ctx = make_context();
        ctx.state = SwapState::Initialized;

        // Cannot receive exec receipt in initialized state
        let result = ctx.transition(
            TransitionEvent::ExecReceiptReceived { domain: DomainId::Evm },
            10,
        );
        assert_eq!(result, Err(TransitionError::InvalidTransition));
    }

    #[test]
    fn test_terminal_state() {
        let mut ctx = make_context();
        ctx.state = SwapState::Completed;

        let result = ctx.transition(TransitionEvent::IntentSubmitted, 0);
        assert_eq!(result, Err(TransitionError::AlreadyTerminal));
    }

    #[test]
    fn test_slash_any_state() {
        let mut ctx = make_context();
        ctx.state = SwapState::PhaseExec;

        let result = ctx.transition(TransitionEvent::SlashEvidence, 50);
        assert_eq!(result.unwrap(), SwapState::Slashed);
    }

    #[test]
    fn test_invariant_check() {
        let mut ctx = make_context();
        ctx.state = SwapState::PhaseLock;

        // Check deadline exceeded invariant
        let violations = ctx.check_invariants(150);
        assert!(!violations.is_empty());
        assert!(matches!(violations[0], InvariantViolation::TimeBoundExceeded { .. }));
    }

    #[test]
    fn test_receipt_dedup() {
        let mut status = ReceiptStatus::default();
        status.add_receipt(Phase::Lock, DomainId::Evm);
        status.add_receipt(Phase::Lock, DomainId::Evm); // duplicate
        assert_eq!(status.lock_receipts.len(), 1);
    }

    #[test]
    fn test_state_properties() {
        assert!(!SwapState::Undefined.is_active());
        assert!(SwapState::Initialized.is_active());
        assert!(SwapState::PhaseLock.is_active());
        assert!(SwapState::Completed.is_terminal());
        assert!(SwapState::Reverted.is_terminal());
        assert!(SwapState::Slashed.is_terminal());

        assert_eq!(SwapState::PhaseLock.current_phase(), Some(Phase::Lock));
        assert_eq!(SwapState::PhaseExec.current_phase(), Some(Phase::Exec));
        assert_eq!(SwapState::PhaseFinal.current_phase(), Some(Phase::Final));
        assert_eq!(SwapState::Completed.current_phase(), None);
    }
}
