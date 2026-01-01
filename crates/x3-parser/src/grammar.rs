//! Canonical binding-power table and operator metadata.
//!
//! The values here MUST match the precedence tiers documented in
//! `openspec/x3-language-grammar.md`. Any drift should be caught by CI.

use x3_common::Symbol;

/// Binding power pair (left, right) for binary operators.
/// Higher numbers bind tighter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindingPower {
    pub left: u8,
    pub right: u8,
}

impl BindingPower {
    pub const fn new(left: u8, right: u8) -> Self {
        Self { left, right }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Precedence tiers (mapped from grammar doc)
// ────────────────────────────────────────────────────────────────────────────
// 120 – function calls, indexing, field access (postfix) – handled separately
// 110 – prefix unary (!, -, +)
// 100 – multiplicative (*, /, %)
//  90 – additive (+, -)
//  80 – shift (<<, >>)
//  70 – comparison (<, >, <=, >=)
//  60 – equality (==, !=)
//  50 – bitwise AND (&)
//  45 – bitwise XOR (^)
//  40 – bitwise OR (|)
//  35 – logical AND (&&)
//  30 – logical OR (||)
//  25 – conditional (?:) – ternary, right-associative
//  20 – assignment (=, +=, -=) – right-associative
//  10 – comma / argument separator – lowest

/// Prefix (unary) binding power. Right-associative, so we return the right bp.
pub const PREFIX_BP: u8 = 110;

/// Assignment binding power – right-associative, very low.
pub const ASSIGN_BP: u8 = 20;

/// Get the `(left_bp, right_bp)` for a binary operator symbol.
/// Returns `None` for symbols that are not binary operators.
pub fn binary_binding_power(symbol: Symbol) -> Option<BindingPower> {
    let bp = match symbol {
        // Logical OR – lowest binary
        Symbol::Or => BindingPower::new(30, 31),
        // Logical AND
        Symbol::And => BindingPower::new(35, 36),
        // Bitwise OR – note: Symbol::Pipe if you have one; otherwise skip
        // Symbol::Pipe => BindingPower::new(40, 41),
        // Bitwise XOR
        Symbol::Caret => BindingPower::new(45, 46),
        // Bitwise AND
        // Symbol::Ampersand => BindingPower::new(50, 51),
        // Equality
        Symbol::DoubleEquals | Symbol::BangEquals => BindingPower::new(60, 61),
        // Comparison
        Symbol::Less | Symbol::LessEqual | Symbol::Greater | Symbol::GreaterEqual => {
            BindingPower::new(70, 71)
        }
        // Shift – if lexer emits these
        // Symbol::ShiftLeft | Symbol::ShiftRight => BindingPower::new(80, 81),
        // Additive
        Symbol::Plus | Symbol::Minus => BindingPower::new(90, 91),
        // Multiplicative
        Symbol::Star | Symbol::Slash | Symbol::Percent => BindingPower::new(100, 101),
        _ => return None,
    };
    Some(bp)
}

/// Map a symbol to its `BinaryOp` variant.
/// Returns `None` if the symbol does not represent a binary operator.
pub fn symbol_to_binary_op(symbol: Symbol) -> Option<x3_ast::BinaryOp> {
    use x3_ast::BinaryOp;
    let op = match symbol {
        Symbol::Plus => BinaryOp::Add,
        Symbol::Minus => BinaryOp::Sub,
        Symbol::Star => BinaryOp::Mul,
        Symbol::Slash => BinaryOp::Div,
        Symbol::Percent => BinaryOp::Mod,
        Symbol::Caret => BinaryOp::Pow,
        Symbol::DoubleEquals => BinaryOp::Equal,
        Symbol::BangEquals => BinaryOp::NotEqual,
        Symbol::Less => BinaryOp::Less,
        Symbol::LessEqual => BinaryOp::LessEqual,
        Symbol::Greater => BinaryOp::Greater,
        Symbol::GreaterEqual => BinaryOp::GreaterEqual,
        Symbol::And => BinaryOp::LogicalAnd,
        Symbol::Or => BinaryOp::LogicalOr,
        _ => return None,
    };
    Some(op)
}
