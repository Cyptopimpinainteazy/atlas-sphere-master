----------------------------- MODULE X3EconomicSecurity -----------------------------
(*
 * X3 Economic Security - TLA+ Counterexample Search
 *
 * PURPOSE: Hunt for economic attack vectors and incentive failures.
 *
 * This specification searches for:
 *   - Rational adversary profit conditions
 *   - Collusion threshold breaches
 *   - Slashing insufficiency
 *   - Griefing attacks
 *   - MEV extraction paths
 *   - Stake manipulation
 *
 * Constitutional Section VII: Economic Security Model
 *)

EXTENDS Integers, Sequences, FiniteSets, TLC

\* ==========================================================================
\* CONSTANTS
\* ==========================================================================

CONSTANTS
    Actors,             \* Set of participants (guardians + attackers)
    SwapValue,          \* Value at risk in the swap
    MinStakeRatio,      \* Minimum stake as ratio of SwapValue (e.g., 10 = 10%)
    SlashRates,         \* Record of violation type -> slash percent
    DetectionProb,      \* Probability of attack detection (0-100)
    CollusionThreshold, \* Number of actors needed to collude
    MaxRounds           \* Simulation rounds

ASSUME Cardinality(Actors) >= 3
ASSUME SwapValue > 0
ASSUME MinStakeRatio > 0 /\ MinStakeRatio <= 100
ASSUME DetectionProb >= 0 /\ DetectionProb <= 100
ASSUME CollusionThreshold >= 1 /\ CollusionThreshold <= Cardinality(Actors)
ASSUME MaxRounds > 0

\* ==========================================================================
\* DERIVED CONSTANTS
\* ==========================================================================

MinStake == (SwapValue * MinStakeRatio) \div 100

\* Violation types and their slash rates
ViolationTypes == {"DeterminismViolation", "ReceiptForgery", "TimeoutNegligence",
                   "InvalidEscalation", "CollusionAttempt"}

\* ==========================================================================
\* VARIABLES
\* ==========================================================================

VARIABLES
    round,              \* Current simulation round

    \* Actor State
    stakes,             \* Actor -> current stake
    reputation,         \* Actor -> reputation score
    colluding,          \* Set of actors currently colluding

    \* Economic State
    protocolFees,       \* Accumulated protocol fees
    slashPool,          \* Slashed funds pool
    rewardPool,         \* Rewards waiting to be distributed

    \* Attack State
    attackInProgress,   \* Type of attack in progress (or NONE)
    attackValue,        \* Potential gain from attack
    attackCost,         \* Cost incurred by attacker
    attackDetected,     \* Was the attack detected?

    \* Metrics
    successfulAttacks,  \* Count of successful attacks
    failedAttacks,      \* Count of failed attacks
    totalSlashed,       \* Total amount slashed

    \* Economic Invariant Violations
    economicViolations  \* Set of detected violations

vars == <<round, stakes, reputation, colluding, protocolFees, slashPool,
          rewardPool, attackInProgress, attackValue, attackCost, attackDetected,
          successfulAttacks, failedAttacks, totalSlashed, economicViolations>>

\* ==========================================================================
\* TYPE INVARIANTS
\* ==========================================================================

TypeOK ==
    /\ round \in 0..MaxRounds
    /\ stakes \in [Actors -> 0..SwapValue*10]
    /\ reputation \in [Actors -> 0..1000]
    /\ colluding \subseteq Actors
    /\ protocolFees \in 0..SwapValue*10
    /\ slashPool \in 0..SwapValue*100
    /\ rewardPool \in 0..SwapValue*10
    /\ attackInProgress \in {"NONE"} \cup ViolationTypes
    /\ attackValue \in 0..SwapValue*2
    /\ attackCost \in 0..SwapValue*10
    /\ attackDetected \in BOOLEAN
    /\ successfulAttacks \in 0..MaxRounds
    /\ failedAttacks \in 0..MaxRounds
    /\ totalSlashed \in 0..SwapValue*100
    /\ economicViolations \subseteq {"ProfitableAttack", "InsufficientSlash",
                                      "CollusionProfit", "GriefingViable",
                                      "MEVExtraction", "RentSeeking"}

\* ==========================================================================
\* INITIAL STATE
\* ==========================================================================

