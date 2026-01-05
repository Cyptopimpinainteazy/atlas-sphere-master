----------------------------- MODULE X3FailureEscalation -----------------------------
(*
 * X3 Failure Escalation - TLA+ Counterexample Search
 *
 * PURPOSE: Hunt for protocol violations, NOT verify correctness.
 *
 * This specification is designed to FIND attacks:
 *   - Delayed finality exploitation
 *   - Quorum split-brain conditions
 *   - Receipt forgery acceptance
 *   - Grace period abuse
 *   - Replay attacks across domains
 *   - Silent failure paths
 *   - Economic free-option attacks
 *
 * Run with: tlc -deadlock X3FailureEscalation.tla
 *
 * Constitutional Sections: V (Sync), VI (Timeout), VII (Economics), VIII (Dispute)
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

\* ==========================================================================
\* CONSTANTS
\* ==========================================================================

CONSTANTS
    Domains,            \* Set of domains: {"EVM", "SVM", "BTC", "X3"}
    Guardians,          \* Set of guardians
    MaxBlocks,          \* Upper bound for model checking
    MinStake,           \* Minimum stake requirement
    SlashPercent,       \* Slashing percentage (0-100)
    ChallengeWindow,    \* Blocks for challenge submission
    GracePeriod         \* Grace period blocks

ASSUME Cardinality(Domains) >= 2
ASSUME Cardinality(Guardians) >= 3
ASSUME MaxBlocks > 0
ASSUME MinStake > 0
ASSUME SlashPercent >= 0 /\ SlashPercent <= 100
ASSUME ChallengeWindow > 0
ASSUME GracePeriod > 0

\* ==========================================================================
\* VARIABLES
\* ==========================================================================

VARIABLES
    \* Time / Blocks
    block,              \* Current block number (X3 canonical)
    domainClocks,       \* Per-domain clock values

    \* Swap State
    phase,              \* Current swap phase
    lockTime,           \* When swap was locked

    \* Guardian State
    stakes,             \* Guardian stakes
    signatures,         \* Set of guardians who signed
    slashed,            \* Set of slashed guardians

    \* Receipts
    receipts,           \* Domain -> receipt hash (or NULL)

    \* Dispute State
    challengeActive,    \* Is there an active challenge?
    challengeStart,     \* When challenge was submitted
    responseReceived,   \* Has response been submitted?

    \* Grace Period
    gracePeriodUsed,    \* Has grace period been consumed?

    \* Attack Flags (for counterexample detection)
    attackDetected      \* Set of detected attacks

vars == <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
          receipts, challengeActive, challengeStart, responseReceived,
          gracePeriodUsed, attackDetected>>

\* ==========================================================================
\* TYPE INVARIANTS
\* ==========================================================================

TypeOK ==
    /\ block \in 0..MaxBlocks
    /\ domainClocks \in [Domains -> 0..MaxBlocks]
    /\ phase \in {"INIT", "LOCK", "PROVE", "FINALIZE", "COMPLETE",
                  "TIMEOUT", "DISPUTE", "FORCE_CLOSE"}
    /\ lockTime \in 0..MaxBlocks
    /\ stakes \in [Guardians -> 0..MinStake*10]
    /\ signatures \subseteq Guardians
    /\ slashed \subseteq Guardians
    /\ receipts \in [Domains -> {"NULL", "VALID", "FORGED"}]
    /\ challengeActive \in BOOLEAN
    /\ challengeStart \in 0..MaxBlocks
    /\ responseReceived \in BOOLEAN
    /\ gracePeriodUsed \in BOOLEAN
    /\ attackDetected \subseteq {"FreeOption", "SplitBrain", "SilentFailure",
                                  "GracePeriodAbuse", "Replay", "ClockRegression",
                                  "ForgedReceipt", "InsufficientQuorum"}

\* ==========================================================================
\* INITIAL STATE
\* ==========================================================================

Init ==
    /\ block = 0
    /\ domainClocks = [d \in Domains |-> 0]
    /\ phase = "INIT"
    /\ lockTime = 0
    /\ stakes = [g \in Guardians |-> MinStake]
    /\ signatures = {}
    /\ slashed = {}
    /\ receipts = [d \in Domains |-> "NULL"]
    /\ challengeActive = FALSE
    /\ challengeStart = 0
    /\ responseReceived = FALSE
    /\ gracePeriodUsed = FALSE
    /\ attackDetected = {}

\* ==========================================================================
\* QUORUM HELPERS
\* ==========================================================================

RequiredQuorum == (2 * Cardinality(Guardians) + 2) \div 3

ActiveGuardians == Guardians \ slashed

HasQuorum == Cardinality(signatures) >= RequiredQuorum

\* ==========================================================================
\* PHASE TRANSITIONS (Constitutional V.2)
\* ==========================================================================

\* V.2.a: Forward progress only
AllowedTransitions ==
    CASE phase = "INIT" -> {"LOCK"}
      [] phase = "LOCK" -> {"PROVE", "TIMEOUT"}
      [] phase = "PROVE" -> {"FINALIZE", "DISPUTE"}
      [] phase = "FINALIZE" -> {"COMPLETE", "FORCE_CLOSE"}
      [] phase = "COMPLETE" -> {}
      [] phase = "TIMEOUT" -> {}
      [] phase = "DISPUTE" -> {"COMPLETE"}
      [] phase = "FORCE_CLOSE" -> {}
      [] OTHER -> {}

CanTransition(newPhase) == newPhase \in AllowedTransitions

\* ==========================================================================
\* ACTIONS - NORMAL PROTOCOL FLOW
\* ==========================================================================

\* Advance block
Tick ==
    /\ block < MaxBlocks
    /\ block' = block + 1
    /\ \E d \in Domains:
        \* Domain clocks advance (possibly non-monotonically for attack search)
        domainClocks' = [domainClocks EXCEPT ![d] =
            IF \E attack \in {"ClockRegression"}: TRUE  \* Allow regression
            THEN domainClocks[d] - 1  \* ATTACK: clock goes backward
            ELSE domainClocks[d] + 1]
    /\ UNCHANGED <<phase, lockTime, stakes, signatures, slashed, receipts,
                   challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed, attackDetected>>

