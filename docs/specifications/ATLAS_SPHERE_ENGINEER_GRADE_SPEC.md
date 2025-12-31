# Atlas Sphere Protocol Specification v1.0 (Engineer-Grade)

**Version:** 1.0  
**Date:** December 30, 2025  
**Status:** Draft - Ready for Implementation  

## Table of Contents

1. [Scope & Non-Goals](#1-scope--non-goals)
2. [System Model](#2-system-model)
3. [Canonical Ledger](#3-canonical-ledger)
4. [Dual-VM Runtime](#4-dual-vm-runtime)
5. [Atlas Kernel (Atomic Commit Layer)](#5-atlas-kernel-atomic-commit-layer)
6. [GPU Swarm Protocol](#6-gpu-swarm-protocol)
7. [X3 Language & VM](#7-x3-language--vm)
8. [Economic Model](#8-economic-model)
9. [Security Model](#9-security-model)
10. [Formal Invariants](#10-formal-invariants)
11. [Implementation Roadmap](#11-implementation-roadmap)
12. [Adversarial Threats](#12-adversarial-threats)
13. [Missing Components](#13-missing-components)

---

## 1. Scope & Non-Goals

### In Scope

- **Canonical Ledger & state model** - Single global state root
- **Dual-VM execution** - EVM + SVM on shared state
- **Atomic cross-VM commit semantics** - All-or-nothing execution
- **GPU Swarm off-chain compute protocol** - Deterministic parallel execution
- **X3 language + VM + verifier** - Strategy programming language
- **Economic & security primitives** - Incentives and safety mechanisms

### Explicit Non-Goals (v1)

- **Trustless cross-chain bridging** to external L1s (future phase)
- **Privacy/ZK rollups** beyond receipt proofs
- **Permissionless AI model training** (inference + simulation only)
- **Full formal verification** of all components (pilot on critical paths)

---

## 2. System Model

### 2.1 Node Types

| Node Type | Responsibilities | Trust Assumptions |
|-----------|------------------|-------------------|
| **Validator** | Consensus, block production, state transition verification | Byzantine fault tolerant (≤ 1/3 faulty) |
| **GPU Swarm Node** | Off-chain deterministic compute, receipt generation | Untrusted, economically bonded |
| **RPC / Indexer** | Read-only access | No trust |
| **Developer Client** | Submits tx, tasks | No trust |

### 2.2 Execution Environment

- **Consensus Layer**: Deterministic, CPU-bound, minimal surface area
- **Compute Layer**: Nondeterministic internally, massively parallel, externally verified
- **Verification Layer**: Proof-based validation of off-chain computation

---

## 3. Canonical Ledger

### 3.1 State Root Model

The chain maintains exactly one global state root:

```
StateRoot = H(
  Accounts,
  Contracts,
  Programs,
  Assets,
  X3State,
  SwarmTasks
)
```

All VMs read from and write to this same root.

### 3.2 Asset Representation

Assets are VM-agnostic primitives:

```rust
struct Asset {
    asset_id: Hash,
    total_supply: u128,
    metadata: AssetMetadata, // { name, symbol, decimals }
    balances: Map<AccountId, u128>
}
```

**ERC-20 and SPL semantics** are VM views, not wrapped assets. VM runtimes map native asset ops → Canonical Ledger ops.

### 3.3 Account Model

```rust
struct Account {
    account_id: AccountId,
    nonce: u64,
    assets: Map<AssetId, u128>,
    vm_state: Map<VMId, bytes>
}
```

---

## 4. Dual-VM Runtime

### 4.1 Execution Routing

Each transaction includes:

```rust
struct TxEnvelope {
    vm_id: VMId, // {EVM | SVM | X3}
    payload: Vec<u8>,
    gas_limit: u64,
    signature: Signature
}
```

Dispatcher routes execution to the appropriate runtime.

### 4.2 VM Isolation

- **EVM and SVM do not share memory**
- **They only communicate via Kernel-mediated calls**
- **No direct cross-VM reentrancy**
- **All state mutations go through Atlas Kernel**

### 4.3 VM Interfaces

```rust
trait VirtualMachine {
    fn execute(&self, payload: &[u8], gas_limit: u64) -> VMResult<VMExecutionResult>;
    fn get_state_diff(&self) -> StateDiff;
    fn get_gas_used(&self) -> u64;
}
```

---

## 5. Atlas Kernel (Atomic Commit Layer)

### 5.1 Comit Object

```rust
struct Comit {
    tx_id: Hash,
    evm_delta: StateDiff,
    svm_delta: StateDiff,
    x3_delta: StateDiff,
    receipts: Vec<Receipt>,
    gas_used: u64,
    hash: Hash
}
```

### 5.2 Two-Phase Commit Protocol

**Phase 1 – Simulation**
- Kernel executes all VM calls in sandbox
- Produces state diffs, no writes
- Validates gas limits and invariants

**Phase 2 – Commit**
- Kernel verifies:
  - Gas limits
  - No invariant violations
  - Receipt validity
- Applies all diffs atomically

### 5.3 Failure Semantics

| Failure Type | Result |
|--------------|--------|
| VM revert | Entire Comit aborted |
| Receipt invalid | Abort |
| Gas exceeded | Abort |
| Validator disagreement | Block rejected |

---

## 6. GPU Swarm Protocol

### 6.1 Task Definition

```rust
struct SwarmTask {
    task_id: Hash,
    x3_bytecode: Vec<u8>,
    input_data: Vec<u8>,
    max_cycles: u64,
    reward: Credits,
    deadline: BlockNumber,
    verifier_id: Option<ValidatorId>
}
```

### 6.2 Execution Flow

1. **Task posted on-chain**
2. **GPU node claims task** (bond locked)
3. **Executes deterministically**
4. **Generates receipt**
5. **Submits receipt on-chain**
6. **Verifier checks proof**
7. **Reward or slash**

### 6.3 Receipt Types (v1)

```rust
struct Receipt {
    task_id: Hash,
    output_hash: Hash,
    trace_root: Hash,
    cycles_used: u64,
    signer: Signature
}
```

**Verification must be O(1) or O(log n) on-chain.**

### 6.4 Slashing Conditions

| Violation | Penalty |
|-----------|---------|
| Invalid receipt | Full bond slash |
| Timeout | Partial slash |
| Nondeterminism | Full slash + ban |
| Double-submit | Full slash |

---

## 7. X3 Language & VM

### 7.1 Execution Model

- **Stack-based**
- **Deterministic**
- **Gas-metered**
- **No syscalls outside hostcalls**

### 7.2 Atomic Block Semantics

```x3
atomic {
    evm_call(...)
    svm_invoke(...)
    swarm_task(...)
}
```

Translated internally into a single Comit.

### 7.3 Hostcalls

| Hostcall | Description |
|----------|-------------|
| `evm_call` | Invoke Solidity contract |
| `svm_invoke_signed` | Invoke Sealevel program |
| `spawn_swarm_task` | Submit off-chain compute |
| `read_asset` | Read canonical asset |
| `write_asset` | Write canonical asset |

### 7.4 X3 VM Interface

```rust
struct X3VM {
    stack: Vec<Value>,
    memory: Memory,
    gas_counter: GasCounter,
    hostcalls: HostcallRegistry
}

impl X3VM {
    fn execute_atomic_block(&mut self, bytecode: &[u8]) -> Result<StateDiff, X3Error>;
    fn verify_deterministic(&self, bytecode: &[u8]) -> bool;
}
```

---

## 8. Economic Model

### 8.1 Blockspace Credits

- **ERC-like native token**
- **Pre-sold for funding GPU supply**
- **Burned on task execution**
- **Deflationary mechanism**

### 8.2 Incentive Alignment

| Actor | Incentive |
|-------|-----------|
| **Validator** | Fees + staking rewards |
| **GPU Node** | Credits + task rewards |
| **Developer** | Cheap parallel compute |
| **Network** | Scales without bloating consensus |

### 8.3 Tokenomics (v1)

```rust
struct Tokenomics {
    total_supply: u128,
    emission_schedule: EmissionSchedule,
    burn_rate: Permill,
    staking_rewards: StakingRewards
}
```

---

## 9. Security Model

### 9.1 Threats Addressed

- **Bridge exploits** → eliminated (no bridges)
- **Partial execution** → impossible (atomic commits)
- **GPU dishonesty** → bonded & verified
- **AI hallucination** → sandboxed X3

### 9.2 Known Risks (v1)

| Risk | Mitigation |
|------|------------|
| Kernel bug | Formal verification (TLA+/Coq) |
| Receipt verifier flaw | Multi-verifier rotation |
| Economic spam | Task fees + staking |
| Strategy centralization | Competitive swarm selection |

---

## 10. Formal Invariants (Non-Negotiable)

1. **Atomicity across all VMs** - Either all state diffs commit, or none do
2. **Single canonical asset state** - No asset duplication or inconsistency
3. **Deterministic off-chain compute** - Same input → same output
4. **Bounded on-chain verification** - O(1) or O(log n) proof checking
5. **Economic punishment > profit from cheating** - Rational actors cannot profit from attacks

**If any of these fail, the chain is compromised.**

---

## 11. Implementation Roadmap

### Phase 1: Core Infrastructure (Weeks 1-4)

- [ ] **Canonical Ledger Implementation**
  - State root calculation
  - Asset registry
  - Account model
- [ ] **Dual-VM Runtime**
  - EVM integration
  - SVM integration
  - Dispatcher
- [ ] **Atlas Kernel**
  - Comit object
  - Two-phase commit
  - Failure handling

### Phase 2: GPU Swarm (Weeks 5-8)

- [ ] **Swarm Protocol**
  - Task definition
  - Execution flow
  - Receipt generation
- [ ] **Economic Model**
  - Token implementation
  - Staking system
  - Reward distribution
- [ ] **Security Model**
  - Slashing conditions
  - Bond management
  - Verifier rotation

### Phase 3: X3 Language (Weeks 9-12)

- [ ] **X3 VM**
  - Stack-based execution
  - Gas metering
  - Hostcall system
- [ ] **Atomic Blocks**
  - Cross-VM coordination
  - Deterministic execution
  - Error handling
- [ ] **Formal Verification**
  - TLA+ specifications
  - Coq proofs
  - Invariant checking

### Phase 4: Integration & Testing (Weeks 13-16)

- [ ] **End-to-End Testing**
  - Cross-VM scenarios
  - Swarm integration
  - Economic simulations
- [ ] **Security Audits**
  - Formal verification
  - Penetration testing
  - Economic modeling
- [ ] **Documentation**
  - Protocol specification
  - Developer guides
  - Security whitepapers

---

## 12. Adversarial Threats

### 12.1 Byzantine GPU Behavior

**Threat**: GPU nodes submit invalid receipts
**Mitigation**: 
- Bond slashing
- Multi-verifier rotation
- Watchdog nodes
- Time-delayed finality

### 12.2 Collusion Attacks

**Threat**: Multiple GPU nodes collude to submit false proofs
**Mitigation**:
- Random task assignment
- Reputation-based selection
- Economic penalties
- Watchdog monitoring

### 12.3 Long-Range Attacks

**Threat**: Historical state manipulation
**Mitigation**:
- Checkpointing
- Finality gadgets
- Economic finality
- Watchdog alerts

### 12.4 Adaptive Adversaries

**Threat**: Attackers adapt to protocol changes
**Mitigation**:
- Protocol upgrades
- Governance mechanisms
- Emergency halts
- Circuit breakers

---

## 13. Missing Components

### 13.1 Formal Kernel Spec

**Status**: Not implemented
**Priority**: Critical
**Action**: TLA+/Coq formalization of Atlas Kernel

### 13.2 Receipt Fraud-Proof Fallback

**Status**: Not implemented
**Priority**: High
**Action**: Optimistic fraud proof system for receipt verification

### 13.3 Swarm Reputation Decay

**Status**: Not implemented
**Priority**: Medium
**Action**: Reputation system with decay for long-term security

### 13.4 Governance Capture Resistance

**Status**: Not implemented
**Priority**: High
**Action**: Decentralized governance with anti-capture mechanisms

### 13.5 Kill Switches for AI Evolution Loops

**Status**: Not implemented
**Priority**: Critical
**Action**: Emergency halt mechanisms for autonomous systems

---

## Conclusion

This specification provides the foundation for building Atlas Sphere as a production-ready blockchain protocol. The design prioritizes:

1. **Security** - Formal invariants and economic guarantees
2. **Scalability** - Parallel compute with deterministic verification
3. **Interoperability** - Native support for multiple VMs
4. **Economic Sustainability** - Incentive-aligned tokenomics

**Next Steps**: Begin Phase 1 implementation with focus on the Canonical Ledger and Dual-VM Runtime. Establish formal verification pipeline for critical components.

**Warning**: This specification is not yet ready for mainnet deployment. Critical components (formal verification, governance, kill switches) must be implemented and audited before production use.

---

*This document is a living specification. Updates will be made as implementation progresses and new insights are gained.*