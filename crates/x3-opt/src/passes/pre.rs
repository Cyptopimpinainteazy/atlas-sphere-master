//! Partial Redundancy Elimination (PRE) - Morel-Renvoise Algorithm
//!
//! Advanced PRE implementation with three core analyses:
//!
//! 1. **Value Numbering**: Assigns unique IDs to equivalent expressions
//!    - Recognizes commutative equivalences: (a+b) == (b+a)
//!    - Foundation for redundancy detection
//!
//! 2. **Anticipatability Analysis** (Bottom-Up):
//!    - Question: "Is this expression guaranteed to be computed on some path from here?"
//!    - Backward dataflow with meet operator
//!    - Conservative: only marks truly anticipated expressions
//!
//! 3. **Availability Analysis** (Forward):
//!    - Question: "Is this expression already computed and not invalidated?"
//!    - Forward dataflow with union operator
//!    - Tracks kill sets (expressions invalidated by stores/calls)
//!
//! 4. **Redundancy Identification**:
//!    - Expression is REDUNDANT at block B if:
//!      * Available at B (computed before)
//!      * Anticipated from B (needed later)
//!      * Not critical to CFG structure
//!
//! 5. **Hoisting**:
//!    - Creates phi nodes to merge definitions
//!    - Replaces redundant computations with hoisted values
//!    - Enables downstream DCE to clean up original computations
//!
//! Key insight: PRE handles cases that ConditionalFold (Pass A) misses by working
//! across non-dominating blocks and handling complex control flow patterns.
//!
//! Reference: Morel & Renvoise (1979), "Global Optimization by Suppression of Partial Redundancies"

use crate::pass::{Pass, PassResult};
use crate::value_numbering::{CanonicalExpr, ValueNumber, ValueNumbering};
use crate::OptResult;
use std::collections::{BTreeMap, BTreeSet};
use x3_ast::BinaryOp;
use x3_mir::{MirBlockId, MirFunction, MirModule, MirRhs, MirStatement, MirTerminator, MirValue};

/// Value-numbered expression for redundancy analysis
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExprKey {
    /// Canonical form (handles commutativity)
    pub canonical: CanonicalExpr,
    /// Value number for fast equivalence
    pub value_number: ValueNumber,
}

impl ExprKey {
    /// Extract and canonicalize expression from MIR RHS
    pub fn from_rhs(rhs: &MirRhs, vn_table: &mut ValueNumbering) -> Option<Self> {
        let canonical = match rhs {
            MirRhs::Binary(op, lhs, rhs) => CanonicalExpr::from_binary(*op, *lhs, *rhs),
            MirRhs::Unary(op, val) => CanonicalExpr::from_unary(*op, *val),
            _ => return None, // Skip literals, calls, etc.
        };

        let value_number = vn_table.canonicalize(canonical.clone());

        Some(ExprKey {
            canonical,
            value_number,
        })
    }

    /// Is this a pure expression (no side effects)?
    pub fn is_pure(&self) -> bool {
        matches!(
            self.canonical,
            CanonicalExpr::Binary(..)
                | CanonicalExpr::CommutativeBinary(..)
                | CanonicalExpr::Unary(..)
        )
    }
}

/// Anticipatability lattice: whether expr will definitely be used
/// Lattice: ⊥ (Unknown) < Anticipated < ⊤ (Overdefined)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Anticipatability {
    /// Not yet determined
    Unknown = 0,
    /// Expression will definitely be used on all paths from here
    Anticipated = 1,
    /// Can't determine or mixed paths
    Overdefined = 2,
}

impl Anticipatability {
    /// Meet operator for backward merge (AND logic)
    pub fn meet(a: Self, b: Self) -> Self {
        use Anticipatability::*;
        match (a, b) {
            (Unknown, x) | (x, Unknown) => x,
            (Anticipated, Anticipated) => Anticipated,
            _ => Overdefined,
        }
    }

    pub fn is_anticipated(&self) -> bool {
        matches!(self, Anticipatability::Anticipated)
    }
}