\* Start swap (INIT -> LOCK)
StartSwap ==
    /\ phase = "INIT"
    /\ CanTransition("LOCK")
    /\ phase' = "LOCK"
    /\ lockTime' = block
    /\ UNCHANGED <<block, domainClocks, stakes, signatures, slashed, receipts,
                   challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed, attackDetected>>

\* Guardian signs
GuardianSign(g) ==
    /\ g \in ActiveGuardians
    /\ g \notin signatures
    /\ stakes[g] >= MinStake
    /\ signatures' = signatures \cup {g}
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, slashed, receipts,
                   challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed, attackDetected>>

\* Submit receipt (LOCK -> PROVE)
SubmitReceipt(d) ==
    /\ phase = "LOCK"
    /\ CanTransition("PROVE")
    /\ receipts[d] = "NULL"
    /\ \/ receipts' = [receipts EXCEPT ![d] = "VALID"]  \* Normal
       \/ receipts' = [receipts EXCEPT ![d] = "FORGED"] \* ATTACK
    /\ phase' = "PROVE"
    /\ UNCHANGED <<block, domainClocks, lockTime, stakes, signatures, slashed,
                   challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed, attackDetected>>

\* Finalize with quorum (PROVE -> FINALIZE)
FinalizeWithQuorum ==
    /\ phase = "PROVE"
    /\ CanTransition("FINALIZE")
    /\ \/ HasQuorum                     \* Normal
       \/ ~HasQuorum                    \* ATTACK: finalize without quorum
    /\ phase' = "FINALIZE"
    /\ UNCHANGED <<block, domainClocks, lockTime, stakes, signatures, slashed, receipts,
                   challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed, attackDetected>>

\* Complete swap (FINALIZE -> COMPLETE)
Complete ==
    /\ phase = "FINALIZE"
    /\ CanTransition("COMPLETE")
    /\ \A d \in Domains: receipts[d] # "NULL"
    /\ phase' = "COMPLETE"
    /\ UNCHANGED <<block, domainClocks, lockTime, stakes, signatures, slashed, receipts,
                   challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed, attackDetected>>

\* ==========================================================================
\* ACTIONS - TIMEOUT & ESCALATION (Constitutional VI)
\* ==========================================================================

