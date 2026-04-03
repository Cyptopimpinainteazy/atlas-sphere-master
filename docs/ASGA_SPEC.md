# Atomic Swap Guardian Agent (ASGA) — Formal Specification

> Purpose: Define a canonical, code-independent specification for the Atomic Swap Guardian Agent (ASGA). This document is the single source of truth for the state machine, receipt formats, invariants, on-chain arbiter semantics, chaos harness, monitoring, and red-team playbooks.

---

## 📘 Overview

**Name:** ATOMIC-GUARDIAN-01

**Role:** Sovereign X3 agent responsible for correctness, safety, and adversarial resilience of cross-domain atomic swaps.

**Prime Directive:** No partial execution survives. Ever. If this cannot be proven, the system reverts, slashes, or halts.

---

## I. Formal Swap State Machine (canonical, code-independent)

### 🔷 State Enumeration

- S0: Initialized
- S1: IntentCommitted
- S2: Locking
- S3: Locked
- S4: Executing
- S5: Finalizing
- S6: Completed
- S7: Reverted
- S8: Slashed

### 🔁 State Transitions (strict)

From → To  | Condition
---|---
S0 → S1 | Valid swap intent signed by quorum
S1 → S2 | Arbiter authorizes lock phase
S2 → S3 | All domains emit valid LOCK_RECEIPT
S3 → S4 | Arbiter validates global lock invariant
S4 → S5 | All domains emit EXEC_RECEIPT
S5 → S6 | Finality thresholds met
ANY → S7 | Timeout or invariant violation
ANY → S8 | Malicious proof or equivocation

**Notes:**
- No transition to S6 (Completed) is valid unless _all_ domain finality conditions are satisfied.
- S7 (Reverted) and S8 (Slashed) are absorbing states.

### 🔒 Global Invariants (machine-enforced)

1. All-or-Nothing Settlement (Atomicity)
   - ∀ domains D: settled(D) ⇔ ∀ domains D' settled(D')
2. Time-Bounded Liveness
   - now < global_deadline(intent_id)
3. Receipt Validity
   - receipt(domain) ⊢ signature ∧ state ∧ amount ∧ intent_id
4. Finality Safety
   - confirmations(domain) ≥ domain_min_finality(domain)

**Violation:** immediate revert (S7) or slash (S8) depending on proof severity.

---

## II. Receipt Formats (domain-sealed, unforgeable)

Receipts are cryptographic facts consumed by X3. They are not mere logs.

### 🔹 Common Receipt Header (all domains)

```text
ReceiptHeader {
  intent_id: Hash,
  domain_id: Enum(EVM | SVM | BTC | X3),
  phase: Enum(LOCK | EXEC | FINAL),
  amount: Uint256,
  asset_id: Hash,
  timestamp: Uint64,
  signer: PubKey,
}
```

- All receipts are SCALE-encoded (or canonical binary) and signed by domain authorities or oracle signers.

### 🔹 Domain-specific payloads

EVMReceipt {
  tx_hash: H256,
  block_number: u64,
  confirmations: u32,
  contract_address: H160,
  calldata_hash: H256,
}

SVMReceipt {
  signature: Signature,
  slot: u64,
  program_id: PubKey,
  escrow_pda: PubKey,
}

BTCReceipt {
  txid: Txid,
  vout: u32,
  script_hash: Hash160,
  confirmations: u32,
  locktime: u32,
}

X3Receipt {
  block_hash: Hash,
  runtime_version: u32,
  arbiter_signature: Signature,
}

**Canonical encoding rules:** receipts must be deterministic, canonical (no extra fields), and have a well-specified binary ordering to avoid equivocation by encoding variability.

---

## III. X3VM Arbiter Pallet (authoritative logic)

**Responsibility:** This pallet decides reality for swaps. Only the pallet can update `SwapState` for an `intent_id`.

### 🔧 Core Responsibilities

- Validate receipts
- Track swap state machine
- Enforce deadlines
- Slash misbehavior
- Emit canonical swap state and audit trail

### 🧱 Storage

- `SwapIntents: map<Hash(intent_id) => IntentData>`
- `SwapState: map<intent_id => State>`
- `Receipts: double_map<intent_id, DomainId => Receipt>`
- `Deadlines: map<intent_id => Timestamp>`
- `Slashes: map<AgentId => Penalty>`

