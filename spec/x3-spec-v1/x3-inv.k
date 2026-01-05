// X3 Invariant DSL - K Framework Semantic Specification
//
// This file defines the formal semantics of the X3 Invariant DSL
// using the K Framework. It ensures that invariant evaluation
// is mathematically precise and verifiable.
//
// Proof Goals:
// 1. DSL evaluation matches formal semantics
// 2. No invariant evaluates inconsistently
// 3. No receipt proof bypasses verification rules

requires "domains.md"

module X3-SYNTAX
    imports DOMAINS-SYNTAX

    // =========================================================================
    // EXPRESSIONS
    // =========================================================================

    syntax Expr ::= "all" "(" Exprs ")"
                  | "any" "(" Exprs ")"
                  | "not" "(" Expr ")"
                  | Pred
                  | Bool

    syntax Exprs ::= List{Expr, ","}

    // =========================================================================
    // PREDICATES
    // =========================================================================

    syntax Pred ::= Value "==" Value
                  | Value "<" Value
                  | Value ">=" Value
                  | Id "(" Values ")"

    syntax Values ::= List{Value, ","}

    // =========================================================================
    // VALUES
    // =========================================================================

    syntax Value ::= Int
                   | Id
                   | "ctx" "." Id

    // =========================================================================
    // ACTIONS
    // =========================================================================

    syntax Action ::= "revert" | "slash" | "halt" | "noop"

    // =========================================================================
    // INVARIANT DEFINITIONS
    // =========================================================================

    syntax Invariant ::= "invariant" Id ":" Expr "=>" "action" Action

    syntax Program ::= List{Invariant, ""}

endmodule

module X3-CONFIGURATION
    imports X3-SYNTAX
    imports DOMAINS

    // =========================================================================
    // CONFIGURATION
    // =========================================================================

    configuration
        <T>
            <k> $PGM:Program </k>
            <store> .Map </store>
            <context>
                <deadline> 0 </deadline>
                <amount> 0 </amount>
                <phase> 0 </phase>
                <currentBlock> 0 </currentBlock>
            </context>
            <gas> 0 </gas>
            <result> .K </result>
            <invariants> .Map </invariants>
        </T>

endmodule

module X3-SEMANTICS
    imports X3-CONFIGURATION

    // =========================================================================
    // EXPRESSION EVALUATION
    // =========================================================================

    // All: conjunction of all expressions
    rule <k> all(E:Expr, Es:Exprs) => E ~> all(Es) ... </k>
    rule <k> true ~> all(Es:Exprs) => all(Es) ... </k>
    rule <k> false ~> all(_) => false ... </k>
    rule <k> all(.Exprs) => true ... </k>

    // Any: disjunction of all expressions
    rule <k> any(E:Expr, Es:Exprs) => E ~> any(Es) ... </k>
    rule <k> false ~> any(Es:Exprs) => any(Es) ... </k>
    rule <k> true ~> any(_) => true ... </k>
    rule <k> any(.Exprs) => false ... </k>

    // Not: negation
    rule <k> not(E:Expr) => E ~> notK ... </k>
    rule <k> true ~> notK => false ... </k>
    rule <k> false ~> notK => true ... </k>

    syntax KItem ::= "notK"

    // =========================================================================
    // PREDICATE EVALUATION
    // =========================================================================

    // Equality comparison
    rule <k> V1:Int == V2:Int => V1 ==Int V2 ... </k>

    // Less than comparison
    rule <k> V1:Int < V2:Int => V1 <Int V2 ... </k>

    // Greater or equal comparison
    rule <k> V1:Int >= V2:Int => V1 >=Int V2 ... </k>

    // =========================================================================
    // VALUE EVALUATION
    // =========================================================================

    // Context field lookup
    rule <k> ctx . deadline => D ... </k>
         <deadline> D </deadline>

    rule <k> ctx . amount => A ... </k>
         <amount> A </amount>

    rule <k> ctx . phase => P ... </k>
         <phase> P </phase>

    rule <k> ctx . currentBlock => B ... </k>
         <currentBlock> B </currentBlock>

    // Variable lookup
    rule <k> X:Id => V ... </k>
         <store> X |-> V ... </store>

    // =========================================================================
    // INVARIANT PROCESSING
    // =========================================================================

    // Register invariant
    rule <k> invariant Name : E => action A
             => . ... </k>
         <invariants> M => M[Name <- invariantDef(E, A)] </invariants>

    syntax InvariantDef ::= "invariantDef" "(" Expr "," Action ")"

    // =========================================================================
    // GAS ACCOUNTING (Deterministic Cost)
    // =========================================================================

    // Each operation costs 1 gas unit
    rule <k> all(_, _) => all(_, _) ... </k>
         <gas> G => G +Int 1 </gas>
      [priority(100)]

    rule <k> any(_, _) => any(_, _) ... </k>
         <gas> G => G +Int 1 </gas>
      [priority(100)]

    rule <k> not(_) => not(_) ... </k>
         <gas> G => G +Int 1 </gas>
      [priority(100)]

    rule <k> _ == _ => _ == _ ... </k>
         <gas> G => G +Int 2 </gas>
      [priority(100)]

    rule <k> _ < _ => _ < _ ... </k>
         <gas> G => G +Int 2 </gas>
      [priority(100)]

    rule <k> _ >= _ => _ >= _ ... </k>
         <gas> G => G +Int 2 </gas>
      [priority(100)]