\* VI.1: Timeout triggers escalation
Timeout ==
    /\ phase = "LOCK"
    /\ block > lockTime + ChallengeWindow
    /\ CanTransition("TIMEOUT")
    /\ phase' = "TIMEOUT"
    /\ UNCHANGED <<block, domainClocks, lockTime, stakes, signatures, slashed, receipts,
                   challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed, attackDetected>>

\* VI.2: Silent timeout (NO escalation event) - ATTACK
SilentTimeout ==
    /\ phase = "LOCK"
    /\ block > lockTime + ChallengeWindow
    \* Phase stays LOCK (silent failure)
    /\ attackDetected' = attackDetected \cup {"SilentFailure"}
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed>>

\* VI.3: Grace period extension
UseGracePeriod ==
    /\ ~gracePeriodUsed
    /\ HasQuorum
    /\ block >= lockTime + ChallengeWindow - GracePeriod
    /\ gracePeriodUsed' = TRUE
    \* Reset effective deadline by GracePeriod blocks
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, responseReceived,
                   attackDetected>>

\* Grace period abuse: use twice - ATTACK
AbuseGracePeriod ==
    /\ gracePeriodUsed  \* Already used
    /\ attackDetected' = attackDetected \cup {"GracePeriodAbuse"}
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed>>

\* ==========================================================================
\* ACTIONS - DISPUTE PROTOCOL (Constitutional VIII)
\* ==========================================================================

\* VIII.1: Submit challenge
SubmitChallenge ==
    /\ phase \in {"PROVE", "FINALIZE"}
    /\ ~challengeActive
    /\ block <= lockTime + ChallengeWindow
    /\ challengeActive' = TRUE
    /\ challengeStart' = block
    /\ phase' = "DISPUTE"
    /\ UNCHANGED <<block, domainClocks, lockTime, stakes, signatures, slashed, receipts,
                   responseReceived, gracePeriodUsed, attackDetected>>

\* VIII.2: Submit response (evidence)
SubmitResponse ==
    /\ challengeActive
    /\ ~responseReceived
    /\ block <= challengeStart + ChallengeWindow
    /\ responseReceived' = TRUE
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, gracePeriodUsed,
                   attackDetected>>

\* VIII.4: Resolve dispute
ResolveDispute ==
    /\ phase = "DISPUTE"
    /\ challengeActive
    /\ block > challengeStart + ChallengeWindow
    /\ challengeActive' = FALSE
    /\ phase' = "COMPLETE"  \* Or slash based on evidence
    /\ UNCHANGED <<block, domainClocks, lockTime, stakes, signatures, slashed, receipts,
                   challengeStart, responseReceived, gracePeriodUsed, attackDetected>>

\* ==========================================================================
\* ACTIONS - ECONOMIC ATTACKS (Constitutional VII)
\* ==========================================================================

\* Free option attack: initiator waits to see outcome
FreeOptionAttack ==
    /\ phase = "LOCK"
    /\ block > lockTime + ChallengeWindow - 1  \* Wait until last moment
    /\ ~HasQuorum  \* Can still back out
    \* Attacker chooses to not sign, keeping the option
    /\ attackDetected' = attackDetected \cup {"FreeOption"}
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed>>

\* Insufficient stake operation
InsufficientStakeOp(g) ==
    /\ g \in Guardians
    /\ stakes[g] < MinStake
    /\ g \in signatures  \* Signed but without proper stake
    /\ attackDetected' = attackDetected \cup {"InsufficientQuorum"}
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed>>

\* ==========================================================================
\* ACTIONS - CROSS-DOMAIN ATTACKS
\* ==========================================================================

\* Clock regression detection
DetectClockRegression(d) ==
    /\ block > 0
    /\ domainClocks'[d] < domainClocks[d]  \* Clock went backward
    /\ attackDetected' = attackDetected \cup {"ClockRegression"}
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed>>

\* Split-brain: domains disagree on finality
SplitBrainAttack ==
    /\ phase = "FINALIZE"
    /\ \E d1, d2 \in Domains:
        /\ d1 # d2
        /\ receipts[d1] = "VALID"
        /\ receipts[d2] = "NULL"  \* One domain doesn't have receipt
    /\ attackDetected' = attackDetected \cup {"SplitBrain"}
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed>>

