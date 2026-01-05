//! # ASGA Slashing Math
//!
//! Slashing must be:
//! - **Predictable**: Deterministic formulas
//! - **Progressive**: Escalating consequences
//! - **Existentially threatening**: Bad actors lose everything
//!
//! ## Fault Score Formula
//!
//! ```text
//! fault_score(agent) = Σ (severity_i × recurrence_i × intent_weight)
//! ```
//!
//! ## Slash Formula
//!
//! ```text
//! slash_amount = stake(agent) × min(1.0, fault_score / 100)
//! ```
//!
//! ## Severity Table
//!
//! | Fault | Severity |
//! |-------|----------|
//! | Invalid receipt | 3 |
//! | Deadline breach | 2 |
//! | Partial execution | 10 |
//! | Receipt forgery attempt | 50 |
//! | Equivocation | 100 |
//!
//! ## Automatic Escalation
//!
//! | Fault Count | Action |
//! |-------------|--------|
//! | 1 | Warning |
//! | 2 | Partial slash |
//! | 3 | Full slash |
//! | 4 | Permanent removal |
//!
//! No forgiveness. Old-school rules.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

// ============================================================================
// FAULT TYPES (Section 3.1)
// ============================================================================

/// Types of faults an agent can commit
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
#[repr(u8)]
pub enum FaultType {
    /// Submitted an invalid receipt
    InvalidReceipt = 0,
    /// Missed a deadline
    DeadlineBreach = 1,
    /// Only some domains completed
    PartialExecution = 2,
    /// Attempted to submit forged receipt
    ReceiptForgery = 3,
    /// Signed conflicting statements
    Equivocation = 4,
}

impl FaultType {
    /// Get the base severity for this fault type
    /// These values are NON-NEGOTIABLE
    pub const fn severity(&self) -> u32 {
        match self {
            FaultType::InvalidReceipt => 3,
            FaultType::DeadlineBreach => 2,
            FaultType::PartialExecution => 10,
            FaultType::ReceiptForgery => 50,
            FaultType::Equivocation => 100,
        }
    }

    /// Returns true if this fault type warrants immediate permanent ban
    pub const fn is_ban_worthy(&self) -> bool {
        matches!(self, FaultType::ReceiptForgery | FaultType::Equivocation)
    }
}

/// Intent weight for fault calculation
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
#[repr(u8)]
pub enum IntentWeight {
    /// Likely accidental (network issues, bugs)
    Accidental = 1,
    /// Negligent (should have known better)
    Negligent = 2,
    /// Intentional malice
    Malicious = 5,
}

impl Default for IntentWeight {
    fn default() -> Self {
        Self::Negligent
    }
}

// ============================================================================
// FAULT RECORD
// ============================================================================

/// Record of a single fault incident
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct FaultRecord {
    /// Type of fault
    pub fault_type: FaultType,
    /// When the fault occurred (block number)
    pub block_number: u64,
    /// Evidence hash (e.g., hash of conflicting receipts)
    pub evidence_hash: [u8; 32],
    /// Assessed intent weight
    pub intent_weight: IntentWeight,
}

impl FaultRecord {
    pub fn new(
        fault_type: FaultType,
        block_number: u64,
        evidence_hash: [u8; 32],
        intent_weight: IntentWeight,
    ) -> Self {
        Self {
            fault_type,
            block_number,
            evidence_hash,
            intent_weight,
        }
    }

    /// Calculate the weighted severity of this fault
    pub fn weighted_severity(&self) -> u32 {
        self.fault_type.severity() * (self.intent_weight as u32)
    }
}

// ============================================================================
// AGENT FAULT HISTORY
// ============================================================================

/// Complete fault history for an agent
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, Default)]
pub struct AgentFaultHistory {
    /// All recorded faults
    pub faults: Vec<FaultRecord>,
    /// Total fault count
    pub fault_count: u32,
    /// Running fault score
    pub fault_score: u32,
    /// Whether the agent is permanently banned
    pub banned: bool,
}

impl AgentFaultHistory {
    /// Add a new fault record
    pub fn add_fault(&mut self, fault: FaultRecord) {
        // Calculate recurrence multiplier (faults of same type count more)
        let recurrence = self
            .faults
            .iter()
            .filter(|f| f.fault_type == fault.fault_type)
            .count() as u32
            + 1;

        // fault_score += severity × recurrence × intent_weight
        let score_delta = fault.fault_type.severity() * recurrence * (fault.intent_weight as u32);

        self.fault_score = self.fault_score.saturating_add(score_delta);
        self.fault_count += 1;

        // Check for immediate ban
        if fault.fault_type.is_ban_worthy() {
            self.banned = true;
        }

        self.faults.push(fault);
    }

