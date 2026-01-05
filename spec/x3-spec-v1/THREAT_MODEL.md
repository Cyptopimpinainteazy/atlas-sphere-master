# X3 — Auditor-Ready Threat Model

**Version:** 1.0
**Status:** CANONICAL
**Classification:** Invariant-Mapped, Enforcement-Explicit

---

## 0. Threat Modeling Philosophy (Non-Negotiable)

- Assume rational, well-funded adversaries
- Assume partial collusion
- Assume delayed finality
- Assume governance is an attack surface
- Assume "honest majority" is a temporary condition, not a property

**Any attack that cannot be mechanically punished is considered successful by default.**

---

## I. Adversary Classes

### A1. Guardian Agent (Malicious or Lazy)

**Capabilities:**
- Signs receipts
- Submits proofs
- Participates in quorum

**Goals:**
- Extract free option
- Avoid slashing
- Shift blame to domain or peers

**Constraints:**
- Staked
- Identifiable
- Slashable
- Non-repudiable

**Invariant Bindings:**
- `QuorumSafetyInvariant` (V.3.a) — 100% slash for quorum failure
- `NoSilentTimeoutInvariant` (VI.2.b) — 30% slash for silent failure
- `EvidenceCompleteness` (VIII.2) — rejection without complete proof

---

### A2. Domain Adversary (EVM / SVM / BTC)

**Capabilities:**
- Reorgs
- Delayed finality
- Fee manipulation
- Withholding inclusion

**Goals:**
- Break atomicity
- Cause asymmetric settlement
- Stall to grief capital

**Invariant Bindings:**
- `ClockMonotonicityInvariant` (V.1.a) — HALT on clock regression
- `BoundedHTLCInvariant` (VI.1.a) — forced refund on expiry
- `CrossDomainComparator` (VIII) — deterministic finality comparison

---

### A3. Relay / Infrastructure Operator

**Capabilities:**
- Drop messages
- Reorder submissions
- Censor selectively

**Goals:**
- Force timeout
- Trigger refunds asymmetrically
- Create inconsistent views

**Invariant Bindings:**
- `PhaseExclusivityInvariant` (V.2.a) — no phase overlap
- `ForwardProgressInvariant` (V.2.b) — no regression without recovery
- `EscalationPath` (VI.2) — forced escalation on liveness failure

---

### A4. Governance Attacker

**Capabilities:**
- Parameter mutation
- Emergency controls
- Arbiter replacement attempts

**Goals:**
- Retroactive forgiveness
- Slashing avoidance
- Protocol capture

**Invariant Bindings:**
- `RetroactiveGuard` (VII.4) — no retroactive parameter application
- `EconomicParams.version` — version-locked parameters
- `ArbiterDecision` (VIII.4) — automated execution before governance

---

### A5. Economic Cartel

**Capabilities:**
- Multi-agent collusion
- Cross-domain coordination

**Goals:**
- Extract MEV
- Bias outcomes
- Undermine deterrence

**Invariant Bindings:**
- `EconomicFinalityInvariant` (VII.2.a) — attack must be unprofitable
- `MinimumStakeRequirement` (VII.1) — stake exceeds extractable value
- `SlashingClass` (VII.2) — 100% terminal slash for determinism violation

---

## II. Attack → Invariant → Enforcement Mapping

### Attack: Partial Finality (Classic Atomic Swap Failure)

**Sequence:**
1. Lock on Domain A
2. Delay or prevent finality on Domain B
3. Claim timeout refund on A
4. Still execute on B later

**Triggered Invariants:**
- `NoPartialFinality` → `PhaseExclusivityInvariant` (V.2.a)
- `BoundedLiveness` → `BoundedHTLCInvariant` (VI.1.a)
- `PhaseTransition` → `ForwardProgressInvariant` (V.2.b)

**Enforcement:**
- Immediate halt on phase overlap
- Forced refund path after HTLC expiry
- Slashing of any agent submitting contradictory receipts

**Outcome:**
- Attacker loses stake
- Capital is conserved
- No free option exists

---

### Attack: Receipt Replay with Context Drift

**Sequence:**
1. Reuse valid receipt hash
2. Submit under altered context (epoch, phase, intent)
3. Attempt double credit