Init ==
    /\ round = 0
    /\ stakes = [a \in Actors |-> MinStake + (SwapValue \div 10)]  \* Start with buffer
    /\ reputation = [a \in Actors |-> 500]  \* Neutral reputation
    /\ colluding = {}
    /\ protocolFees = 0
    /\ slashPool = 0
    /\ rewardPool = SwapValue \div 20  \* 5% of swap value as rewards
    /\ attackInProgress = "NONE"
    /\ attackValue = 0
    /\ attackCost = 0
    /\ attackDetected = FALSE
    /\ successfulAttacks = 0
    /\ failedAttacks = 0
    /\ totalSlashed = 0
    /\ economicViolations = {}

\* ==========================================================================
\* HELPER FUNCTIONS
\* ==========================================================================

\* Calculate expected value of attack
ExpectedGain(value, detection) ==
    (value * (100 - detection)) \div 100

\* Calculate expected loss from detection
ExpectedLoss(stake, slashRate, detection) ==
    (stake * slashRate * detection) \div 10000

\* Get slash rate for violation type
GetSlashRate(violation) ==
    CASE violation = "DeterminismViolation" -> 100
      [] violation = "ReceiptForgery" -> 100
      [] violation = "TimeoutNegligence" -> 20
      [] violation = "InvalidEscalation" -> 50
      [] violation = "CollusionAttempt" -> 100
      [] OTHER -> 10

\* Is attack economically rational?
IsRational(gain, loss) == gain > loss

\* Total colluding stake
ColludingStake ==
    LET activeColluders == {a \in colluding : stakes[a] >= MinStake}
    IN IF Cardinality(activeColluders) = 0 THEN 0
       ELSE \* Sum stakes - simplified for TLA+
           Cardinality(activeColluders) * MinStake

\* ==========================================================================
\* ATTACK ACTIONS
\* ==========================================================================

\* Start an attack attempt
StartAttack(attacker, violation) ==
    /\ attackInProgress = "NONE"
    /\ attacker \in Actors
    /\ stakes[attacker] >= MinStake
    /\ attackInProgress' = violation
    /\ attackValue' = SwapValue  \* Potential gain is full swap value
    /\ attackCost' = stakes[attacker]  \* Risk is full stake
    /\ attackDetected' = FALSE
    /\ UNCHANGED <<round, stakes, reputation, colluding, protocolFees,
                   slashPool, rewardPool, successfulAttacks, failedAttacks,
                   totalSlashed, economicViolations>>

\* Detection occurs (probabilistic in real system, deterministic for model)
DetectAttack ==
    /\ attackInProgress # "NONE"
    /\ ~attackDetected
    /\ attackDetected' = TRUE
    /\ UNCHANGED <<round, stakes, reputation, colluding, protocolFees,
                   slashPool, rewardPool, attackInProgress, attackValue,
                   attackCost, successfulAttacks, failedAttacks, totalSlashed,
                   economicViolations>>

\* Attack succeeds (undetected)
AttackSucceeds(attacker) ==
    /\ attackInProgress # "NONE"
    /\ ~attackDetected
    /\ attacker \in Actors
    \* Attacker gains swap value
    /\ successfulAttacks' = successfulAttacks + 1
    \* Check if this is a profitable attack
    /\ LET gain == ExpectedGain(attackValue, DetectionProb)
           loss == ExpectedLoss(stakes[attacker],
                               GetSlashRate(attackInProgress), DetectionProb)
       IN IF gain > loss
          THEN economicViolations' = economicViolations \cup {"ProfitableAttack"}
          ELSE economicViolations' = economicViolations
    /\ attackInProgress' = "NONE"
    /\ attackValue' = 0
    /\ attackCost' = 0
    /\ UNCHANGED <<round, stakes, reputation, colluding, protocolFees,
                   slashPool, rewardPool, attackDetected, failedAttacks,
                   totalSlashed>>