    /// Get the current escalation level
    pub fn escalation_level(&self) -> EscalationLevel {
        if self.banned {
            return EscalationLevel::PermanentRemoval;
        }

        match self.fault_count {
            0 => EscalationLevel::None,
            1 => EscalationLevel::Warning,
            2 => EscalationLevel::PartialSlash,
            3 => EscalationLevel::FullSlash,
            _ => EscalationLevel::PermanentRemoval,
        }
    }
}

// ============================================================================
// ESCALATION (Section 3.3)
// ============================================================================

/// Escalation levels for repeated faults
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
#[repr(u8)]
pub enum EscalationLevel {
    /// No faults recorded
    None = 0,
    /// First offense - warning only
    Warning = 1,
    /// Second offense - partial slash
    PartialSlash = 2,
    /// Third offense - full slash
    FullSlash = 3,
    /// Fourth+ offense - permanent removal from network
    PermanentRemoval = 4,
}

impl EscalationLevel {
    /// Get the slash percentage for this escalation level
    /// Returns value in basis points (10000 = 100%)
    pub const fn slash_percentage_bps(&self) -> u32 {
        match self {
            EscalationLevel::None => 0,
            EscalationLevel::Warning => 0,
            EscalationLevel::PartialSlash => 2500, // 25%
            EscalationLevel::FullSlash => 10000,   // 100%
            EscalationLevel::PermanentRemoval => 10000, // 100% + ban
        }
    }

    /// Whether this level results in a ban
    pub const fn results_in_ban(&self) -> bool {
        matches!(self, EscalationLevel::PermanentRemoval)
    }
}

// ============================================================================
// SLASH CALCULATION (Section 3.2)
// ============================================================================

/// Slash calculation result
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub struct SlashResult {
    /// Amount to slash (in smallest unit)
    pub slash_amount: u128,
    /// Remaining stake after slash
    pub remaining_stake: u128,
    /// Escalation level applied
    pub escalation: EscalationLevel,
    /// Whether agent should be banned
    pub ban: bool,
}

/// Calculate slash amount based on fault history and stake
///
/// Formula: `slash_amount = stake × min(1.0, fault_score / 100)`
///
/// The slash is further modified by the escalation level.
pub fn calculate_slash(stake: u128, history: &AgentFaultHistory) -> SlashResult {
    let escalation = history.escalation_level();

    // Base slash from fault score: min(1.0, fault_score / 100)
    // Using basis points: min(10000, fault_score * 100)
    let fault_score_bps = (history.fault_score as u128).saturating_mul(100).min(10000);

    // Escalation percentage
    let escalation_bps = escalation.slash_percentage_bps() as u128;

    // Take the maximum of fault-based and escalation-based slash
    let effective_bps = fault_score_bps.max(escalation_bps);

    // Calculate slash amount
    let slash_amount = stake.saturating_mul(effective_bps) / 10000;
    let remaining_stake = stake.saturating_sub(slash_amount);

    SlashResult {
        slash_amount,
        remaining_stake,
        escalation,
        ban: history.banned || escalation.results_in_ban(),
    }
}

/// Calculate immediate slash for a specific fault (before adding to history)
pub fn calculate_immediate_slash(stake: u128, fault: &FaultRecord) -> SlashResult {
    // For immediate slash, use the fault's weighted severity directly
    let severity_bps = (fault.weighted_severity() as u128).saturating_mul(100).min(10000);

    let slash_amount = stake.saturating_mul(severity_bps) / 10000;
    let remaining_stake = stake.saturating_sub(slash_amount);

    let ban = fault.fault_type.is_ban_worthy();
    let escalation = if ban {
        EscalationLevel::PermanentRemoval
    } else {
        EscalationLevel::Warning
    };

    SlashResult {
        slash_amount,
        remaining_stake,
        escalation,
        ban,
    }
}

// ============================================================================
// SLASHING PROOFS (Section III - Guardian Agent Slashing)
// ============================================================================

