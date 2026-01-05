# X3 Invariant Constitution — Canonical Specification

**Version:** 1.0
**Status:** IMMUTABLE
**Date:** 2025-01-16

---

## Preamble

This document defines the constitutional invariants of the X3 atomic swap protocol. These invariants are not suggestions, guidelines, or best practices. They are enforcement rules compiled into the protocol at genesis.

Violation of any constitutional invariant triggers one of:
- **HALT**: System stops until manual recovery
- **SLASH**: Guardian stake reduction (10%–100%)
- **ESCALATE**: Bounded dispute/recovery path
- **DENY**: Operation rejected

No governance mechanism can override a constitutional invariant during execution.

---

## Section V: Cross-Domain Synchronization

### V.1 Domain Clock Monotonicity

```
invariant CrossDomainClockMonotonic:
  forall D, t1, t2:
    t2 > t1 => clock(D, t2) >= clock(D, t1)
  else:
    HALT("Clock regression", D)
```

**Enforcement:** DomainClock adapter validates monotonicity. Any regression halts the affected domain's participation.

### V.2 Swap Phase State Machine

```
state_machine SwapPhase:
  INIT -> LOCK -> PROVE -> FINALIZE -> COMPLETE
  LOCK -> TIMEOUT
  PROVE -> DISPUTE
  FINALIZE -> FORCE_CLOSE
```

#### V.2.a Phase Exclusivity

```
invariant PhaseExclusivity:
  exactly_one(SwapPhase)
  else HALT("Illegal phase overlap")
```

#### V.2.b Forward Progress

```
invariant ForwardProgress:
  no phase regression unless Recovery (Section IX)
```

### V.3 Quorum Requirements

```
const QuorumThreshold = ceil(2/3 * ActiveGuardians)
```

#### V.3.a Quorum Safety

```
invariant QuorumSafety:
  transition.requires(quorum >= QuorumThreshold)
  else SLASH(100%)
```

**Rationale:** 2/3 supermajority prevents split-brain finality. Quorum failure is terminal violation.

---

## Section VI: Timeout & Liveness Guarantees

### VI.1 HTLC Bounded Duration

```
struct HTLCWindow:
  min_blocks: u32     // Earliest claim
  max_blocks: u32     // Latest claim before refund
  grace_blocks: u32   // Extension window (Section VI.3)
```

#### VI.1.a Bounded HTLC

```
invariant BoundedHTLC:
  now <= LockTime + DeltaDomain
  else ESCALATE(REFUND_ELIGIBLE)
```

### VI.2 Escalation Path

```
enum EscalationPath:
  TIMEOUT    -> REFUND_ELIGIBLE
  LIVENESS   -> FORCE_CLOSE
  DISPUTE    -> ARBITRATION
```

#### VI.2.b No Silent Timeout

```
invariant NoSilentTimeout:
  on timeout:
    emit EscalationEvent
  else SLASH(30%)
```

**Enforcement:** Timeouts must produce observable escalation. Silent failure = slashing.

### VI.3 Grace Period Rule

```
rule GracePeriod:
  allow once if quorum_signed && no_prior_violation
  else DENY
```

**Constraints:**
- Maximum one extension per swap
- Requires quorum approval
- No prior violations by requester

---

## Section VII: Economic Security Model

### VII.1 Minimum Stake Requirement

```
const MinStake = max(alpha * MaxSwapValue, AbsoluteFloor)

where:
  alpha = 0.10 (10%)
  AbsoluteFloor = 1000 tokens
```

### VII.2 Slashing Classes

| Violation | Slash % | Terminal | Reference |
|-----------|---------|----------|-----------|
| DeterminismViolation | 100% | Yes | VII.2.a |
| ReceiptForgery | 100% | Yes | VII.2.b |
| TimeoutNegligence | 10-30% | No | VII.2.c |
| InvalidEscalation | 50% | No | VII.2.d |
| LivenessFailure | Variable | No | VII.2.e |

#### VII.2.a Economic Finality

```
invariant EconomicFinality:
  expected_loss(adversary) > expected_gain(adversary)
  else HALT("Unsafe incentive configuration")
```

**Formula:**
```
expected_loss = stake * detection_probability * slash_percent
expected_gain = swap_value * (1 - detection_probability) * success_rate
```

### VII.3 Reward Curve

```
reward_multiplier = f(stake_duration, performance_score)

where:
  f is convex (rewards long-term commitment)
  performance_score in [0, 1]
```

### VII.4 Retroactive Guard

```
invariant NoRetroactiveApplication:
  economic_params.version >= swap.created_at.params_version
  else DENY("Retroactive parameter application")
```

---

## Section VIII: Dispute Resolution Protocol

### VIII.1 Challenge Window

```
struct ChallengeWindow:
  t0: BlockNumber           // Challenge start
  delta_challenge: u32      // Window duration
```

```
challenge.valid_if:
  t0 <= now <= t0 + delta_challenge
```

### VIII.2 Evidence Requirements

```
struct CounterProof:
  invariant_hash: [u8; 32]      // Which invariant violated
  receipt_set: Vec<Receipt>     // Domain receipts
  evaluation_trace: Hash        // Deterministic replay
  domain_context: Hash          // Cross-domain state
  guardian_signature: Signature // Non-repudiation
```