\* Attack fails (detected, slashing occurs)
AttackFails(attacker) ==
    /\ attackInProgress # "NONE"
    /\ attackDetected
    /\ attacker \in Actors
    /\ LET slashRate == GetSlashRate(attackInProgress)
           slashAmount == (stakes[attacker] * slashRate) \div 100
       IN /\ stakes' = [stakes EXCEPT ![attacker] = @ - slashAmount]
          /\ slashPool' = slashPool + slashAmount
          /\ totalSlashed' = totalSlashed + slashAmount
          \* Check if slashing is sufficient deterrent
          /\ IF slashAmount < attackValue
             THEN economicViolations' = economicViolations \cup {"InsufficientSlash"}
             ELSE economicViolations' = economicViolations
    /\ failedAttacks' = failedAttacks + 1
    /\ reputation' = [reputation EXCEPT ![attacker] = @ - 100]
    /\ attackInProgress' = "NONE"
    /\ attackValue' = 0
    /\ attackCost' = 0
    /\ attackDetected' = FALSE
    /\ UNCHANGED <<round, colluding, protocolFees, rewardPool, successfulAttacks>>

\* ==========================================================================
\* COLLUSION ACTIONS
\* ==========================================================================

\* Actor joins collusion group
JoinCollusion(actor) ==
    /\ actor \in Actors
    /\ actor \notin colluding
    /\ Cardinality(colluding) < CollusionThreshold
    /\ colluding' = colluding \cup {actor}
    /\ UNCHANGED <<round, stakes, reputation, protocolFees, slashPool,
                   rewardPool, attackInProgress, attackValue, attackCost,
                   attackDetected, successfulAttacks, failedAttacks,
                   totalSlashed, economicViolations>>

\* Collusion threshold reached - coordinated attack
CollusionAttack ==
    /\ Cardinality(colluding) >= CollusionThreshold
    /\ attackInProgress = "NONE"
    /\ LET totalStake == ColludingStake
       IN /\ attackInProgress' = "CollusionAttempt"
          /\ attackValue' = SwapValue * 2  \* Higher gain from coordination
          /\ attackCost' = totalStake
          \* Check if collusion is profitable
          /\ IF (SwapValue * 2) > totalStake
             THEN economicViolations' = economicViolations \cup {"CollusionProfit"}
             ELSE economicViolations' = economicViolations
    /\ attackDetected' = FALSE
    /\ UNCHANGED <<round, stakes, reputation, colluding, protocolFees,
                   slashPool, rewardPool, successfulAttacks, failedAttacks,
                   totalSlashed>>

\* ==========================================================================
\* GRIEFING ATTACK
\* ==========================================================================

\* Griefing: attacker loses money but causes greater loss to victim
GriefingAttack(attacker, victim) ==
    /\ attacker \in Actors
    /\ victim \in Actors
    /\ attacker # victim
    /\ stakes[attacker] >= MinStake
    /\ stakes[victim] >= MinStake
    \* Griefing ratio: victim loss / attacker loss
    /\ LET attackerLoss == stakes[attacker]
           victimLoss == stakes[victim] * 2  \* Victim loses more
       IN IF victimLoss > attackerLoss
          THEN economicViolations' = economicViolations \cup {"GriefingViable"}
          ELSE economicViolations' = economicViolations
    /\ UNCHANGED <<round, stakes, reputation, colluding, protocolFees,
                   slashPool, rewardPool, attackInProgress, attackValue,
                   attackCost, attackDetected, successfulAttacks, failedAttacks,
                   totalSlashed>>

\* ==========================================================================
\* MEV EXTRACTION
\* ==========================================================================

\* MEV extraction opportunity detected
MEVExtraction(extractor) ==
    /\ extractor \in Actors
    /\ round > 0
    \* MEV exists when timing matters
    /\ LET mevValue == SwapValue \div 100  \* 1% MEV opportunity
       IN IF mevValue > 0
          THEN economicViolations' = economicViolations \cup {"MEVExtraction"}
          ELSE economicViolations' = economicViolations
    /\ UNCHANGED <<round, stakes, reputation, colluding, protocolFees,
                   slashPool, rewardPool, attackInProgress, attackValue,
                   attackCost, attackDetected, successfulAttacks, failedAttacks,
                   totalSlashed>>

\* ==========================================================================
\* LEGITIMATE ACTIONS
\* ==========================================================================

\* Advance round
AdvanceRound ==
    /\ round < MaxRounds
    /\ round' = round + 1
    /\ UNCHANGED <<stakes, reputation, colluding, protocolFees, slashPool,
                   rewardPool, attackInProgress, attackValue, attackCost,
                   attackDetected, successfulAttacks, failedAttacks,
                   totalSlashed, economicViolations>>

