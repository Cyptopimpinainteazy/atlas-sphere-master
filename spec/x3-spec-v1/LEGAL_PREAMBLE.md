# X3 Protocol
## Formal Legal & Interpretive Preamble

**Version:** 1.0 — Canonical
**Status:** NORMATIVE
**Effective:** Upon first interaction with Protocol

---

## 1. Purpose and Scope

This document establishes the authoritative interpretive framework governing the X3 Protocol, including but not limited to its invariant system, atomic execution mechanisms, guardian agents, slashing logic, and cross-domain coordination components (collectively, the "Protocol").

**This preamble is normative, not descriptive.**

In the event of any ambiguity, omission, or conflict between:

1. this preamble,
2. any technical specification,
3. any implementation,
4. any commentary, documentation, marketing material, or oral statement,

**this preamble SHALL control interpretation.**

---

## 2. No Discretion Clause

The X3 Protocol is explicitly designed to operate **without human discretion at execution time**.

Accordingly:

No participant, operator, developer, validator, guardian, or governance body has authority to:

- override invariant outcomes,
- suspend enforcement actions,
- retroactively alter execution results,
- reinterpret protocol-defined violations after occurrence.

**Any outcome produced by invariant evaluation, receipt verification, or state transition logic is final, binding, and mechanically justified.**

Intent, negligence, mistake, good faith, or extenuating circumstances are **irrelevant to enforcement**.

---

## 3. Determinism and Finality

All Protocol actions are governed by deterministic rules.

For legal and operational purposes:

- A Protocol outcome is deemed **final** at the moment the relevant invariant evaluates to true or false and the associated action is triggered.

- There is **no appeal, rollback, forgiveness, or discretionary remediation** within the Protocol.

- Any external agreement or understanding that purports to contradict or supersede Protocol finality is **void** with respect to on-chain or protocol-governed outcomes.

---

## 4. Assumption Rejection

The Protocol expressly rejects reliance on the following assumptions:

- honest majority
- rational economic behavior absent enforcement
- social consensus during failure
- off-chain arbitration as a safety net
- informal norms or operator goodwill

Any system, agreement, or integration that depends on such assumptions operates **outside the guarantees of X3** and does so at its own risk.

---

## 5. Role of Governance

Governance within the X3 ecosystem, where present, is **prospective only**.

**Governance MAY:**

- set parameters in advance,
- upgrade code paths subject to pre-defined rules,
- define future invariant sets.

**Governance MAY NOT:**

- interfere with active swaps,
- halt enforcement mid-execution,
- pardon violations,
- alter the consequences of invariant failure after the fact.

**Governance is a configuration mechanism, not an adjudicative body.**

---

## 6. Guardian Agents and Liability Boundary

Guardian agents and similar actors operate as **bonded executors, not fiduciaries**.

By participating, such agents explicitly acknowledge that:

- enforcement is automatic,
- slashing is mechanical,
- fault attribution is protocol-derived,
- participation constitutes consent to collateral forfeiture under defined conditions.

The Protocol **does not recognize defenses** based on:

- operator error,
- software bugs outside the invariant boundary,
- misconfiguration,
- latency, downtime, or infrastructure failure.

---

## 7. No Warranty / No Representation of Fitness

The X3 Protocol makes **no representation** that it is suitable for:

- consumer applications,
- retail financial products,
- discretionary financial management,
- experimental or exploratory use cases.

The Protocol is intended for **adversarial, high-stakes, machine-governed execution environments only**.

Use outside this scope is **explicitly disclaimed**.

---

## 8. Evidence Supremacy Clause

Protocol-generated artifacts—including but not limited to:

- invariant evaluation traces,
- receipt proofs,
- slashing proofs,
- state transition records,

constitute the **sole authoritative evidence** of events occurring within the Protocol.

External logs, testimony, or reconstructions have **no interpretive authority** unless explicitly referenced by the Protocol itself.

---

## 9. No Implied Partnership or Agency

Participation in, development of, or interaction with the X3 Protocol **does not create**:

- partnership,
- joint venture,
- fiduciary duty,
- agency relationship,

between any parties, regardless of economic outcome.

---

## 10. Interpretive Maxim

This Protocol SHALL be interpreted under the following maxim:

> **If a behavior is not explicitly permitted by invariant and state machine definition, it is forbidden.**
>
> **If a failure mode is not explicitly handled, execution must halt or revert.**

There is no gap for intention to fill.

---

## 11. Binding Nature

Use of, interaction with, or deployment upon the X3 Protocol constitutes **irrevocable acceptance** of this preamble and its consequences.

---

## 12. Severability

If any provision of this preamble is held to be unenforceable or invalid under applicable law, such provision shall be modified to the minimum extent necessary to make it enforceable while preserving its original intent. All other provisions shall remain in full force and effect.

---

## 13. Governing Framework

This preamble is designed to be **jurisdiction-neutral**. It describes the mechanical behavior of the Protocol, not legal rights or obligations under any particular legal system.

Where local law conflicts with Protocol mechanics:

- The Protocol will continue to execute as designed.
- Legal consequences are the responsibility of participants to assess.
- The Protocol makes no accommodation for jurisdictional variation.

---

## 14. Amendment

This preamble may only be amended through:

1. Formal version increment (e.g., Version 2.0)
2. Publication with new hash anchor
3. Prospective application only (no retroactive effect)

Amendments do not apply to swaps, disputes, or enforcement actions initiated prior to the amendment's effective date.

---

## 15. Definitions

| Term | Definition |
|------|------------|
| **Invariant** | A machine-enforced rule that, when violated, triggers automatic enforcement action |
| **Guardian Agent** | A bonded participant authorized to sign receipts and participate in quorum |
| **Slashing** | Automatic forfeiture of staked collateral upon protocol-defined violation |
| **Finality** | The irreversible commitment of a state transition |
| **Quorum** | The minimum threshold of guardian signatures required for valid state transition |
| **Execution Time** | The period during which a swap is active and subject to invariant enforcement |
| **Governance** | The mechanism for setting future parameters; explicitly excluded from execution-time authority |

---

## 16. Contact and Dispute Resolution

The Protocol does not provide:

- customer support,
- dispute resolution services,
- mediation or arbitration,
- recovery mechanisms for user error.

Participants are responsible for their own due diligence, operational security, and legal compliance.

---

## Acknowledgment

By reading this document, you acknowledge that:

1. You have read and understood this preamble.
2. You accept its binding nature.
3. You understand that Protocol outcomes are final and non-negotiable.
4. You assume all risks associated with participation.

**If you do not accept these terms, do not interact with the X3 Protocol.**

---

*This preamble is hash-anchored to the X3 Constitutional Specification v1.0. Any modification invalidates the anchor.*
