//! Constant Folding Pass
//!
//! Evaluates constant expressions at compile time, replacing operations on
//! known constants with their computed results.
//!
//! # Transformations
//!
//! - `Binary(Add, Literal(3), Literal(5))` → `Literal(8)`
//! - `Binary(Mul, Literal(0), x)` → `Literal(0)` (for integer types)
//! - `Unary(Negate, Literal(-5))` → `Literal(5)`
//! - `Binary(LogicalAnd, Literal(false), x)` → `Literal(false)` (short-circuit)
//! - `Binary(LogicalOr, Literal(true), x)` → `Literal(true)` (short-circuit)

use crate::pass::{Pass, PassResult};
use crate::OptResult;
use std::collections::BTreeMap;
use x3_ast::BinaryOp;
use x3_common::Literal;
use x3_mir::{MirModule, MirRhs, MirStatement, MirValue};

/// Constant folding optimization pass.
pub struct ConstantFoldPass {
    /// Track which values are known constants.
    _constants: BTreeMap<MirValue, Literal>,
}

impl ConstantFoldPass {
    pub fn new() -> Self {
        ConstantFoldPass {
            _constants: BTreeMap::new(),
        }
    }

    /// Attempt to fold a binary operation on two literals.
    fn fold_binary(&self, op: BinaryOp, left: &Literal, right: &Literal) -> Option<Literal> {
        use BinaryOp::*;
        use Literal::*;

        match (op, left, right) {
            // Integer arithmetic
            (Add, Integer(a), Integer(b)) => Some(Integer(a.wrapping_add(*b))),
            (Sub, Integer(a), Integer(b)) => Some(Integer(a.wrapping_sub(*b))),
            (Mul, Integer(a), Integer(b)) => Some(Integer(a.wrapping_mul(*b))),
            (Div, Integer(a), Integer(b)) if *b != 0 => Some(Integer(a.wrapping_div(*b))),
            (Mod, Integer(a), Integer(b)) if *b != 0 => Some(Integer(a.wrapping_rem(*b))),

            // Float arithmetic
            (Add, Float(a), Float(b)) => Some(Float(a + b)),
            (Sub, Float(a), Float(b)) => Some(Float(a - b)),
            (Mul, Float(a), Float(b)) => Some(Float(a * b)),
            (Div, Float(a), Float(b)) if *b != 0.0 => Some(Float(a / b)),

            // Integer comparisons
            (Equal, Integer(a), Integer(b)) => Some(Bool(a == b)),
            (NotEqual, Integer(a), Integer(b)) => Some(Bool(a != b)),
            (Less, Integer(a), Integer(b)) => Some(Bool(a < b)),
            (LessEqual, Integer(a), Integer(b)) => Some(Bool(a <= b)),
            (Greater, Integer(a), Integer(b)) => Some(Bool(a > b)),
            (GreaterEqual, Integer(a), Integer(b)) => Some(Bool(a >= b)),

            // Float comparisons
            (Equal, Float(a), Float(b)) => Some(Bool(a == b)),
            (NotEqual, Float(a), Float(b)) => Some(Bool(a != b)),
            (Less, Float(a), Float(b)) => Some(Bool(a < b)),
            (LessEqual, Float(a), Float(b)) => Some(Bool(a <= b)),
            (Greater, Float(a), Float(b)) => Some(Bool(a > b)),
            (GreaterEqual, Float(a), Float(b)) => Some(Bool(a >= b)),

            // Boolean comparisons
            (Equal, Bool(a), Bool(b)) => Some(Bool(a == b)),
            (NotEqual, Bool(a), Bool(b)) => Some(Bool(a != b)),

            // Logical operations (short-circuit semantics preserved)
            (LogicalAnd, Bool(false), _) => Some(Bool(false)),
            (LogicalAnd, Bool(true), Bool(b)) => Some(Bool(*b)),
            (LogicalOr, Bool(true), _) => Some(Bool(true)),
            (LogicalOr, Bool(false), Bool(b)) => Some(Bool(*b)),

            // String concatenation
            (Add, String(a), String(b)) => Some(String(format!("{}{}", a, b))),

            _ => None,
        }
    }

    /// Attempt to fold a unary operation on a literal.
    fn fold_unary(&self, op: x3_ast::UnaryOp, val: &Literal) -> Option<Literal> {
        use x3_ast::UnaryOp::*;
        use Literal::*;

        match (op, val) {
            (Negate, Integer(n)) => Some(Integer(-n)),
            (Negate, Float(f)) => Some(Float(-f)),
            (Not, Bool(b)) => Some(Bool(!b)),
            _ => None,
        }
    }