**Triggered Invariants:**
- `ReceiptCommitment` → `CounterProof.invariant_hash` (VIII.2)
- `NonRepudiation` → `CounterProof.guardian_signature` (VIII.2)
- `DeterministicEvaluation` → `constitutional_hash()` binding

**Enforcement:**
- Receipt rejected at verification step
- Slashing proof auto-generated
- Non-repudiation record emitted to audit trail

**Outcome:**
- Replay fails
- Attacker identified
- Evidence preserved

---

### Attack: Guardian Non-Response (Silent Failure)

**Sequence:**
1. Agent participates in early phase
2. Goes silent before finality
3. Attempts to avoid responsibility

**Triggered Invariants:**
- `BoundedLiveness` → `HTLCWindow.max_blocks`
- `NoSilentTimeoutInvariant` (VI.2.b)
- `ResponsibilityAttribution` → `EscalationPath`

**Enforcement:**
- Automatic 30% slashing after silence window
- Removal from active quorum
- Escalation to fallback guardian set

**Outcome:**
- Silent failure is impossible
- Guardian loses stake proportionally
- Swap proceeds or refunds deterministically

---

### Attack: Governance Rollback Attempt

**Sequence:**
1. Invariant fires
2. Slashing event initiated
3. Governance proposal attempts override

**Triggered Invariants:**
- `GovernanceNonRetroactivity` → `RetroactiveGuard` (VII.4)
- `InvariantSupremacy` → Constitutional enforcement order

**Enforcement:**
- Proposal invalid by construction
- No runtime path to bypass invariant result
- Slashing executes before governance can act

**Outcome:**
- Governance cannot save violators
- Slashing is final
- Protocol integrity preserved

---

### Attack: Free Option via Timeout Asymmetry

**Sequence:**
1. Initiate swap
2. Wait to observe counterparty commitment
3. Exploit timeout difference to claim refund while counterparty is locked

**Triggered Invariants:**
- `BoundedHTLCInvariant` (VI.1.a)
- `GracePeriodRule` (VI.3) — one extension only
- `EconomicFinalityInvariant` (VII.2.a)

**Enforcement:**
- Symmetric timeout paths
- Grace period requires quorum + no prior violations
- Stake forfeiture exceeds option value

**Outcome:**
- Free option eliminated
- Attacker loses stake if detected
- No asymmetric advantage

---

### Attack: Quorum Split-Brain

**Sequence:**
1. Guardians sign conflicting receipts for different domains
2. Attempt to create inconsistent finality views
3. Exploit disagreement for profit

**Triggered Invariants:**
- `QuorumSafetyInvariant` (V.3.a) — 2/3 supermajority required
- `PhaseExclusivityInvariant` (V.2.a) — exactly one phase
- `CrossDomainComparator` (VIII)

**Enforcement:**
- Conflicting signatures → 100% terminal slash
- Deterministic evaluation identifies correct view
- One side loses stake entirely

**Outcome:**
- Split-brain is economically suicidal
- Honest quorum preserves consistency
- Attackers identified and slashed

---

## III. Red-Team Playbooks (Executable Mental Models)

### Playbook RT-01: "Free Option via Delay"

**Objective:** Extract optionality by exploiting timeout asymmetry.

**Attack Vector:**
1. Initiate swap
2. Observe market movement during lock period
3. Claim refund if unfavorable, execute if favorable

**Defense:**
- Symmetric timeout enforcement
- Grace period limited to one use
- Stake at risk exceeds option value

**Result:** FAILS. Timeout paths are symmetric, invariant-gated, and slashable.

---

### Playbook RT-02: "Split-Brain Guardians"

**Objective:** Force quorum disagreement across domains.

**Attack Vector:**
1. Bribe subset of guardians
2. Have them sign conflicting receipts
3. Create irreconcilable state

**Defense:**
- 2/3 supermajority required
- Conflicting signatures = 100% slash
- Deterministic evaluation selects canonical view

**Result:** FAILS. Quorum thresholds + deterministic evaluation → one side slashed.

---

### Playbook RT-03: "Receipt Flood / Race"

**Objective:** Overwhelm verifier to slip malformed receipt.

**Attack Vector:**
1. Submit many receipts rapidly
2. Include malformed receipt in flood
3. Hope verification is skipped