\* Forged receipt accepted
ForgedReceiptAccepted ==
    /\ phase \in {"FINALIZE", "COMPLETE"}
    /\ \E d \in Domains: receipts[d] = "FORGED"
    /\ attackDetected' = attackDetected \cup {"ForgedReceipt"}
    /\ UNCHANGED <<block, domainClocks, phase, lockTime, stakes, signatures, slashed,
                   receipts, challengeActive, challengeStart, responseReceived,
                   gracePeriodUsed>>

\* ==========================================================================
\* NEXT STATE
\* ==========================================================================

Next ==
    \/ Tick
    \/ StartSwap
    \/ \E g \in Guardians: GuardianSign(g)
    \/ \E d \in Domains: SubmitReceipt(d)
    \/ FinalizeWithQuorum
    \/ Complete
    \/ Timeout
    \/ SilentTimeout
    \/ UseGracePeriod
    \/ AbuseGracePeriod
    \/ SubmitChallenge
    \/ SubmitResponse
    \/ ResolveDispute
    \/ FreeOptionAttack
    \/ \E g \in Guardians: InsufficientStakeOp(g)
    \/ \E d \in Domains: DetectClockRegression(d)
    \/ SplitBrainAttack
    \/ ForgedReceiptAccepted

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

\* ==========================================================================
\* SAFETY PROPERTIES (These should be VIOLATED to find attacks)
\* ==========================================================================

\* Property: No attacks detected
NoAttacks == attackDetected = {}

\* Property: Atomicity - either all domains complete or none
Atomicity ==
    phase = "COMPLETE" =>
        \A d \in Domains: receipts[d] \in {"VALID"}

\* Property: No silent failure
NoSilentFailure ==
    "SilentFailure" \notin attackDetected

\* Property: No free option
NoFreeOption ==
    "FreeOption" \notin attackDetected

\* Property: Quorum always required
QuorumRequired ==
    phase \in {"FINALIZE", "COMPLETE"} => HasQuorum

\* Property: Grace period used at most once
GracePeriodOnce ==
    "GracePeriodAbuse" \notin attackDetected

\* Property: No forged receipts in final state
NoForgedReceipts ==
    phase = "COMPLETE" => \A d \in Domains: receipts[d] # "FORGED"

\* Property: Clock monotonicity
ClockMonotonicity ==
    "ClockRegression" \notin attackDetected

\* Property: No split-brain finality
NoSplitBrain ==
    "SplitBrain" \notin attackDetected

\* ==========================================================================
\* LIVENESS PROPERTIES
\* ==========================================================================

\* Eventually terminate (either complete, timeout, or force close)
EventualTermination ==
    <>(phase \in {"COMPLETE", "TIMEOUT", "FORCE_CLOSE"})

\* Disputes eventually resolve
DisputeResolution ==
    challengeActive ~> ~challengeActive

\* ==========================================================================
\* COUNTEREXAMPLE HUNTING INVARIANTS
\*
\* These are INTENTIONALLY designed to fail and produce counterexamples.
\* When TLC finds a violation, that's the attack vector we're hunting.
\* ==========================================================================

\* HUNT: Find a path where forged receipt leads to COMPLETE
HuntForgedReceiptSuccess ==
    ~(phase = "COMPLETE" /\ \E d \in Domains: receipts[d] = "FORGED")

\* HUNT: Find a path where swap completes without quorum
HuntQuorumBypass ==
    ~(phase = "COMPLETE" /\ ~HasQuorum)

\* HUNT: Find silent timeout path
HuntSilentTimeout ==
    ~("SilentFailure" \in attackDetected)

\* HUNT: Find free option exploitation
HuntFreeOption ==
    ~("FreeOption" \in attackDetected)

\* HUNT: Find grace period abuse
HuntGracePeriodAbuse ==
    ~("GracePeriodAbuse" \in attackDetected)

\* HUNT: Find split-brain scenario
HuntSplitBrain ==
    ~("SplitBrain" \in attackDetected)

\* ==========================================================================
\* MODEL VALUES (for TLC)
\*
\* Use these in TLC model config:
\*   Domains <- {"EVM", "SVM", "BTC", "X3"}
\*   Guardians <- {"G1", "G2", "G3", "G4", "G5"}
\*   MaxBlocks <- 20
\*   MinStake <- 1000
\*   SlashPercent <- 100
\*   ChallengeWindow <- 5
\*   GracePeriod <- 2
\* ==========================================================================

=============================================================================