    /// Check if a literal is zero (for algebraic simplifications).
    fn is_zero(lit: &Literal) -> bool {
        match lit {
            Literal::Integer(0) => true,
            Literal::Float(f) => *f == 0.0,
            _ => false,
        }
    }

    /// Check if a literal is one (for algebraic simplifications).
    #[allow(dead_code)]
    fn is_one(lit: &Literal) -> bool {
        match lit {
            Literal::Integer(1) => true,
            Literal::Float(f) => *f == 1.0,
            _ => false,
        }
    }
}

impl Default for ConstantFoldPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for ConstantFoldPass {
    fn name(&self) -> &'static str {
        "constant_fold"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut changes = 0usize;

        for func in module.functions.iter_mut() {
            // Build constant map for this function
            let mut constants: BTreeMap<MirValue, Literal> = BTreeMap::new();

            for block in func.blocks.iter_mut() {
                let mut new_statements = Vec::with_capacity(block.statements.len());

                for stmt in block.statements.drain(..) {
                    let MirStatement { target, rhs } = stmt;

                    let new_rhs = match &rhs {
                        // Record literal values
                        MirRhs::Literal(lit) => {
                            constants.insert(target, lit.clone());
                            rhs
                        }

                        // Fold binary operations
                        MirRhs::Binary(op, left, right) => {
                            let left_const = constants.get(left).cloned();
                            let right_const = constants.get(right).cloned();

                            match (left_const, right_const) {
                                // Both operands are constants - fully fold
                                (Some(l), Some(r)) => {
                                    if let Some(result) = self.fold_binary(*op, &l, &r) {
                                        constants.insert(target, result.clone());
                                        changes += 1;
                                        MirRhs::Literal(result)
                                    } else {
                                        rhs
                                    }
                                }

                                // Algebraic simplifications with one constant
                                (Some(l), None) => {
                                    use BinaryOp::*;
                                    match op {
                                        // 0 * x => 0
                                        Mul if Self::is_zero(&l) => {
                                            constants.insert(target, l.clone());
                                            changes += 1;
                                            MirRhs::Literal(l)
                                        }
                                        // false && x => false
                                        LogicalAnd if matches!(l, Literal::Bool(false)) => {
                                            constants.insert(target, Literal::Bool(false));
                                            changes += 1;
                                            MirRhs::Literal(Literal::Bool(false))
                                        }
                                        // true || x => true
                                        LogicalOr if matches!(l, Literal::Bool(true)) => {
                                            constants.insert(target, Literal::Bool(true));
                                            changes += 1;
                                            MirRhs::Literal(Literal::Bool(true))
                                        }
                                        _ => rhs,
                                    }
                                }

                                (None, Some(r)) => {
                                    use BinaryOp::*;
                                    match op {
                                        // x * 0 => 0
                                        Mul if Self::is_zero(&r) => {
                                            constants.insert(target, r.clone());
                                            changes += 1;
                                            MirRhs::Literal(r)
                                        }
                                        // x && false => false
                                        LogicalAnd if matches!(r, Literal::Bool(false)) => {
                                            constants.insert(target, Literal::Bool(false));
                                            changes += 1;
                                            MirRhs::Literal(Literal::Bool(false))
                                        }
                                        // x || true => true
                                        LogicalOr if matches!(r, Literal::Bool(true)) => {
                                            constants.insert(target, Literal::Bool(true));
                                            changes += 1;
                                            MirRhs::Literal(Literal::Bool(true))
                                        }
                                        _ => rhs,
                                    }
                                }

                                _ => rhs,
                            }
                        }

                        // Fold unary operations
                        MirRhs::Unary(op, src) => {
                            if let Some(val) = constants.get(src) {
                                if let Some(result) = self.fold_unary(*op, val) {
                                    constants.insert(target, result.clone());
                                    changes += 1;
                                    MirRhs::Literal(result)
                                } else {
                                    rhs
                                }
                            } else {
                                rhs
                            }
                        }

                        // Calls are not folded (may have side effects)
                        MirRhs::Call { .. } => rhs,

                        // Loads and stores are not folded
                        MirRhs::Load { .. } => rhs,
                        MirRhs::Store { .. } => rhs,
                    };

                    new_statements.push(MirStatement {
                        target,
                        rhs: new_rhs,
                    });
                }

                block.statements = new_statements;
            }
        }

        if changes > 0 {
            Ok(PassResult::with_count(
                changes,
                format!("folded {} constant expressions", changes),
            ))
        } else {
            Ok(PassResult::no_change())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::BinaryOp;
    use x3_common::Span;
    use x3_mir::{MirBlock, MirBlockId, MirFunction, MirTerminator, SymbolId};

    fn make_func(statements: Vec<MirStatement>) -> MirFunction {
        MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![MirBlock {
                id: MirBlockId(0),
                statements,
                terminator: Some(MirTerminator::Return(None)),
            }],
            span: Span::dummy(),
        }
    }

