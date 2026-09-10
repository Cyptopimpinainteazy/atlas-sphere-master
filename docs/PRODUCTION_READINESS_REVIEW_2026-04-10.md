# Atlas Sphere Production Readiness Review

**Date:** 2026-04-10  
**Reviewer:** Codex (deep static and build-surface review)

## Executive Verdict

Atlas Sphere is **not ready for production mainnet launch** yet. The current codebase contains hard blockers in build integrity, runtime correctness, RPC truthfulness, and verification completeness. This report focuses on practical launch-critical gaps and an implementation plan to close them.

---

## Severity Model

- **P0 / Blocker:** Must be fixed before any production launch
- **P1 / Critical:** Must be fixed before permissionless operations
- **P2 / High:** Required for performance, economics, and operator safety
- **P3 / Medium:** Should be completed before broad ecosystem onboarding

---

## Findings

### 1) Build and workspace integrity is broken (**P0**)

- Workspace members reference missing local `x3-lang` paths (`../../x3-lang/...`) from `crates/cross-vm-bridge/Cargo.toml`, but that directory is absent in this repository checkout.
- This blocks full workspace compilation and therefore blocks reproducible CI, release builds, and security scanning.

**Evidence:**
- `crates/cross-vm-bridge/Cargo.toml` path deps to `../../x3-lang/...`.
- `cargo check -q -p runtime` fails with: `failed to read .../x3-lang/crates/x3-ast/Cargo.toml`.

**Required action:**
1. Replace local path deps with workspace crates that exist in-repo, or vendor/submodule `x3-lang` deterministically.
2. Enforce a CI gate: `cargo check --workspace --all-targets` must pass on clean clone.
3. Lock reproducibility with pinned toolchain and deterministic dependency resolution.

---

### 2) Node behavior is partially stubbed/non-operational (**P0**)

- `node/src/main_stub.rs` prints a banner and exits immediately (`process::exit(0)`), indicating non-functional node entry behavior if this target is used.

**Required action:**
- Remove/retire stub from production build graph, and assert runtime node start in integration tests (boot, sync, produce block, serve RPC).

---

### 3) RPC methods return placeholders instead of canonical state (**P0**)

Multiple methods in `node/src/rpc.rs` still return hardcoded placeholders:

- Canonical balance -> `0u128`
- Authorization -> `false`
- Asset metadata -> `None`
- Comit receipt -> `None`
- Indexer methods return static or empty structures

This is a launch blocker: operators and integrators would consume inaccurate chain state.

**Required action:**
1. Implement storage-backed RPC reads from runtime APIs.
2. Add contract tests comparing RPC responses against direct storage snapshots.
3. Add response schema/versioning and non-zero telemetry for hit/error/latency.

---

### 4) Runtime VM adapters still include mock-path semantics and missing mapping (**P1**)

- `runtime/src/vm_adapters.rs` has deterministic fallback adapters for EVM/SVM when real features are disabled.
- Real adapters still skip critical data mapping (`logs: BoundedVec::default()` TODOs).

This implies a risk that execution observability and state audit trails are incomplete.

**Required action:**
1. Make production build profile fail-fast unless `real-evm`, `real-svm`, and `real-x3` are enabled.
2. Implement full log/event/state-diff mapping into receipts.
3. Add feature-matrix CI to ensure WASM determinism and native parity.

---

### 5) Runtime composition includes intentionally disabled functionality (**P1**)

- `runtime/src/lib.rs` contains TODO-disabled pallets (e.g., botchain commandments) waiting for stable2412 updates.

**Required action:**
- Either complete and re-enable with migration tests, or formally remove from v1 scope and docs to avoid mismatch between protocol claims and chain reality.

---

### 6) Security-critical TODOs remain in verification/guardian surfaces (**P1**)

- `crates/gpu-swarm/src/verification.rs` indicates ed25519 verification TODO.
- `pallets/atomic-guardian/src/lib.rs` includes TODOs for cryptographic receipt validation and slashing logic.

This blocks trust assumptions for off-chain compute receipts and challenge/slash economics.

**Required action:**
1. Complete signature/receipt verification paths.
2. Implement and test slashing conditions and appeal windows.
3. Add adversarial tests (forged receipts, replay, equivocation, timeout gaming).

---