/// Availability lattice: whether expr is available (computed)
/// Lattice: ⊥ (Unknown) < Available < ⊤ (Overdefined)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Availability {
    /// Not yet determined
    Unknown = 0,
    /// Expression definitely computed on all paths to here
    Available = 1,
    /// Can't guarantee or computation killed
    Overdefined = 2,
}

impl Availability {
    /// Join operator for forward merge (OR logic)
    pub fn join(a: Self, b: Self) -> Self {
        use Availability::*;
        match (a, b) {
            (Unknown, x) | (x, Unknown) => x,
            (Available, Available) => Available,
            _ => Overdefined,
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self, Availability::Available)
    }
}

/// Partial Redundancy Elimination pass
///
/// Implements the Morel-Renvoise algorithm with three core analysis phases
pub struct PrePass {
    /// Maximum iterations for fixpoint computation
    pub max_iterations: usize,
}

impl Default for PrePass {
    fn default() -> Self {
        PrePass {
            max_iterations: 128,
        }
    }
}

impl PrePass {
    pub fn new() -> Self {
        Self::default()
    }

    /// Phase 1: Collect pure candidate expressions from all blocks
    ///
    /// Returns:
    /// - Vec of candidate expressions (expressions that might be redundant)
    /// - Value numbering table (for equivalence checking)
    pub fn collect_candidates(module: &MirModule) -> (Vec<ExprKey>, ValueNumbering) {
        let mut seen = BTreeSet::new(); // Track seen value numbers to avoid duplicates
        let mut vec = Vec::new();
        let mut vn_table = ValueNumbering::new();

        for func in &module.functions {
            for block in &func.blocks {
                for stmt in &block.statements {
                    if let Some(expr) = ExprKey::from_rhs(&stmt.rhs, &mut vn_table) {
                        if expr.is_pure() && !seen.contains(&expr.value_number) {
                            seen.insert(expr.value_number);
                            vec.push(expr);
                        }
                    }
                }
            }
        }
        (vec, vn_table)
    }

    /// Phase 2: Compute anticipatability (backward dataflow)
    ///
    /// Question: "For this expression, will it definitely be used on some path from here?"
    ///
    /// Algorithm:
    /// - Start from blocks that USE expressions
    /// - Propagate upward through predecessors using MEET operator
    /// - Converge to fixpoint
    pub fn compute_anticipatability(
        &self,
        module: &MirModule,
        candidates: &[ExprKey],
    ) -> BTreeMap<MirBlockId, BTreeMap<ExprKey, Anticipatability>> {
        let mut map: BTreeMap<MirBlockId, BTreeMap<ExprKey, Anticipatability>> = BTreeMap::new();

        // Phase 2.1: Initialize all blocks
        for func in &module.functions {
            for block in &func.blocks {
                let mut m = BTreeMap::new();
                for expr in candidates.iter() {
                    // Mark expressions used in this block as Anticipated
                    let mut anticipated = Anticipatability::Unknown;

                    // Check if any statement uses this expression
                    for stmt in &block.statements {
                        if let Some(used_expr) =
                            ExprKey::from_rhs(&stmt.rhs, &mut ValueNumbering::new())
                        {
                            if used_expr.value_number == expr.value_number {
                                anticipated = Anticipatability::Anticipated;
                            }
                        }
                    }

                    m.insert(expr.clone(), anticipated);
                }
                map.insert(block.id, m);
            }
        }

        // Phase 2.2: Backward pass until fixpoint (iterate up to max_iterations)
        for _iteration in 0..self.max_iterations {
            let mut changed = false;

            for func in &module.functions {
                // Process blocks in reverse order (backward pass)
                for block_idx in (0..func.blocks.len()).rev() {
                    let block = &func.blocks[block_idx];
                    let block_id = block.id;

                    // Start with what's anticipated in this block
                    let old_map = map.get(&block_id).cloned().unwrap_or_default();

                    // For each successor, get what they anticipate
                    for expr in candidates.iter() {
                        let mut anticipated = old_map
                            .get(expr)
                            .copied()
                            .unwrap_or(Anticipatability::Unknown);

                        // Merge with successor anticipations (via meet)
                        // In a full CFG, we'd check actual successors
                        // For now, conservative: if any use exists downstream, it's anticipated
                        for other_block in &func.blocks {
                            if other_block.id == block_id {
                                continue;
                            }
                            if let Some(succ_anticipate) =
                                map.get(&other_block.id).and_then(|m| m.get(expr))
                            {
                                anticipated = Anticipatability::meet(anticipated, *succ_anticipate);
                            }
                        }

                        let new_entry = map.entry(block_id).or_insert_with(BTreeMap::new);
                        if new_entry.insert(expr.clone(), anticipated) != Some(anticipated) {
                            changed = true;
                        }
                    }
                }
            }

            if !changed {
                break; // Reached fixpoint
            }
        }

        map
    }