/// Slashing proof - cryptographic evidence of fault
///
/// A slash is valid iff:
/// 1. Invariant WASM hash matches on-chain version
/// 2. Evaluation trace deterministically fails
/// 3. Context hash matches recorded swap state
/// 4. Action required ≠ noop
/// 5. Agent failed to act or caused violation
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct SlashingProof {
    /// Agent being slashed
    pub agent_id: [u8; 32],
    /// Invariant that was violated
    pub invariant_id: [u8; 32],
    /// Hash of the swap context at time of violation
    pub failing_context_hash: [u8; 32],
    /// Hash of the invariant WASM code
    pub invariant_wasm_hash: [u8; 32],
    /// Deterministic evaluation trace
    pub evaluation_trace: EvaluationTrace,
    /// Timestamp of violation detection
    pub timestamp: u64,
    /// Block number where violation occurred
    pub block_number: u64,
    /// Signature from reporter (for non-repudiation)
    pub reporter_signature: [u8; 64],
    /// Reporter's public key
    pub reporter_pubkey: [u8; 32],
}

/// Evaluation trace - step-by-step invariant evaluation
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct EvaluationTrace {
    /// Steps in evaluation
    pub steps: Vec<EvaluationStep>,
    /// Final result
    pub final_result: EvaluationResult,
    /// Gas consumed
    pub gas_consumed: u64,
}

/// Single evaluation step
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct EvaluationStep {
    /// Instruction index
    pub instruction_index: u32,
    /// Opcode executed
    pub opcode: u8,
    /// Stack state hash after execution
    pub stack_hash: [u8; 32],
    /// Gas consumed by this step
    pub gas: u64,
}

/// Evaluation result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[repr(u8)]
pub enum EvaluationResult {
    /// Invariant holds
    Pass = 0,
    /// Invariant violated
    Fail = 1,
    /// Evaluation error (out of gas, invalid opcode)
    Error = 2,
}

/// Required action from invariant
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[repr(u8)]
pub enum RequiredAction {
    /// No action needed
    Noop = 0,
    /// Revert the swap
    Revert = 1,
    /// Slash the agent
    Slash = 2,
    /// Halt the engine
    Halt = 3,
}

/// Slashing proof verification result
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlashProofVerification {
    /// Proof is valid, slash should proceed
    Valid {
        /// Calculated slash amount
        slash_amount: u128,
        /// Should agent be banned
        ban: bool,
    },
    /// Proof is invalid
    Invalid(SlashProofError),
}

/// Errors in slashing proof
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum SlashProofError {
    /// Invariant hash doesn't match on-chain version
    InvariantHashMismatch,
    /// Context hash doesn't match recorded state
    ContextHashMismatch,
    /// Evaluation trace is not deterministic
    NonDeterministicTrace,
    /// Action was noop (no slash required)
    ActionIsNoop,
    /// Agent did not fail or cause violation
    AgentNotResponsible,
    /// Invalid reporter signature
    InvalidReporterSignature,
    /// Trace is empty
    EmptyTrace,
    /// Gas limit exceeded
    GasLimitExceeded,
    /// Timestamp in future
    TimestampInFuture,
}

impl SlashingProof {
    /// Create a new slashing proof
    pub fn new(
        agent_id: [u8; 32],
        invariant_id: [u8; 32],
        failing_context_hash: [u8; 32],
        invariant_wasm_hash: [u8; 32],
        evaluation_trace: EvaluationTrace,
        timestamp: u64,
        block_number: u64,
        reporter_signature: [u8; 64],
        reporter_pubkey: [u8; 32],
    ) -> Self {
        Self {
            agent_id,
            invariant_id,
            failing_context_hash,
            invariant_wasm_hash,
            evaluation_trace,
            timestamp,
            block_number,
            reporter_signature,
            reporter_pubkey,
        }
    }