### 7) X3 pipeline is functional in parts but not production-complete (**P1**)

Across `x3-*` crates there are explicit TODOs for bytecode verification completeness, MIR lowering coverage, float typing, and backend safety checks.

**Required action:**
1. Define X3 language v1 frozen subset and reject unsupported constructs at compile-time.
2. Complete verifier coverage for all emitted opcode families.
3. Add differential tests (compiler -> VM -> receipt) and fuzzing harnesses.

---

### 8) Spec/implementation tracking shows critical items unchecked (**P1**)

`docs/specifications/IMPLEMENTATION_TICKETS.md` still lists critical P1 acceptance criteria unchecked for core ledger, dispatcher, and two-phase commit readiness.

**Required action:**
- Move to machine-verifiable status board tied to CI artifacts (tests, benchmarks, audits) instead of unchecked markdown alone.

---

### 9) Coverage and performance targets are not currently enforceable (**P2**)

- Coverage targets exist in workspace metadata, but current workspace build breakage and incomplete test gates prevent trustworthy percentage claims.

**Required action:**
1. Add mandatory coverage jobs (llvm-cov/grcov) with per-package thresholds.
2. Add TPS benchmarking harnesses for EVM, SVM, X3, and mixed cross-VM bundles.
3. Establish reproducible baseline environment for GPU swarm throughput tests.

---

## Mainnet Launch Plan (Practical)

## Phase A — Unblock Build + Truthful RPC (1-2 weeks)

- Fix missing `x3-lang` dependency strategy.
- Make workspace compile on clean clone.
- Remove/disable non-production stub entry paths.
- Replace all placeholder RPC values with runtime-backed reads.
- **Exit criteria:** CI green for check/test, RPC contract tests passing.

## Phase B — Execution Correctness + Security (2-4 weeks)

- Enforce production feature flags for real VM adapters.
- Implement receipt/log/state-diff mapping.
- Complete cryptographic validation + slashing in guardian and swarm verifier.
- Add property/fuzz tests for kernel atomicity and receipt fraud.
- **Exit criteria:** adversarial suite passes; no known critical TODOs on verification paths.

## Phase C — X3 & Cross-VM Hardening (2-4 weeks)

- Freeze X3 v1 subset and compiler diagnostics for unsupported syntax.
- Complete verifier and MIR/backend TODO backlog required for on-chain safety.
- Add cross-VM conformance tests (EVM<->SVM<->X3), deterministic replay, and failure rollback checks.
- **Exit criteria:** deterministic replay corpus passes across independent nodes.

## Phase D — Performance, GPU Validation, and Launch Readiness (2-3 weeks)

- Build benchmark matrix:
  - pure EVM TPS
  - pure SVM TPS
  - pure X3 TPS
  - mixed cross-VM TPS
  - GPU receipt verification throughput
- Tune block weights/length and mempool policies from measured data, not assumptions.
- Run canary + incentivized testnet with chaos drills and incident runbooks.
- **Exit criteria:** sustained SLOs, no consensus divergence, response plans validated.

---

## GPU Validation + Max TPS Readiness Checklist

1. Deterministic task execution transcript hash is stable across hardware classes.
2. Receipt signature and trace-root verification is cryptographically complete.
3. Timeout, re-org, and equivocation penalties are active and tested.
4. Coordinator/verifier selection is Sybil-resistant and observable.
5. TPS is reported at p50/p95/p99 latency with reproducible benchmark scripts.

---

## Coverage Reality (Important)

A literal **100% coverage** target is rarely a meaningful security metric by itself. For mainnet readiness, enforce:

- line + branch coverage thresholds,
- property tests for invariants,
- fuzzers for parsers/verifiers,
- deterministic replay tests for consensus-critical transitions,
- formal checks for select critical modules (kernel commit protocol, receipt verifier).

Use “coverage + adversarial assurance” as the ship gate, not coverage alone.

---

## Recommended Ship Gates (Non-negotiable)

1. Workspace clean build + tests on clean clone.
2. Zero placeholder RPC methods in production profile.
3. No TODOs in critical paths (execution, verification, slashing, state transitions).
4. Independent security audit findings resolved for P0/P1.
5. Reproducible performance report with declared hardware and config.
6. Testnet burn-in period with on-call/incident drills completed.