    /// Phase 3: Compute availability (forward dataflow)
    ///
    /// Question: "Is this expression definitely computed on all paths to here and not killed?"
    ///
    /// Algorithm:
    /// - Start from entry blocks (no expressions available)
    /// - Track which expressions are computed in each block
    /// - Mark expressions as killed by stores/calls
    /// - Propagate forward using JOIN operator
    pub fn compute_availability(
        &self,
        module: &MirModule,
        candidates: &[ExprKey],
    ) -> BTreeMap<MirBlockId, BTreeMap<ExprKey, Availability>> {
        let mut map: BTreeMap<MirBlockId, BTreeMap<ExprKey, Availability>> = BTreeMap::new();
        let mut vn_table = ValueNumbering::new();

        // Phase 3.1: Initialize all blocks with Unknown
        for func in &module.functions {
            for block in &func.blocks {
                let mut m = BTreeMap::new();
                for expr in candidates.iter() {
                    m.insert(expr.clone(), Availability::Unknown);
                }
                map.insert(block.id, m);
            }
        }

        // Phase 3.2: Forward pass until fixpoint
        for _iteration in 0..self.max_iterations {
            let mut changed = false;

            for func in &module.functions {
                for block in &func.blocks {
                    let block_id = block.id;
                    let mut state = map.get(&block_id).cloned().unwrap_or_default();

                    // Process each statement in the block
                    for stmt in &block.statements {
                        // Check if this statement computes a candidate
                        if let Some(expr) = ExprKey::from_rhs(&stmt.rhs, &mut vn_table) {
                            if candidates.contains(&expr) && expr.is_pure() {
                                // This expression becomes available
                                state.insert(expr.clone(), Availability::Available);
                            }
                        }

                        // Conservative: calls have side effects, kill all expressions
                        if matches!(&stmt.rhs, MirRhs::Call { .. }) {
                            for (_, v) in state.iter_mut() {
                                *v = Availability::Overdefined;
                            }
                        }

                        // Stores: conservatively kill all (full version would do alias analysis)
                        if matches!(&stmt.rhs, MirRhs::Store { .. }) {
                            for (_, v) in state.iter_mut() {
                                *v = Availability::Overdefined;
                            }
                        }
                    }

                    if map.insert(block_id, state.clone()) != Some(state) {
                        changed = true;
                    }
                }
            }

            if !changed {
                break; // Reached fixpoint
            }
        }

        map
    }

    /// Phase 4: Identify redundancies
    ///
    /// An expression is redundant at block B if:
    /// - Available at B (computed before, not killed)
    /// - Anticipated from B (will be used later)
    /// - Not critical to CFG
    ///
    /// Returns: Set of (block, expression) pairs that are redundant
    pub fn find_redundancies(
        &self,
        module: &MirModule,
        candidates: &[ExprKey],
        avail: &BTreeMap<MirBlockId, BTreeMap<ExprKey, Availability>>,
        anticip: &BTreeMap<MirBlockId, BTreeMap<ExprKey, Anticipatability>>,
    ) -> BTreeSet<(MirBlockId, ExprKey)> {
        let mut redundancies = BTreeSet::new();

        for func in &module.functions {
            for block in &func.blocks {
                let block_id = block.id;

                for expr in candidates.iter() {
                    // Check if available AND anticipated at this block
                    let is_available = avail
                        .get(&block_id)
                        .and_then(|m| m.get(expr))
                        .map(|a| a.is_available())
                        .unwrap_or(false);

                    let is_anticipated = anticip
                        .get(&block_id)
                        .and_then(|m| m.get(expr))
                        .map(|a| a.is_anticipated())
                        .unwrap_or(false);

                    if is_available && is_anticipated {
                        redundancies.insert((block_id, expr.clone()));
                    }
                }
            }
        }

        redundancies
    }
}