### ⚖️ Dispatchable Actions

- `submit_intent(origin, intent)` — submit a signed intent, create `SwapIntent`, set initial deadlines.
- `submit_receipt(origin, intent_id, domain, receipt)` — accepts signed domain receipts and stores them after deterministic validation.
- `advance_state(origin, intent_id)` — attempts to advance the state machine if invariants/conditions satisfied; only callable by Arbiter or by off-chain worker signatures.
- `force_revert(origin, intent_id, evidence)` — forces S7 on proof of invariant violation or timeout.
- `slash_agent(origin, agent_id, evidence)` — impose penalty on misbehaving agent or validator.

**Policy:** No external chain or unilateral party can advance state to S6 without checks.

---

## IV. Regtest / Forked Chaos Harness

**Purpose:** Continuously run destructive scenarios to validate system resilience and produce deterministic replays.

### 🔥 Chaos Scenarios (non-exhaustive)

- EVM: reorg after lock (forked mainnet + simulate reorg depth)
- SVM: slot race / skipped confirmation
- BTC: fee spike + mempool eviction
- X3: validator equivocation / malicious block

### 🧪 Harness Stack

- EVM: Anvil + forked mainnet
- SVM: solana-test-validator (with time/slot controls)
- BTC: bitcoin-core regtest (controlled mempool and fee policies)
- X3: local devnet with fault injection (delayed gossip, split-brain)

Each test:
- records execution trace
- captures receipts and evidence
- replays deterministically
- stores artifact for audit and causality debugging

---

## V. Live Invariant Monitoring (automated response)

This runs off-chain and is authoritative for automated actions (no human-in-the-loop required for immediate emergency actions).

### 📡 Metrics exposed
- Active swaps by state
- Deadline proximity
- Confirmation lag per domain
- Receipt mismatches
- Agent fault scores

### 🤖 Automated Actions

| Condition | Action |
|---|---|
| Deadline breach | force_revert(intent_id) |
| Invalid receipt | slash_agent(signer) + halt swap engine |
| Partial execution detected | emergency_freeze (halt new swaps) |
| Repeated fault | agent_quarantine (prevent new responsibilities) |

**Note:** All automated actions are logged to the audit trail with cryptographic receipts and must be replayable.

---

## VI. Built-in Red Team (the agent attacks itself)

**Playbooks:** reorg griefing, signature replay, fee starvation, receipt forgery attempts, deadline manipulation, cross-domain desync.

Each attack is:
- Simulated (chaos harness)
- Logged
- Scored (severity / likelihood)
- Used to harden invariants and improve thresholds

**Scoring:** produce actionable risk metrics that feed back into the invariant monitor's thresholds.

---

## VII. The Agent (ATOMIC-GUARDIAN-01)

**Permissions:**
- Observe all swap activity
- Trigger chaos tests
- Freeze swap engine
- Slash agents
- Propose invariant upgrades

**Kill Switch:**
- If invariants violated twice within configured window: agent halts new swaps and locks funds until governance proof (multisig/council vote) resumes operations.

**Audit Trail:**
- All agent decisions and automated actions create signed audit receipts stored in `Receipts` and off-chain artifact storage (immutable replay format).

---

## VIII. Governance, Upgrades & Auditability

- **Governance hooks:** on-chain governance proposals to change invariant thresholds, enable/disable agent autonomy, or accept new signer keys.
- **Upgrade policy:** Invariant upgrade proposals must include formal verification or extensive red-team results.
- **Audit trail format:** deterministic replay archive including inputs, receipts, traces, and artifact signatures for regulatory-grade evidence.

---

## IX. Implementation notes & integration plan

- On-chain pallet: Rust, FRAME v2
- Off-chain services: Rust/Go/Python (monitoring, chaos harness orchestration, replay storage)
- Test infra: CI job that runs the chaos harness nightly and validates invariants with deterministic replays

---

## X. Appendix

- Threat model
- Formal definitions (proof obligations to show: atomicity, liveness, safety)
- Example canonical swap with receipts and transition proofs

---

*End of specification — edits should be made in PRs referencing this spec; this document is intended as the canonical starting point for implementation.*