#### VIII.2.a Evidence Completeness

```
invariant EvidenceCompleteness:
  all_fields_present(CounterProof)
  else REJECT
```

### VIII.3 Multi-Round Verification

```
enum VerificationStep:
  1: StaticVerification      // Signature + format
  2: DeterministicReplay     // Re-execute invariant
  3: CrossDomainConsistency  // Domain state agreement
  4: EconomicAttribution     // Stake at risk
  5: FinalAdjudication       // Verdict
```

### VIII.4 Final Arbiter

```
arbiter FinalDecision:
  automated -> governance_if_unresolved

where:
  automated = deterministic_vm_execution
  governance = emergency_council (Section X only)
```

**Automated verdicts:**
- SlashConfirmed: Evidence proves violation
- SlashReversed: Counter-evidence proves innocence
- ChallengerPenalized: Frivolous challenge

**Governance escalation (rare):**
- Ambiguous evidence requiring interpretation
- Cross-domain inconsistency beyond protocol scope
- Novel attack vector not covered by constitution

---

## Cross-Reference Map

| Crate | Sections | Primary Types |
|-------|----------|---------------|
| x3-liveness | V.2, VI.1-3 | DomainClock, HTLCWindow, SwapPhase |
| x3-dispute | VI, VIII | Challenge, Response, Dispute, Verdict |
| x3-economics | VII | SlashingClass, EconomicParams, RewardCurve |
| x3-finality | V.3, VIII | FinalityGadget, QuorumTracker |
| x3-audit-trail | IX, X | EventLog, IncidentReport |
| x3-inv-compiler | ALL | ConstitutionalInvariant, CompiledInvariant |

---

## Test Vectors

### V.1: Clock Monotonicity

```rust
// PASS: Normal progression
ClockMonotonicityInvariant::check(EVM, t1=100, clock1=1000, t2=200, clock2=2000)
  => InvariantResult::Pass

// FAIL: Clock regression
ClockMonotonicityInvariant::check(EVM, t1=100, clock1=2000, t2=200, clock2=1000)
  => InvariantResult::Halt(ClockRegression)
```

### V.3.a: Quorum Safety

```rust
// Required: ceil(2/3 * 10) = 7
QuorumSafetyInvariant::required_quorum(10) == 7

// PASS: Quorum met
QuorumSafetyInvariant::check(signatures=7, guardians=10)
  => InvariantResult::Pass

// FAIL: Quorum not met
QuorumSafetyInvariant::check(signatures=6, guardians=10)
  => InvariantResult::Slash(100%, terminal=true)
```

### VI.2.b: No Silent Timeout

```rust
// PASS: Timeout with escalation
NoSilentTimeoutInvariant::check(timeout=true, escalation_emitted=true)
  => InvariantResult::Pass

// FAIL: Silent timeout
NoSilentTimeoutInvariant::check(timeout=true, escalation_emitted=false)
  => InvariantResult::Slash(30%)
```

### VI.3: Grace Period

```rust
// PASS: First extension with quorum
GracePeriodRule::check(quorum=true, violations=0, extensions=0)
  => InvariantResult::Pass

// FAIL: Second extension
GracePeriodRule::check(quorum=true, violations=0, extensions=1)
  => InvariantResult::Deny(GracePeriodExhausted)
```

### VII.2.a: Economic Finality

```rust
// PASS: Attack unprofitable
EconomicFinalityInvariant::check(
  stake=1_000_000,
  swap_value=100_000,
  slash_percent=100,
  detection_probability=95%
) => InvariantResult::Pass

// FAIL: Attack profitable
EconomicFinalityInvariant::check(
  stake=1_000,
  swap_value=100_000,
  slash_percent=10,
  detection_probability=10%
) => InvariantResult::Halt(UnsafeIncentives)
```

---

## Appendix A: Constitutional Hash

The canonical hash of this specification anchors the immutable version:

```
constitutional_hash() -> [u8; 32]

Input concatenation:
  "X3-CONSTITUTIONAL-V1"
  + "V.1.a:ClockMonotonicity"
  + "V.2.a:PhaseExclusivity"
  + "V.2.b:ForwardProgress"
  + "V.3.a:QuorumSafety:100%"
  + "VI.1.a:BoundedHTLC"
  + "VI.2.b:NoSilentTimeout:30%"
  + "VI.3:GracePeriod:once"
  + "VII.1:MinStake:alpha*max+floor"
  + "VII.2:DeterminismViolation:100%:terminal"
  + "VII.2:ReceiptForgery:100%:terminal"
  + "VII.2:TimeoutNegligence:10-30%"
  + "VII.2:InvalidEscalation:50%"
  + "VII.2.a:EconomicFinality"
  + "VIII.1:ChallengeWindow"
  + "VIII.2:EvidenceCompleteness:5fields"
  + "VIII.3:5StepVerification"
  + "VIII.4:AutomatedThenGovernance"

Hash: SHA-256
```

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-01-16 | Initial constitutional specification |

---

*This document is immutable. Amendments require new version with migration path.*