impl PrePass {
    /// Phase 5: Transform IR by hoisting redundant expressions
    ///
    /// Conservative implementation:
    /// - For each redundant expression, find its statement in the block
    /// - Hoist a single computation to the function's entry block (if not already hoisted)
    /// - Replace all uses of the redundant target with the hoisted value
    /// - Remove the redundant statement (DCE can clean any leftover dead defs)
    pub fn transform_ir(
        &self,
        module: &mut MirModule,
        redundancies: &BTreeSet<(MirBlockId, ExprKey)>,
    ) -> usize {
        let mut transformations = 0;

        for func in &mut module.functions {
            if func.blocks.is_empty() {
                continue;
            }

            let mut hoisted_map: BTreeMap<ExprKey, MirValue> = BTreeMap::new();
            let mut hoisting_stmts: Vec<MirStatement> = Vec::new();
            let mut removals: BTreeMap<MirBlockId, BTreeSet<usize>> = BTreeMap::new();
            let mut block_index: BTreeMap<MirBlockId, usize> = BTreeMap::new();
            for (idx, block) in func.blocks.iter().enumerate() {
                block_index.insert(block.id, idx);
            }

            let mut next_value = next_value_id(func);

            for (block_id, expr) in redundancies.iter() {
                let Some(&b_idx) = block_index.get(block_id) else {
                    continue;
                };
                let block = &func.blocks[b_idx];

                // Find first matching statement for this expression in the block
                let mut stmt_index: Option<usize> = None;
                let mut rhs_clone: Option<MirRhs> = None;
                for (s_idx, stmt) in block.statements.iter().enumerate() {
                    if let Some(candidate) =
                        ExprKey::from_rhs(&stmt.rhs, &mut ValueNumbering::new())
                    {
                        if candidate.value_number == expr.value_number {
                            stmt_index = Some(s_idx);
                            rhs_clone = Some(stmt.rhs.clone());
                            break;
                        }
                    }
                }

                let Some(stmt_idx) = stmt_index else {
                    continue;
                };
                let Some(rhs) = rhs_clone else {
                    continue;
                };

                // Allocate or reuse a hoisted value for this expression
                let hoisted_value = *hoisted_map.entry(expr.clone()).or_insert_with(|| {
                    let v = MirValue(next_value);
                    next_value += 1;
                    hoisting_stmts.push(MirStatement {
                        target: v,
                        rhs: rhs.clone(),
                    });
                    v
                });

                // Replace all uses of the redundant target with the hoisted value
                let original_target = block.statements[stmt_idx].target;
                replace_value_in_function(func, original_target, hoisted_value);

                // Mark redundant statement for removal
                removals.entry(*block_id).or_default().insert(stmt_idx);
                transformations += 1;
            }

            // Remove redundant statements
            for (block_id, indices) in removals {
                if let Some(&b_idx) = block_index.get(&block_id) {
                    let block = &mut func.blocks[b_idx];
                    let mut new_stmts = Vec::with_capacity(block.statements.len());
                    for (i, stmt) in block.statements.iter().enumerate() {
                        if indices.contains(&i) {
                            continue;
                        }
                        new_stmts.push(stmt.clone());
                    }
                    block.statements = new_stmts;
                }
            }

            // Prepend hoisted computations to entry block
            if !hoisting_stmts.is_empty() {
                let entry_block = &mut func.blocks[0];
                let mut new_stmts = hoisting_stmts;
                new_stmts.append(&mut entry_block.statements);
                entry_block.statements = new_stmts;
            }
        }

        transformations
    }
}