endmodule

module X3-VERIFICATION
    imports X3-SEMANTICS

    // =========================================================================
    // VERIFICATION CONDITIONS
    // =========================================================================

    // Property: All invariant evaluations are total
    // Every well-formed expression produces a boolean result
    claim <k> E:Expr => B:Bool </k>
          <context> _ </context>
          <gas> _ </gas>
      requires isWellFormed(E)

    // Property: Evaluation is deterministic
    // Same context + same expression => same result
    claim <k> E => B1 </k>
          <context> C </context>
       /\ <k> E => B2 </k>
          <context> C </context>
       => B1 ==K B2

    // Property: Gas consumption is bounded
    // No expression can consume more than maxGas
    claim <k> E:Expr => _ </k>
          <gas> G1 => G2 </gas>
      ensures G2 -Int G1 <=Int maxGas(E)

    // =========================================================================
    // HELPER FUNCTIONS
    // =========================================================================

    syntax Bool ::= "isWellFormed" "(" Expr ")"  [function]
    rule isWellFormed(all(Es)) => isWellFormedList(Es)
    rule isWellFormed(any(Es)) => isWellFormedList(Es)
    rule isWellFormed(not(E)) => isWellFormed(E)
    rule isWellFormed(_:Pred) => true
    rule isWellFormed(_:Bool) => true

    syntax Bool ::= "isWellFormedList" "(" Exprs ")"  [function]
    rule isWellFormedList(.Exprs) => true
    rule isWellFormedList(E, Es) => isWellFormed(E) andBool isWellFormedList(Es)

    syntax Int ::= "maxGas" "(" Expr ")"  [function]
    rule maxGas(all(Es)) => 1 +Int maxGasList(Es)
    rule maxGas(any(Es)) => 1 +Int maxGasList(Es)
    rule maxGas(not(E)) => 1 +Int maxGas(E)
    rule maxGas(_ == _) => 2
    rule maxGas(_ < _) => 2
    rule maxGas(_ >= _) => 2
    rule maxGas(_:Bool) => 0

    syntax Int ::= "maxGasList" "(" Exprs ")"  [function]
    rule maxGasList(.Exprs) => 0
    rule maxGasList(E, Es) => maxGas(E) +Int maxGasList(Es)

endmodule

module X3-RECEIPT-VERIFICATION
    imports X3-SEMANTICS

    // =========================================================================
    // RECEIPT PROOF VERIFICATION RULES
    // =========================================================================

    syntax ReceiptProof ::= "evmProof" "(" Int "," Int "," Bytes ")"     // confirmations, block, merkleProof
                         | "svmProof" "(" Int "," Int ")"               // slot, commitment
                         | "btcProof" "(" Int "," Int "," Bytes ")"     // confirmations, txid, merkleProof
                         | "x3Proof" "(" Int "," Int "," Bytes ")"      // validators, weight, signatures

    syntax Bool ::= "verifyProof" "(" ReceiptProof ")"  [function]

    // EVM: verify confirmations >= minimum
    rule verifyProof(evmProof(C, _, _)) => C >=Int 12

    // SVM: verify finalized commitment
    rule verifyProof(svmProof(_, Commitment)) => Commitment ==Int 2  // 2 = Finalized

    // BTC: verify confirmations >= minimum
    rule verifyProof(btcProof(C, _, _)) => C >=Int 6

    // X3: verify quorum (>2/3)
    rule verifyProof(x3Proof(Validators, Weight, _)) =>
        Weight *Int 3 >Int Validators *Int 2

    // =========================================================================
    // NO RECEIPT PROOF BYPASS
    // =========================================================================

    // Property: Valid proof is required for release
    // A domain cannot release without a valid proof
    claim <k> releaseDomain(D) => . </k>
          <receipts> D |-> Proof ... </receipts>
      requires verifyProof(Proof)

endmodule