**Defense:**
- Bounded runtime per verification
- Canonical ordering (no race conditions)
- Deterministic reject on any malformation

**Result:** FAILS. Bounded runtime + canonical ordering + deterministic reject.

---

### Playbook RT-04: "Governance Saves Me"

**Objective:** Get caught, then appeal politically.

**Attack Vector:**
1. Violate invariant
2. Submit governance proposal to forgive
3. Rally social consensus

**Defense:**
- Invariants execute before governance can act
- Retroactive forgiveness blocked by construction
- No runtime path to bypass slashing

**Result:** FAILS. Invariants execute before governance can act.

---

### Playbook RT-05: "MEV Extraction via Timing"

**Objective:** Extract value by manipulating execution order.

**Attack Vector:**
1. Observe pending swaps
2. Front-run or sandwich
3. Extract MEV from price movement

**Defense:**
- Deterministic ordering
- Bounded execution windows
- Economic parameters version-locked

**Result:** MITIGATED. MEV bounded by deterministic execution order.

---

### Playbook RT-06: "Griefing Attack"

**Objective:** Cause loss to others even at personal cost.

**Attack Vector:**
1. Initiate swaps with no intent to complete
2. Force counterparties into timeout paths
3. Waste their capital lockup time

**Defense:**
- Stake at risk exceeds griefing benefit
- Timeout slashing applies to initiator
- Reputation system (future enhancement)

**Result:** FAILS. Griefing costs exceed griefing benefit.

---

## IV. Economic Reality Check

**If an attack:**
- Costs less than it pays → **the system is broken**
- Cannot be proven → **the system is broken**
- Requires trust → **the system is broken**

**X3 passes because:**
- Slashing > max extractable value
- Proofs are mandatory
- Humans cannot intervene mid-execution

---

## V. Invariant Coverage Matrix

| Attack Class | Primary Invariant | Secondary Invariant | Enforcement |
|--------------|-------------------|---------------------|-------------|
| Partial Finality | V.2.a PhaseExclusivity | VI.1.a BoundedHTLC | HALT + REFUND |
| Receipt Replay | VIII.2 EvidenceCompleteness | constitutional_hash | REJECT + SLASH |
| Silent Failure | VI.2.b NoSilentTimeout | VI.2 EscalationPath | SLASH 30% |
| Governance Override | VII.4 RetroactiveGuard | VIII.4 ArbiterDecision | DENY |
| Free Option | VI.1.a BoundedHTLC | VII.2.a EconomicFinality | SLASH |
| Split-Brain | V.3.a QuorumSafety | V.2.a PhaseExclusivity | SLASH 100% |
| Receipt Flood | VIII.2 EvidenceCompleteness | Bounded runtime | REJECT |
| MEV Extraction | VII.2.a EconomicFinality | Deterministic ordering | MITIGATE |
| Griefing | VII.1 MinStake | VI.2.b NoSilentTimeout | SLASH |

---

## VI. TLA+ Counterexample Verification

The following TLA+ specifications hunt for violations of this threat model:

| Specification | Attacks Hunted |
|---------------|----------------|
| `X3FailureEscalation.tla` | Partial finality, silent timeout, split-brain, grace period abuse |
| `X3EconomicSecurity.tla` | Free option, collusion, griefing, MEV, insufficient slashing |

**Run with:** `tlc -deadlock <spec>.tla`

Counterexample discovery = threat model failure = spec update required.

---

## VII. Auditor Checklist

### For Each Attack Vector:

- [ ] Identify triggering invariant(s)
- [ ] Verify enforcement action is automatic
- [ ] Confirm no governance override path exists
- [ ] Validate economic parameters make attack unprofitable
- [ ] Check TLA+ specification includes hunt invariant
- [ ] Review test vectors for edge cases

### For Protocol as Whole:

- [ ] All adversary classes mapped to invariants
- [ ] All invariants have enforcement actions
- [ ] No discretionary paths during execution
- [ ] Governance is prospective only
- [ ] Slashing exceeds extractable value
- [ ] Evidence is hash-chained and non-repudiable

---

*This threat model is designed to be attacked. If you find a gap, file an issue. If the gap is exploitable, you've earned a bounty.*