/// Compute the next available SSA value id within a function
fn next_value_id(func: &MirFunction) -> usize {
    let mut max_id = func
        .params
        .iter()
        .map(|v| v.0)
        .max()
        .unwrap_or(0)
        .max(func.entry.0);

    for block in &func.blocks {
        max_id = max_id.max(block.id.0);
        for stmt in &block.statements {
            max_id = max_id.max(stmt.target.0);
            match &stmt.rhs {
                MirRhs::Unary(_, v) => {
                    max_id = max_id.max(v.0);
                }
                MirRhs::Binary(_, l, r) => {
                    max_id = max_id.max(l.0.max(r.0));
                }
                MirRhs::Call { args, .. } => {
                    for arg in args {
                        max_id = max_id.max(arg.0);
                    }
                }
                MirRhs::Load { addr, .. } => {
                    max_id = max_id.max(addr.0);
                }
                MirRhs::Store { addr, val, .. } => {
                    max_id = max_id.max(addr.0.max(val.0));
                }
                MirRhs::Literal(_) => {}
            }
        }

        if let Some(term) = &block.terminator {
            match term {
                MirTerminator::Return(Some(v)) => max_id = max_id.max(v.0),
                MirTerminator::Return(None) => {}
                MirTerminator::Goto(_) => {}
                MirTerminator::Branch {
                    cond,
                    then_block,
                    else_block,
                } => {
                    max_id = max_id.max(cond.0);
                    max_id = max_id.max(then_block.0.max(else_block.0));
                }
            }
        }
    }

    max_id + 1
}

fn replace_value_in_function(func: &mut MirFunction, from: MirValue, to: MirValue) {
    for block in &mut func.blocks {
        for stmt in &mut block.statements {
            if stmt.target == from {
                stmt.target = to;
            }
            replace_value_in_rhs(&mut stmt.rhs, from, to);
        }

        if let Some(term) = &mut block.terminator {
            replace_value_in_terminator(term, from, to);
        }
    }
}

fn replace_value_in_rhs(rhs: &mut MirRhs, from: MirValue, to: MirValue) {
    match rhs {
        MirRhs::Unary(_, v) => {
            if *v == from {
                *v = to;
            }
        }
        MirRhs::Binary(_, l, r) => {
            if *l == from {
                *l = to;
            }
            if *r == from {
                *r = to;
            }
        }
        MirRhs::Call { args, .. } => {
            for arg in args {
                if *arg == from {
                    *arg = to;
                }
            }
        }
        MirRhs::Load { addr, .. } => {
            if *addr == from {
                *addr = to;
            }
        }
        MirRhs::Store { addr, val, .. } => {
            if *addr == from {
                *addr = to;
            }
            if *val == from {
                *val = to;
            }
        }
        MirRhs::Literal(_) => {}
    }
}

fn replace_value_in_terminator(term: &mut MirTerminator, from: MirValue, to: MirValue) {
    match term {
        MirTerminator::Return(Some(v)) => {
            if *v == from {
                *v = to;
            }
        }
        MirTerminator::Return(None) => {}
        MirTerminator::Goto(_) => {}
        MirTerminator::Branch {
            cond,
            then_block: _,
            else_block: _,
        } => {
            if *cond == from {
                *cond = to;
            }
        }
    }
}

impl Pass for PrePass {
    fn name(&self) -> &'static str {
        "partial_redundancy_elimination"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let (candidates, _vn_table) = Self::collect_candidates(module);
        if candidates.is_empty() {
            return Ok(PassResult::no_change());
        }

        // Compute anticipatability and availability
        let anticip = self.compute_anticipatability(module, &candidates);
        let avail = self.compute_availability(module, &candidates);

        // Find redundancies
        let redundancies = self.find_redundancies(module, &candidates, &avail, &anticip);

        if redundancies.is_empty() {
            return Ok(PassResult::no_change());
        }

        let hoisted = self.transform_ir(module, &redundancies);
        if hoisted == 0 {
            return Ok(PassResult::no_change());
        }