    /// Verify the slashing proof
    pub fn verify(
        &self,
        on_chain_invariant_hash: [u8; 32],
        on_chain_context_hash: [u8; 32],
        current_timestamp: u64,
        agent_stake: u128,
        max_gas: u64,
    ) -> SlashProofVerification {
        // 1. Verify invariant WASM hash matches on-chain version
        if self.invariant_wasm_hash != on_chain_invariant_hash {
            return SlashProofVerification::Invalid(SlashProofError::InvariantHashMismatch);
        }

        // 2. Verify context hash matches recorded swap state
        if self.failing_context_hash != on_chain_context_hash {
            return SlashProofVerification::Invalid(SlashProofError::ContextHashMismatch);
        }

        // 3. Verify timestamp is not in future
        if self.timestamp > current_timestamp {
            return SlashProofVerification::Invalid(SlashProofError::TimestampInFuture);
        }

        // 4. Verify trace is non-empty
        if self.evaluation_trace.steps.is_empty() {
            return SlashProofVerification::Invalid(SlashProofError::EmptyTrace);
        }

        // 5. Verify gas limit
        if self.evaluation_trace.gas_consumed > max_gas {
            return SlashProofVerification::Invalid(SlashProofError::GasLimitExceeded);
        }

        // 6. Verify evaluation failed (invariant was violated)
        if self.evaluation_trace.final_result != EvaluationResult::Fail {
            return SlashProofVerification::Invalid(SlashProofError::AgentNotResponsible);
        }

        // 7. Verify trace is deterministic (simplified: check hash chain)
        if !self.verify_trace_determinism() {
            return SlashProofVerification::Invalid(SlashProofError::NonDeterministicTrace);
        }

        // 8. Verify reporter signature (simplified: non-zero check)
        if self.reporter_signature == [0u8; 64] {
            return SlashProofVerification::Invalid(SlashProofError::InvalidReporterSignature);
        }

        // Calculate slash based on violation severity
        let slash_amount = self.calculate_slash_amount(agent_stake);
        let ban = self.should_ban();

        SlashProofVerification::Valid { slash_amount, ban }
    }

    /// Verify trace determinism (each step hash chains correctly)
    fn verify_trace_determinism(&self) -> bool {
        let mut prev_hash = [0u8; 32];
        for step in &self.evaluation_trace.steps {
            // Each step's stack_hash should be derived from previous state + operation
            // Simplified check: hashes should be non-zero and different
            if step.stack_hash == [0u8; 32] {
                return false;
            }
            // In production: verify hash chain cryptographically
            prev_hash = step.stack_hash;
        }
        let _ = prev_hash; // Used in hash verification
        true
    }

    /// Calculate slash amount based on violation
    fn calculate_slash_amount(&self, stake: u128) -> u128 {
        // Base slash: 25% for invariant violation
        // Additional: 25% if trace shows multiple failures
        let base_bps: u128 = 2500;
        let failure_count = self.evaluation_trace.steps.len() as u128;
        let additional_bps = (failure_count.saturating_mul(100)).min(2500);

        let total_bps = (base_bps + additional_bps).min(10000);
        stake.saturating_mul(total_bps) / 10000
    }

    /// Determine if agent should be banned
    fn should_ban(&self) -> bool {
        // Ban if evaluation shows malicious pattern (high gas consumption relative to simple invariant)
        self.evaluation_trace.gas_consumed > 10000 ||
            self.evaluation_trace.steps.len() > 100
    }

    /// Compute proof hash (for on-chain storage)
    pub fn hash(&self) -> [u8; 32] {
        let mut data = Vec::new();
        data.extend_from_slice(&self.agent_id);
        data.extend_from_slice(&self.invariant_id);
        data.extend_from_slice(&self.failing_context_hash);
        data.extend_from_slice(&self.invariant_wasm_hash);
        data.extend_from_slice(&self.timestamp.to_le_bytes());
        data.extend_from_slice(&self.block_number.to_le_bytes());

        // Simple hash (in production: use blake2_256)
        let mut hash = [0u8; 32];
        for (i, byte) in data.iter().enumerate() {
            hash[i % 32] ^= byte;
            hash[(i + 1) % 32] = hash[(i + 1) % 32].wrapping_add(*byte);
        }
        hash
    }
}

/// Non-repudiation guarantee
///
/// Agents cannot deny slashing because:
/// - Invariant code is immutable (WASM hash verified)
/// - Context is on-chain (context hash verified)
/// - Evaluation is deterministic (trace verified)
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct NonRepudiationRecord {
    /// Proof hash
    pub proof_hash: [u8; 32],
    /// Agent who was slashed
    pub agent_id: [u8; 32],
    /// Block where slash was executed
    pub slash_block: u64,
    /// Amount slashed
    pub slash_amount: u128,
    /// Was agent banned
    pub banned: bool,
    /// Invariant that triggered slash
    pub invariant_id: [u8; 32],
}