\* Stake increase (legitimate)
IncreaseStake(actor, amount) ==
    /\ actor \in Actors
    /\ amount > 0
    /\ amount <= SwapValue
    /\ stakes' = [stakes EXCEPT ![actor] = @ + amount]
    /\ UNCHANGED <<round, reputation, colluding, protocolFees, slashPool,
                   rewardPool, attackInProgress, attackValue, attackCost,
                   attackDetected, successfulAttacks, failedAttacks,
                   totalSlashed, economicViolations>>

\* Collect rewards
CollectReward(actor) ==
    /\ actor \in Actors
    /\ rewardPool > 0
    /\ reputation[actor] >= 400  \* Must have decent reputation
    /\ LET reward == rewardPool \div Cardinality(Actors)
       IN /\ rewardPool' = rewardPool - reward
          /\ stakes' = [stakes EXCEPT ![actor] = @ + reward]
    /\ UNCHANGED <<round, reputation, colluding, protocolFees, slashPool,
                   attackInProgress, attackValue, attackCost, attackDetected,
                   successfulAttacks, failedAttacks, totalSlashed,
                   economicViolations>>

\* ==========================================================================
\* NEXT STATE
\* ==========================================================================

Next ==
    \/ AdvanceRound
    \/ \E a \in Actors, v \in ViolationTypes: StartAttack(a, v)
    \/ DetectAttack
    \/ \E a \in Actors: AttackSucceeds(a)
    \/ \E a \in Actors: AttackFails(a)
    \/ \E a \in Actors: JoinCollusion(a)
    \/ CollusionAttack
    \/ \E a, v \in Actors: a # v => GriefingAttack(a, v)
    \/ \E a \in Actors: MEVExtraction(a)
    \/ \E a \in Actors, amt \in 1..SwapValue: IncreaseStake(a, amt)
    \/ \E a \in Actors: CollectReward(a)

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

\* ==========================================================================
\* ECONOMIC INVARIANTS (Should FAIL to reveal attack vectors)
\* ==========================================================================

\* VII.2.a: Attack must be unprofitable
AttackUnprofitable ==
    "ProfitableAttack" \notin economicViolations

\* VII.2: Slashing must exceed potential gain
SlashingSufficient ==
    "InsufficientSlash" \notin economicViolations

\* Collusion must be unprofitable
CollusionUnprofitable ==
    "CollusionProfit" \notin economicViolations

\* Griefing must be unviable
GriefingUnviable ==
    "GriefingViable" \notin economicViolations

\* MEV must be eliminated or bounded
MEVEliminated ==
    "MEVExtraction" \notin economicViolations

\* ==========================================================================
\* SAFETY PROPERTIES
\* ==========================================================================

\* Total stake in system never goes negative
NonNegativeStakes ==
    \A a \in Actors: stakes[a] >= 0

\* Slashing is bounded by stake
SlashingBounded ==
    totalSlashed <= Cardinality(Actors) * (MinStake + SwapValue)

\* ==========================================================================
\* LIVENESS PROPERTIES
\* ==========================================================================

\* Eventually all attacks are resolved
AttacksResolved ==
    <>(attackInProgress = "NONE")

\* ==========================================================================
\* COUNTEREXAMPLE HUNTING
\* ==========================================================================

\* HUNT: Find profitable attack vector
HuntProfitableAttack ==
    ~("ProfitableAttack" \in economicViolations)

\* HUNT: Find collusion profit scenario
HuntCollusionProfit ==
    ~("CollusionProfit" \in economicViolations)

\* HUNT: Find griefing scenario
HuntGriefing ==
    ~("GriefingViable" \in economicViolations)

\* HUNT: Find successful attack with low slashing
HuntLowSlash ==
    ~(successfulAttacks > 0 /\ totalSlashed < SwapValue)

\* ==========================================================================
\* MODEL VALUES (for TLC)
\*
\*   Actors <- {"A1", "A2", "A3", "A4", "A5"}
\*   SwapValue <- 10000
\*   MinStakeRatio <- 10 (10%)
\*   SlashRates <- [DeterminismViolation |-> 100, ReceiptForgery |-> 100, ...]
\*   DetectionProb <- 90 (90%)
\*   CollusionThreshold <- 3
\*   MaxRounds <- 10
\* ==========================================================================

=============================================================================
