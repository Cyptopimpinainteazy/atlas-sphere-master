# Atlas Sphere SVM Integration Plan

## Purpose

This document captures the planned approach for integrating Solana Virtual Machine (SVM) execution into the Atlas Sphere runtime. The design treats SVM as a first-class execution environment alongside the EVM, with the Atlas Kernel pallet acting as the canonical ledger and coordination layer for both.

---

## High-Level Architecture

| Layer                 | Role                                                                    |
|-----------------------|-------------------------------------------------------------------------|
| Atlas Runtime         | Hosts FRAME pallets including Atlas Kernel and the forthcoming SVM pallet. |
| Atlas Kernel          | Canonical ledger, Comit coordination, asset registry, and dual-VM routing. |
| SVM Execution Engine  | External sidecar service providing deterministic Solana program execution. |
| Receipt Verifier      | Runtime component that validates sidecar receipts before state application. |

---

## Execution Flow

1. **Comit Submission**  
   - A user submits a Comit containing `svm_payload` (serialized Solana transaction bundle) via `pallet-atlas-kernel::submit_comit`.
   - The Atlas Kernel emits `ComitSubmitted` and enqueues the Comit for SVM processing.

2. **Sidecar Dispatch**  
   - A dedicated SVM sidecar service subscribes to Comit events through RPC.
   - The sidecar executes the Solana programs inside a deterministic SVM runtime (e.g., Agave/Sealevel-compatible execution).

3. **Receipt Generation**  
   - Upon execution, the sidecar produces a receipt containing:
     - Program logs
     - Account state diffs
     - Signatures and execution hash (`prepare_root` validation target)
     - Resource usage metrics (e.g., compute units consumed)

4. **Receipt Submission**  
   - The sidecar (or an authorized relayer) calls a runtime extrinsic (planned `submit_svm_receipt`) provided by the SVM pallet.
   - The extrinsic performs:
     - Signature and hash checks
     - Deterministic replay verification for critical paths (e.g., Merkle proof over account state)
     - Fee accounting via Atlas Kernel

5. **Canonical Ledger Update**  
   - On successful verification, the pallet invokes Atlas Kernel APIs to mutate the canonical ledger balances and account metadata.
   - `ComitFinalized` is emitted referencing the original Comit ID.

6. **Failure Handling**  
   - If receipt verification fails, the Comit is marked failed through Atlas Kernel events and optional slash/penalty logic is triggered for the submitting relayer.

---

## Sidecar Execution Model

- **Isolation**: SVM execution is delegated to a separate process to keep the runtime light and to reuse existing Solana execution engines.
- **Determinism**: Sidecar must operate deterministically given the same inputs. Execution artifacts (state diffs, logs) are hashed and included in receipts.
- **Redundancy**: Multiple sidecars can run in parallel; consensus comes from the runtime verifying receipts, not from the sidecar network itself.
- **Networking**: Sidecars connect to the node via WebSocket RPC and submit receipts through signed extrinsics. Future extensions may use libp2p-based gossip for redundancy.

---

## Receipt Verification Strategy

1. **Prepare Root Check**  
   - The receipt includes the `prepare_root` computed in the Comit. Runtime recomputes the hash over the payloads and compares.

2. **State Diff Validation**  
   - Sidecar encodes account diffs as Merkle proofs against Solana account state commitments maintained by the Atlas Kernel.
   - Runtime verifies the proofs and ensures resulting balances match canonical ledger constraints.

3. **Program Authenticity**  
   - Programs are referenced by their Solana `Pubkey`. The pallet maintains a registry of whitelisted programs/modules to mitigate malicious code execution.

4. **Fee Reconciliation**  
   - Receipt includes compute unit usage. Runtime calculates fees in ATLAS (or other registered assets) using Atlas Kernel pricing data.
   - Fees are debited from the origin account in the canonical ledger.

5. **Replay Protection**  
   - Nonce tracking lives in the Atlas Kernel. Each Comit + receipt pair is single-use, enforced by storage flags in the SVM pallet.

---

## Canonical Ledger Interaction

- **Balance Updates**  
  - SVM pallet calls `AtlasKernel::update_canonical_balance` to write post-execution state.  
  - Multi-asset support allows Solana-native tokens (e.g., SOL, USDC) to map onto Atlas asset IDs.
- **Account Mapping**  
  - Solana account public keys map to Atlas IDs. The pallet provides helper methods to derive these IDs and link them to Substrate accounts.
- **Asset Registry**  
  - The pallet relies on the Atlas Kernel asset registry. New Solana assets are registered through kernel governance flows before being usable in SVM programs.

---

## Solana Toolchain Compatibility

- **Program Compilation**  
  - Developers can compile programs using the standard Rust Solana toolchain (`cargo build-bpf` / `cargo build-sbf`).  
  - Build artifacts (`.so` files) are deployed to Atlas Sphere via sidecar APIs that mirror Solana’s deploy instructions.

- **Transaction Encoding**  
  - Serialized Solana transactions are accepted directly. The SVM pallet includes utilities for converting standard Solana wire formats to the required SCALE-encoded payloads.

- **CLI Support**  
  - The sidecar will expose a compatibility layer for `solana-cli`, enabling commands like `solana transfer` routed through Atlas Sphere.

---

## Anchor Framework Support

1. **IDL Synchronization**  
   - Planned support for uploading Anchor IDLs to Atlas Sphere for off-chain clients.
   - The sidecar will cache IDLs and expose them through REST/GraphQL endpoints.

2. **Anchor Client Compatibility**  
   - Anchor clients will interact with Atlas Sphere via the sidecar proxy, ensuring instruction serialization matches the runtime expectations.

3. **Program Deployment**  
   - Anchor `deploy` workflows map to sidecar deployment endpoints, which then register the program with the SVM pallet and Atlas Kernel.

---

## Roadmap & Milestones

| Milestone                                | Description                                                                 | Status |
|------------------------------------------|-----------------------------------------------------------------------------|--------|
| SVM Pallet Skeleton                      | FRAME pallet with storage for receipts, configs, and linkage to Atlas Kernel | Planned |
| Sidecar Prototype                        | Basic execution runner using Solana runtime and RPC integration             | Planned |
| Receipt Verification MVP                 | Hash & signature checks, prepare root validation, canonical ledger updates  | Planned |
| Anchor Compatibility Layer               | Sidecar endpoints + tooling for Anchor developers                           | Planned |
| Full Solana CLI Support                  | Proxy all standard CLI commands through Atlas Sphere                        | Planned |
| Redundant Sidecar Network                | Multiple verifiers with economic incentives                                 | Planned |

---

## Open Questions

- **Economic Incentives**: How will sidecar operators be rewarded or slashed for correct/incorrect receipts?
- **State Proofs**: What is the optimal commitment scheme for Solana account state to minimize receipt size while preserving security?
- **Concurrency**: How will the runtime handle conflicting receipts for the same Comit (e.g., equivocation by a malicious sidecar)?
- **Governance**: Which programs and assets should be whitelisted by default, and how are changes approved?

---

## Next Steps

1. Finalize the SVM pallet interface and storage schema.
2. Implement the sidecar prototype with deterministic execution and receipt generation.
3. Define SCALE-encoded receipt structures and integrate them with the Atlas Kernel.
4. Build developer tooling (CLI adapters, Anchor plugins).
5. Stand up testnet infrastructure for end-to-end validation.

--- 

For questions or contributions, please coordinate via the Atlas Sphere repo issues or development channels.