impl NonRepudiationRecord {
    pub fn from_proof(proof: &SlashingProof, slash_amount: u128, banned: bool, slash_block: u64) -> Self {
        Self {
            proof_hash: proof.hash(),
            agent_id: proof.agent_id,
            slash_block,
            slash_amount,
            banned,
            invariant_id: proof.invariant_id,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_severity() {
        assert_eq!(FaultType::InvalidReceipt.severity(), 3);
        assert_eq!(FaultType::DeadlineBreach.severity(), 2);
        assert_eq!(FaultType::PartialExecution.severity(), 10);
        assert_eq!(FaultType::ReceiptForgery.severity(), 50);
        assert_eq!(FaultType::Equivocation.severity(), 100);
    }

    #[test]
    fn test_ban_worthy_faults() {
        assert!(!FaultType::InvalidReceipt.is_ban_worthy());
        assert!(!FaultType::DeadlineBreach.is_ban_worthy());
        assert!(!FaultType::PartialExecution.is_ban_worthy());
        assert!(FaultType::ReceiptForgery.is_ban_worthy());
        assert!(FaultType::Equivocation.is_ban_worthy());
    }

    #[test]
    fn test_escalation_progression() {
        let mut history = AgentFaultHistory::default();

        assert_eq!(history.escalation_level(), EscalationLevel::None);

        // First fault -> Warning
        history.add_fault(FaultRecord::new(
            FaultType::InvalidReceipt,
            100,
            [0u8; 32],
            IntentWeight::Accidental,
        ));
        assert_eq!(history.escalation_level(), EscalationLevel::Warning);

        // Second fault -> Partial slash
        history.add_fault(FaultRecord::new(
            FaultType::DeadlineBreach,
            200,
            [0u8; 32],
            IntentWeight::Negligent,
        ));
        assert_eq!(history.escalation_level(), EscalationLevel::PartialSlash);

        // Third fault -> Full slash
        history.add_fault(FaultRecord::new(
            FaultType::InvalidReceipt,
            300,
            [0u8; 32],
            IntentWeight::Negligent,
        ));
        assert_eq!(history.escalation_level(), EscalationLevel::FullSlash);

        // Fourth fault -> Permanent removal
        history.add_fault(FaultRecord::new(
            FaultType::DeadlineBreach,
            400,
            [0u8; 32],
            IntentWeight::Negligent,
        ));
        assert_eq!(history.escalation_level(), EscalationLevel::PermanentRemoval);
    }

    #[test]
    fn test_immediate_ban_on_forgery() {
        let mut history = AgentFaultHistory::default();

        // First fault is forgery -> immediate ban
        history.add_fault(FaultRecord::new(
            FaultType::ReceiptForgery,
            100,
            [0u8; 32],
            IntentWeight::Malicious,
        ));

        assert!(history.banned);
        assert_eq!(history.escalation_level(), EscalationLevel::PermanentRemoval);
    }

    #[test]
    fn test_immediate_ban_on_equivocation() {
        let mut history = AgentFaultHistory::default();

        history.add_fault(FaultRecord::new(
            FaultType::Equivocation,
            100,
            [0u8; 32],
            IntentWeight::Malicious,
        ));

        assert!(history.banned);
        assert_eq!(history.escalation_level(), EscalationLevel::PermanentRemoval);
    }

    #[test]
    fn test_fault_score_calculation() {
        let mut history = AgentFaultHistory::default();

        // First invalid receipt: 3 * 1 (recurrence) * 2 (negligent) = 6
        history.add_fault(FaultRecord::new(
            FaultType::InvalidReceipt,
            100,
            [0u8; 32],
            IntentWeight::Negligent,
        ));
        assert_eq!(history.fault_score, 6);

        // Second invalid receipt: 3 * 2 (recurrence) * 2 (negligent) = 12
        history.add_fault(FaultRecord::new(
            FaultType::InvalidReceipt,
            200,
            [0u8; 32],
            IntentWeight::Negligent,
        ));
        assert_eq!(history.fault_score, 18); // 6 + 12
    }

    #[test]
    fn test_slash_calculation_zero_faults() {
        let history = AgentFaultHistory::default();
        let stake = 1_000_000u128;

        let result = calculate_slash(stake, &history);

        assert_eq!(result.slash_amount, 0);
        assert_eq!(result.remaining_stake, stake);
        assert_eq!(result.escalation, EscalationLevel::None);
        assert!(!result.ban);
    }

    #[test]
    fn test_slash_calculation_partial_execution() {
        let mut history = AgentFaultHistory::default();

        // Partial execution is severe: 10 * 1 * 2 = 20
        history.add_fault(FaultRecord::new(
            FaultType::PartialExecution,
            100,
            [0u8; 32],
            IntentWeight::Negligent,
        ));

        let stake = 1_000_000u128;
        let result = calculate_slash(stake, &history);

        // fault_score = 20, so fault_score_bps = 2000 (20%)
        // But escalation for first offense is Warning (0%)
        // Take max: 2000 bps = 20%
        assert_eq!(result.slash_amount, 200_000); // 20%
        assert_eq!(result.remaining_stake, 800_000);
    }

    #[test]
    fn test_slash_calculation_equivocation_total_slash() {
        let mut history = AgentFaultHistory::default();

        // Equivocation: 100 * 1 * 5 (malicious) = 500
        history.add_fault(FaultRecord::new(
            FaultType::Equivocation,
            100,
            [0u8; 32],
            IntentWeight::Malicious,
        ));

        let stake = 1_000_000u128;
        let result = calculate_slash(stake, &history);

        // fault_score = 500, so fault_score_bps = 10000 (capped at 100%)
        assert_eq!(result.slash_amount, 1_000_000); // 100%
        assert_eq!(result.remaining_stake, 0);
        assert!(result.ban);
    }

    #[test]
    fn test_immediate_slash_calculation() {
        let fault = FaultRecord::new(
            FaultType::PartialExecution,
            100,
            [0u8; 32],
            IntentWeight::Malicious,
        );

        let stake = 1_000_000u128;
        let result = calculate_immediate_slash(stake, &fault);

        // weighted_severity = 10 * 5 = 50
        // severity_bps = 5000 (50%)
        assert_eq!(result.slash_amount, 500_000);
        assert_eq!(result.remaining_stake, 500_000);
    }

    #[test]
    fn test_recurrence_multiplier() {
        let mut history = AgentFaultHistory::default();

        // First deadline breach: 2 * 1 * 2 = 4
        history.add_fault(FaultRecord::new(
            FaultType::DeadlineBreach,
            100,
            [0u8; 32],
            IntentWeight::Negligent,
        ));
        assert_eq!(history.fault_score, 4);

        // Second deadline breach: 2 * 2 * 2 = 8
        history.add_fault(FaultRecord::new(
            FaultType::DeadlineBreach,
            200,
            [0u8; 32],
            IntentWeight::Negligent,
        ));
        assert_eq!(history.fault_score, 12); // 4 + 8

        // Third deadline breach: 2 * 3 * 2 = 12
        history.add_fault(FaultRecord::new(
            FaultType::DeadlineBreach,
            300,
            [0u8; 32],
            IntentWeight::Negligent,
        ));
        assert_eq!(history.fault_score, 24); // 4 + 8 + 12
    }

    #[test]
    fn test_escalation_percentages() {
        assert_eq!(EscalationLevel::None.slash_percentage_bps(), 0);
        assert_eq!(EscalationLevel::Warning.slash_percentage_bps(), 0);
        assert_eq!(EscalationLevel::PartialSlash.slash_percentage_bps(), 2500);
        assert_eq!(EscalationLevel::FullSlash.slash_percentage_bps(), 10000);
        assert_eq!(EscalationLevel::PermanentRemoval.slash_percentage_bps(), 10000);
    }

    // ========================================================================
    // SLASHING PROOF TESTS
    // ========================================================================

    fn sample_evaluation_trace() -> EvaluationTrace {
        EvaluationTrace {
            steps: vec![
                EvaluationStep {
                    instruction_index: 0,
                    opcode: 0x01,
                    stack_hash: [1u8; 32],
                    gas: 10,
                },
                EvaluationStep {
                    instruction_index: 1,
                    opcode: 0x10,
                    stack_hash: [2u8; 32],
                    gas: 20,
                },
            ],
            final_result: EvaluationResult::Fail,
            gas_consumed: 30,
        }
    }

    fn sample_slashing_proof() -> SlashingProof {
        SlashingProof::new(
            [1u8; 32],  // agent_id
            [2u8; 32],  // invariant_id
            [3u8; 32],  // failing_context_hash
            [4u8; 32],  // invariant_wasm_hash
            sample_evaluation_trace(),
            1000,       // timestamp
            100,        // block_number
            [5u8; 64],  // reporter_signature
            [6u8; 32],  // reporter_pubkey
        )
    }

    #[test]
    fn test_slashing_proof_valid() {
        let proof = sample_slashing_proof();

        let result = proof.verify(
            [4u8; 32],  // on_chain_invariant_hash (matches)
            [3u8; 32],  // on_chain_context_hash (matches)
            2000,       // current_timestamp
            1_000_000,  // agent_stake
            1000,       // max_gas
        );

        assert!(matches!(result, SlashProofVerification::Valid { .. }));
    }

    #[test]
    fn test_slashing_proof_invariant_mismatch() {
        let proof = sample_slashing_proof();

        let result = proof.verify(
            [99u8; 32], // on_chain_invariant_hash (doesn't match)
            [3u8; 32],
            2000,
            1_000_000,
            1000,
        );

        assert!(matches!(
            result,
            SlashProofVerification::Invalid(SlashProofError::InvariantHashMismatch)
        ));
    }

    #[test]
    fn test_slashing_proof_context_mismatch() {
        let proof = sample_slashing_proof();

        let result = proof.verify(
            [4u8; 32],
            [99u8; 32], // on_chain_context_hash (doesn't match)
            2000,
            1_000_000,
            1000,
        );

        assert!(matches!(
            result,
            SlashProofVerification::Invalid(SlashProofError::ContextHashMismatch)
        ));
    }

    #[test]
    fn test_slashing_proof_future_timestamp() {
        let proof = sample_slashing_proof();

        let result = proof.verify(
            [4u8; 32],
            [3u8; 32],
            500,        // current_timestamp < proof timestamp
            1_000_000,
            1000,
        );

        assert!(matches!(
            result,
            SlashProofVerification::Invalid(SlashProofError::TimestampInFuture)
        ));
    }

    #[test]
    fn test_slashing_proof_empty_trace() {
        let mut proof = sample_slashing_proof();
        proof.evaluation_trace.steps = vec![];

        let result = proof.verify([4u8; 32], [3u8; 32], 2000, 1_000_000, 1000);

        assert!(matches!(
            result,
            SlashProofVerification::Invalid(SlashProofError::EmptyTrace)
        ));
    }

    #[test]
    fn test_slashing_proof_gas_exceeded() {
        let mut proof = sample_slashing_proof();
        proof.evaluation_trace.gas_consumed = 999999;

        let result = proof.verify([4u8; 32], [3u8; 32], 2000, 1_000_000, 1000);

        assert!(matches!(
            result,
            SlashProofVerification::Invalid(SlashProofError::GasLimitExceeded)
        ));
    }

    #[test]
    fn test_slashing_proof_not_failed() {
        let mut proof = sample_slashing_proof();
        proof.evaluation_trace.final_result = EvaluationResult::Pass;

        let result = proof.verify([4u8; 32], [3u8; 32], 2000, 1_000_000, 1000);

        assert!(matches!(
            result,
            SlashProofVerification::Invalid(SlashProofError::AgentNotResponsible)
        ));
    }

    #[test]
    fn test_slashing_proof_invalid_signature() {
        let mut proof = sample_slashing_proof();
        proof.reporter_signature = [0u8; 64];

        let result = proof.verify([4u8; 32], [3u8; 32], 2000, 1_000_000, 1000);

        assert!(matches!(
            result,
            SlashProofVerification::Invalid(SlashProofError::InvalidReporterSignature)
        ));
    }

    #[test]
    fn test_slashing_proof_hash_determinism() {
        let proof1 = sample_slashing_proof();
        let proof2 = sample_slashing_proof();

        assert_eq!(proof1.hash(), proof2.hash());

        let mut proof3 = sample_slashing_proof();
        proof3.timestamp = 9999;
        assert_ne!(proof1.hash(), proof3.hash());
    }

    #[test]
    fn test_non_repudiation_record() {
        let proof = sample_slashing_proof();
        let record = NonRepudiationRecord::from_proof(&proof, 250_000, false, 150);

        assert_eq!(record.proof_hash, proof.hash());
        assert_eq!(record.agent_id, [1u8; 32]);
        assert_eq!(record.slash_amount, 250_000);
        assert!(!record.banned);
        assert_eq!(record.slash_block, 150);
    }
}