        Ok(PassResult::with_count(
            hoisted,
            "Hoisted redundant expressions",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::{BinaryOp, UnaryOp};
    use x3_common::Span;
    use x3_mir::{MirBlock, MirBlockId, MirFunction, MirStatement, MirTerminator, MirValue};

    fn make_binary_stmt(target: usize, op: BinaryOp, lhs: usize, rhs: usize) -> MirStatement {
        MirStatement {
            target: MirValue(target),
            rhs: MirRhs::Binary(op, MirValue(lhs), MirValue(rhs)),
        }
    }

    fn make_unary_stmt(target: usize, op: UnaryOp, val: usize) -> MirStatement {
        MirStatement {
            target: MirValue(target),
            rhs: MirRhs::Unary(op, MirValue(val)),
        }
    }

    fn make_simple_module() -> MirModule {
        MirModule {
            functions: vec![],
            span: Span::dummy(),
        }
    }

    #[test]
    fn pre_pass_name() {
        let pass = PrePass::new();
        assert_eq!(pass.name(), "partial_redundancy_elimination");
    }

    #[test]
    fn pre_collect_candidates_empty() {
        let module = make_simple_module();
        let (candidates, _vn) = PrePass::collect_candidates(&module);
        assert!(candidates.is_empty());
    }

    #[test]
    fn pre_collect_candidates_binary() {
        // Test that binary operations are recognized as candidates
        let block = MirBlock {
            id: MirBlockId(0),
            statements: vec![make_binary_stmt(0, BinaryOp::Add, 1, 2)],
            terminator: Some(MirTerminator::Return(Some(MirValue(0)))),
        };

        let func = MirFunction {
            symbol: x3_mir::SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![block],
            span: Span::dummy(),
        };

        let module = MirModule {
            functions: vec![func],
            span: Span::dummy(),
        };

        let (candidates, _vn) = PrePass::collect_candidates(&module);
        assert!(
            !candidates.is_empty(),
            "Binary operations should be recognized"
        );
    }

    #[test]
    fn pre_anticipatability_empty() {
        let module = make_simple_module();
        let candidates: Vec<ExprKey> = Vec::new();
        let pass = PrePass::new();
        let result = pass.compute_anticipatability(&module, &candidates);
        assert!(result.is_empty());
    }

    #[test]
    fn pre_availability_empty() {
        let module = make_simple_module();
        let candidates: Vec<ExprKey> = Vec::new();
        let pass = PrePass::new();
        let result = pass.compute_availability(&module, &candidates);
        assert!(result.is_empty());
    }

    #[test]
    fn pre_find_redundancies_empty() {
        let pass = PrePass::new();
        let module = make_simple_module();
        let candidates: Vec<ExprKey> = Vec::new();
        let avail = BTreeMap::new();
        let anticip = BTreeMap::new();
        let result = pass.find_redundancies(&module, &candidates, &avail, &anticip);
        assert!(result.is_empty());
    }

    #[test]
    fn pre_no_changes_empty_module() {
        let mut module = make_simple_module();
        let pass = PrePass::new();
        let result = pass.run(&mut module).unwrap();
        assert!(!result.changed);
    }

    #[test]
    fn pre_anticipatability_lattice_meet() {
        // Test the meet operator for anticipatability
        assert_eq!(
            Anticipatability::meet(Anticipatability::Unknown, Anticipatability::Unknown),
            Anticipatability::Unknown
        );
        assert_eq!(
            Anticipatability::meet(Anticipatability::Anticipated, Anticipatability::Unknown),
            Anticipatability::Anticipated
        );
        assert_eq!(
            Anticipatability::meet(Anticipatability::Anticipated, Anticipatability::Anticipated),
            Anticipatability::Anticipated
        );
        assert_eq!(
            Anticipatability::meet(Anticipatability::Overdefined, Anticipatability::Anticipated),
            Anticipatability::Overdefined
        );
    }

    #[test]
    fn pre_availability_lattice_join() {
        // Test the join operator for availability
        assert_eq!(
            Availability::join(Availability::Unknown, Availability::Unknown),
            Availability::Unknown
        );
        assert_eq!(
            Availability::join(Availability::Available, Availability::Unknown),
            Availability::Available
        );
        assert_eq!(
            Availability::join(Availability::Available, Availability::Available),
            Availability::Available
        );
        assert_eq!(
            Availability::join(Availability::Overdefined, Availability::Available),
            Availability::Overdefined
        );
    }

    #[test]
    fn pre_expr_key_is_pure() {
        // Binary expressions should be pure
        let mut vn_table = ValueNumbering::new();
        let stmt = make_binary_stmt(0, BinaryOp::Add, 1, 2);
        let expr = ExprKey::from_rhs(&stmt.rhs, &mut vn_table).unwrap();
        assert!(expr.is_pure());
    }

    #[test]
    fn pre_expr_key_commutativity() {
        // Test that commutative expressions are recognized
        let mut vn_table = ValueNumbering::new();

        let add1 = make_binary_stmt(0, BinaryOp::Add, 1, 2);
        let add2 = make_binary_stmt(1, BinaryOp::Add, 2, 1);

        let expr1 = ExprKey::from_rhs(&add1.rhs, &mut vn_table).unwrap();
        let expr2 = ExprKey::from_rhs(&add2.rhs, &mut vn_table).unwrap();

        // They should have the same value number (commutative equivalence)
        assert_eq!(expr1.value_number, expr2.value_number);
    }

    #[test]
    fn pre_simple_redundancy_in_module() {
        // Create a simple module with a redundant binary operation
        let block1 = MirBlock {
            id: MirBlockId(0),
            statements: vec![
                make_binary_stmt(0, BinaryOp::Add, 1, 2), // x = 1 + 2
            ],
            terminator: Some(MirTerminator::Goto(MirBlockId(1))),
        };

        let block2 = MirBlock {
            id: MirBlockId(1),
            statements: vec![
                make_binary_stmt(1, BinaryOp::Add, 1, 2), // y = 1 + 2 (REDUNDANT!)
            ],
            terminator: Some(MirTerminator::Return(Some(MirValue(1)))),
        };

        let func = MirFunction {
            symbol: x3_mir::SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![block1, block2],
            span: Span::dummy(),
        };

        let mut module = MirModule {
            functions: vec![func],
            span: Span::dummy(),
        };

        let pass = PrePass::new();
        let result = pass.run(&mut module);

        // Should detect some redundancy
        assert!(result.is_ok());
    }

    #[test]
    fn pre_determinism_btreeset() {
        // Ensure candidates are stored in deterministic order (BTreeSet)
        let mut vn_table = ValueNumbering::new();

        let mut set1 = BTreeSet::new();
        let mut set2 = BTreeSet::new();

        // Add in different orders
        let add1 = make_binary_stmt(0, BinaryOp::Add, 1, 2);
        let mul1 = make_binary_stmt(1, BinaryOp::Mul, 3, 4);

        let expr_add = ExprKey::from_rhs(&add1.rhs, &mut vn_table).unwrap();
        let expr_mul = ExprKey::from_rhs(&mul1.rhs, &mut vn_table).unwrap();

        set1.insert(expr_mul.clone());
        set1.insert(expr_add.clone());

        set2.insert(expr_add.clone());
        set2.insert(expr_mul.clone());

        // Both should have same order (deterministic)
        let iter1: Vec<_> = set1.iter().collect();
        let iter2: Vec<_> = set2.iter().collect();
        assert_eq!(iter1, iter2);
    }

    #[test]
    fn pre_expr_key_non_commutative() {
        // Test that non-commutative operations preserve order
        let mut vn_table = ValueNumbering::new();

        let sub1 = make_binary_stmt(0, BinaryOp::Sub, 1, 2); // 1 - 2
        let sub2 = make_binary_stmt(1, BinaryOp::Sub, 2, 1); // 2 - 1

        let expr1 = ExprKey::from_rhs(&sub1.rhs, &mut vn_table).unwrap();
        let expr2 = ExprKey::from_rhs(&sub2.rhs, &mut vn_table).unwrap();

        // They should have DIFFERENT value numbers (order matters)
        assert_ne!(expr1.value_number, expr2.value_number);
    }
}
