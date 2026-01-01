//! Value Numbering: Canonical Expression Representation
//!
//! Value numbering assigns unique identifiers to equivalent expressions,
//! enabling detection of commutative equivalences and identical computations.
//!
//! Example:
//!   a + b   → VN = 42
//!   b + a   → VN = 42  (same! canonicalized)
//!   a * c   → VN = 43

use std::collections::{BTreeMap, BTreeSet};
use x3_ast::BinaryOp;
use x3_mir::MirValue;

/// Unique identifier for a value
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueNumber(u32);

impl ValueNumber {
    pub fn new(id: u32) -> Self {
        ValueNumber(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// Canonical form of an expression (normalized for equivalence)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalExpr {
    /// Commutative binary operation (operands as canonical strings)
    CommutativeBinary(String, String, String), // op, left, right
    /// Non-commutative binary operation
    Binary(String, String, String), // op, left, right
    /// Unary operation
    Unary(String, String), // op, operand
    /// Variable/value reference
    Variable(String),
}

impl CanonicalExpr {
    /// Canonicalize a binary operation: sort operands for commutative ops
    pub fn from_binary(op: BinaryOp, lhs: MirValue, rhs: MirValue) -> Self {
        let op_str = format!("{:?}", op);
        let is_commutative = matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Mul
                | BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::LogicalAnd
                | BinaryOp::LogicalOr
        );

        let lhs_str = format!("{:?}", lhs);
        let rhs_str = format!("{:?}", rhs);

        if is_commutative {
            let (l, r) = if lhs_str <= rhs_str {
                (lhs_str, rhs_str)
            } else {
                (rhs_str, lhs_str)
            };
            CanonicalExpr::CommutativeBinary(op_str, l, r)
        } else {
            CanonicalExpr::Binary(op_str, lhs_str, rhs_str)
        }
    }

    /// Create from unary operation
    pub fn from_unary(op: x3_ast::UnaryOp, val: MirValue) -> Self {
        let op_str = format!("{:?}", op);
        let val_str = format!("{:?}", val);
        CanonicalExpr::Unary(op_str, val_str)
    }

    /// Create from variable reference
    pub fn from_variable(val: MirValue) -> Self {
        CanonicalExpr::Variable(format!("{:?}", val))
    }
}

/// Value numbering table: maps canonical expressions to value numbers
#[derive(Clone, Debug, Default)]
pub struct ValueNumbering {
    /// Map from canonical expression to assigned value number
    expr_to_vn: BTreeMap<CanonicalExpr, ValueNumber>,
    /// Map from value number back to canonical expression (for debugging)
    vn_to_canonical: BTreeMap<ValueNumber, CanonicalExpr>,
    /// Next available value number
    next_vn: u32,
}

impl ValueNumbering {
    /// Create a new value numbering table
    pub fn new() -> Self {
        ValueNumbering {
            expr_to_vn: BTreeMap::new(),
            vn_to_canonical: BTreeMap::new(),
            next_vn: 1,
        }
    }

    /// Assign or retrieve a value number for an expression
    pub fn canonicalize(&mut self, expr: CanonicalExpr) -> ValueNumber {
        match self.expr_to_vn.get(&expr) {
            Some(&vn) => vn,
            None => {
                let vn = ValueNumber(self.next_vn);
                self.next_vn += 1;
                self.expr_to_vn.insert(expr.clone(), vn);
                self.vn_to_canonical.insert(vn, expr);
                vn
            }
        }
    }

    /// Check if two expressions have the same value number
    pub fn are_equivalent(&mut self, expr1: CanonicalExpr, expr2: CanonicalExpr) -> bool {
        let vn1 = self.canonicalize(expr1);
        let vn2 = self.canonicalize(expr2);
        vn1 == vn2
    }

    /// Get the value number for an expression (without allocating if new)
    pub fn lookup(&self, expr: &CanonicalExpr) -> Option<ValueNumber> {
        self.expr_to_vn.get(expr).copied()
    }

    /// Get canonical expression for a value number
    pub fn get_expr(&self, vn: ValueNumber) -> Option<&CanonicalExpr> {
        self.vn_to_canonical.get(&vn)
    }

    /// Get all value numbers (for iteration)
    pub fn all_value_numbers(&self) -> BTreeSet<ValueNumber> {
        self.vn_to_canonical.keys().copied().collect()
    }

    /// Merge another value numbering table into this one
    pub fn merge(&mut self, other: &ValueNumbering) {
        for (expr, _vn) in &other.expr_to_vn {
            self.canonicalize(expr.clone());
        }
    }

    /// Reset to clean state
    pub fn clear(&mut self) {
        self.expr_to_vn.clear();
        self.vn_to_canonical.clear();
        self.next_vn = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_mir::MirValue;

    #[test]
    fn vn_commutative_equivalence() {
        let mut vn = ValueNumbering::new();
        let a = MirValue(1);
        let b = MirValue(2);

        let expr1 = CanonicalExpr::from_binary(BinaryOp::Add, a, b);
        let expr2 = CanonicalExpr::from_binary(BinaryOp::Add, b, a);

        let vn1 = vn.canonicalize(expr1);
        let vn2 = vn.canonicalize(expr2);

        assert_eq!(vn1, vn2, "Commutative expressions should have same VN");
    }

    #[test]
    fn vn_non_commutative_difference() {
        let mut vn = ValueNumbering::new();
        let a = MirValue(1);
        let b = MirValue(2);

        let expr1 = CanonicalExpr::from_binary(BinaryOp::Sub, a, b);
        let expr2 = CanonicalExpr::from_binary(BinaryOp::Sub, b, a);

        let vn1 = vn.canonicalize(expr1);
        let vn2 = vn.canonicalize(expr2);

        assert_ne!(vn1, vn2, "Non-commutative expressions should differ");
    }

    #[test]
    fn vn_repeated_lookup() {
        let mut vn = ValueNumbering::new();
        let a = MirValue(1);
        let b = MirValue(2);

        let expr = CanonicalExpr::from_binary(BinaryOp::Mul, a, b);
        let vn1 = vn.canonicalize(expr.clone());
        let vn2 = vn.canonicalize(expr);

        assert_eq!(
            vn1, vn2,
            "Same expression should get same VN on repeated lookup"
        );
    }

    #[test]
    fn vn_lookup_retrieval() {
        let mut vn = ValueNumbering::new();
        let a = MirValue(1);
        let b = MirValue(2);

        let expr = CanonicalExpr::from_binary(BinaryOp::Add, a, b);
        let vn1 = vn.canonicalize(expr.clone());

        assert_eq!(
            vn.get_expr(vn1),
            Some(&expr),
            "Should retrieve expression by VN"
        );
    }

    #[test]
    fn vn_multiple_expressions() {
        let mut vn = ValueNumbering::new();
        let a = MirValue(1);
        let b = MirValue(2);
        let c = MirValue(3);

        let add_expr = CanonicalExpr::from_binary(BinaryOp::Add, a, b);
        let mul_expr = CanonicalExpr::from_binary(BinaryOp::Mul, b, c);

        let vn_add = vn.canonicalize(add_expr);
        let vn_mul = vn.canonicalize(mul_expr);

        assert_ne!(
            vn_add, vn_mul,
            "Different expressions should have different VNs"
        );
    }
}