    fn make_module(func: MirFunction) -> MirModule {
        MirModule {
            functions: vec![func],
            span: Span::dummy(),
        }
    }

    #[test]
    fn fold_int_addition() {
        // v0 = 3
        // v1 = 5
        // v2 = v0 + v1  => should become v2 = 8
        let stmts = vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(3)),
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Literal(Literal::Integer(5)),
            },
            MirStatement {
                target: MirValue(2),
                rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(1)),
            },
        ];

        let mut module = make_module(make_func(stmts));
        let pass = ConstantFoldPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        assert_eq!(result.transformations, 1);

        // Check v2 is now Literal(8)
        let v2_stmt = &module.functions[0].blocks[0].statements[2];
        assert_eq!(v2_stmt.rhs, MirRhs::Literal(Literal::Integer(8)));
    }

    #[test]
    fn fold_bool_and_short_circuit() {
        // v0 = false
        // v1 = v0 && v2  => should become false
        let stmts = vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Bool(false)),
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Binary(BinaryOp::LogicalAnd, MirValue(0), MirValue(99)), // v99 unknown
            },
        ];

        let mut module = make_module(make_func(stmts));
        let pass = ConstantFoldPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let v1_stmt = &module.functions[0].blocks[0].statements[1];
        assert_eq!(v1_stmt.rhs, MirRhs::Literal(Literal::Bool(false)));
    }

    #[test]
    fn fold_mul_by_zero() {
        // v0 = 0
        // v1 = v0 * v99  => should become 0
        let stmts = vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(0)),
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Binary(BinaryOp::Mul, MirValue(0), MirValue(99)),
            },
        ];

        let mut module = make_module(make_func(stmts));
        let pass = ConstantFoldPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let v1_stmt = &module.functions[0].blocks[0].statements[1];
        assert_eq!(v1_stmt.rhs, MirRhs::Literal(Literal::Integer(0)));
    }

    #[test]
    fn fold_negation() {
        // v0 = -5
        // v1 = -v0  => should become 5
        let stmts = vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(-5)),
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Unary(x3_ast::UnaryOp::Negate, MirValue(0)),
            },
        ];

        let mut module = make_module(make_func(stmts));
        let pass = ConstantFoldPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let v1_stmt = &module.functions[0].blocks[0].statements[1];
        assert_eq!(v1_stmt.rhs, MirRhs::Literal(Literal::Integer(5)));
    }

    #[test]
    fn fold_comparison() {
        // v0 = 3
        // v1 = 5
        // v2 = v0 < v1  => should become true
        let stmts = vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(3)),
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Literal(Literal::Integer(5)),
            },
            MirStatement {
                target: MirValue(2),
                rhs: MirRhs::Binary(BinaryOp::Less, MirValue(0), MirValue(1)),
            },
        ];

        let mut module = make_module(make_func(stmts));
        let pass = ConstantFoldPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let v2_stmt = &module.functions[0].blocks[0].statements[2];
        assert_eq!(v2_stmt.rhs, MirRhs::Literal(Literal::Bool(true)));
    }

    #[test]
    fn no_fold_division_by_zero() {
        // v0 = 10
        // v1 = 0
        // v2 = v0 / v1  => should NOT fold (division by zero)
        let stmts = vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(10)),
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Literal(Literal::Integer(0)),
            },
            MirStatement {
                target: MirValue(2),
                rhs: MirRhs::Binary(BinaryOp::Div, MirValue(0), MirValue(1)),
            },
        ];

        let mut module = make_module(make_func(stmts));
        let pass = ConstantFoldPass::new();
        let result = pass.run(&mut module).unwrap();

        // Should not fold - division by zero is a runtime error
        let v2_stmt = &module.functions[0].blocks[0].statements[2];
        assert!(matches!(v2_stmt.rhs, MirRhs::Binary(BinaryOp::Div, _, _)));
    }

    #[test]
    fn no_change_non_constant() {
        // v0 = call foo()  -- not a constant
        // v1 = v0 + 1
        let stmts = vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Call {
                    target: SymbolId(1),
                    args: vec![],
                },
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Literal(Literal::Integer(1)),
            },
            MirStatement {
                target: MirValue(2),
                rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(1)),
            },
        ];

        let mut module = make_module(make_func(stmts));
        let pass = ConstantFoldPass::new();
        let result = pass.run(&mut module).unwrap();

        // The Add cannot be folded since v0 is not constant
        assert!(!result.changed || result.transformations == 0);
    }
}
