//! Type inference engine for X3.
//!
//! This module implements constraint-based type inference using a
//! simplified Hindley-Milner algorithm. It generates type constraints
//! from expressions and solves them via unification.

use crate::env::TypeEnv;
use crate::error::{TypeError, TypeErrorKind};
use crate::types::{PrimitiveType, Type, TypeKind};
use x3_common::Span;

/// A type constraint to be solved.
#[derive(Clone, Debug)]
pub struct Constraint {
    /// The left-hand type.
    pub left: Type,
    /// The right-hand type.
    pub right: Type,
    /// Where this constraint originated.
    pub span: Span,
    /// Description of why this constraint exists.
    pub reason: ConstraintReason,
}

/// Reason for a type constraint.
#[derive(Clone, Debug)]
pub enum ConstraintReason {
    /// Assignment: left is the target, right is the value.
    Assignment,
    /// Binary operation result.
    BinaryOp(String),
    /// Function argument.
    FunctionArgument { param_index: usize },
    /// Function return type.
    FunctionReturn,
    /// Condition must be bool.
    Condition,
    /// Array element types must match.
    ArrayElement,
    /// If/else branches must have same type.
    BranchTypes,
    /// Explicit type annotation.
    Annotation,
    /// Inferred from context.
    Inferred,
}

impl Constraint {
    pub fn new(left: Type, right: Type, span: Span, reason: ConstraintReason) -> Self {
        Self {
            left,
            right,
            span,
            reason,
        }
    }

    pub fn assignment(left: Type, right: Type, span: Span) -> Self {
        Self::new(left, right, span, ConstraintReason::Assignment)
    }

    pub fn binary_op(result: Type, expected: Type, op: &str, span: Span) -> Self {
        Self::new(
            result,
            expected,
            span,
            ConstraintReason::BinaryOp(op.to_string()),
        )
    }

    pub fn argument(param: Type, arg: Type, param_index: usize, span: Span) -> Self {
        Self::new(
            param,
            arg,
            span,
            ConstraintReason::FunctionArgument { param_index },
        )
    }

    pub fn return_type(expected: Type, found: Type, span: Span) -> Self {
        Self::new(expected, found, span, ConstraintReason::FunctionReturn)
    }

    pub fn condition(found: Type, span: Span) -> Self {
        Self::new(Type::bool(), found, span, ConstraintReason::Condition)
    }

    pub fn branches(then_ty: Type, else_ty: Type, span: Span) -> Self {
        Self::new(then_ty, else_ty, span, ConstraintReason::BranchTypes)
    }
}

/// Type inference engine.
pub struct TypeInference<'env> {
    /// Reference to the type environment.
    env: &'env mut TypeEnv,
    /// Accumulated constraints.
    constraints: Vec<Constraint>,
    /// Errors encountered during inference.
    errors: Vec<TypeError>,
}

impl<'env> TypeInference<'env> {
    /// Create a new type inference engine.
    pub fn new(env: &'env mut TypeEnv) -> Self {
        Self {
            env,
            constraints: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a constraint to be solved.
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Record an error.
    pub fn error(&mut self, err: TypeError) {
        self.errors.push(err);
    }

    /// Create a fresh type variable.
    pub fn fresh_var(&mut self) -> Type {
        self.env.fresh_type_var()
    }

    /// Solve all accumulated constraints.
    pub fn solve(mut self) -> Result<(), Vec<TypeError>> {
        // Process constraints in order
        for constraint in std::mem::take(&mut self.constraints) {
            if let Err(err) = self.unify(&constraint.left, &constraint.right, constraint.span) {
                self.errors.push(err);
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    /// Unify two types, producing substitutions or an error.
    fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> Result<(), TypeError> {
        // Apply any existing substitutions first
        let t1 = self.env.apply_substitutions(t1);
        let t2 = self.env.apply_substitutions(t2);

        match (&t1.kind, &t2.kind) {
            // Same types unify trivially
            (TypeKind::Primitive(p1), TypeKind::Primitive(p2)) if p1 == p2 => Ok(()),
            (TypeKind::Unit, TypeKind::Unit) => Ok(()),
            (TypeKind::Never, TypeKind::Never) => Ok(()),
            (TypeKind::Any, _) | (_, TypeKind::Any) => Ok(()),
            (TypeKind::String, TypeKind::String) => Ok(()),
            (TypeKind::Bytes, TypeKind::Bytes) => Ok(()),

            // Type variable on the left: substitute
            (TypeKind::TypeVar(id), _) => {
                // Occurs check to prevent infinite types
                if self.env.occurs_in(*id, &t2) {
                    return Err(TypeError::new(
                        TypeErrorKind::RecursiveType(format!("?T{}", id)),
                        span,
                    ));
                }
                self.env.substitute(*id, t2);
                Ok(())
            }

            // Type variable on the right: substitute
            (_, TypeKind::TypeVar(id)) => {
                if self.env.occurs_in(*id, &t1) {
                    return Err(TypeError::new(
                        TypeErrorKind::RecursiveType(format!("?T{}", id)),
                        span,
                    ));
                }
                self.env.substitute(*id, t1);
                Ok(())
            }

            // Function types: unify params and return type
            (TypeKind::Function(sig1), TypeKind::Function(sig2)) => {
                if sig1.params.len() != sig2.params.len() {
                    return Err(TypeError::new(
                        TypeErrorKind::UnificationFailure {
                            type1: t1.clone(),
                            type2: t2.clone(),
                        },
                        span,
                    ));
                }

                for (p1, p2) in sig1.params.iter().zip(sig2.params.iter()) {
                    self.unify(p1, p2, span)?;
                }
                self.unify(&sig1.return_type, &sig2.return_type, span)
            }

            // Array types: unify element and check size
            (
                TypeKind::Array {
                    element: e1,
                    size: s1,
                },
                TypeKind::Array {
                    element: e2,
                    size: s2,
                },
            ) => {
                if s1 != s2 {
                    return Err(TypeError::new(
                        TypeErrorKind::TypeMismatch {
                            expected: t1.clone(),
                            found: t2.clone(),
                        },
                        span,
                    ));
                }
                self.unify(e1, e2, span)
            }

            // Vector types: unify element
            (TypeKind::Vector(e1), TypeKind::Vector(e2)) => self.unify(e1, e2, span),

            // Option types: unify inner
            (TypeKind::Option(i1), TypeKind::Option(i2)) => self.unify(i1, i2, span),

            // Result types: unify both
            (TypeKind::Result { ok: ok1, err: err1 }, TypeKind::Result { ok: ok2, err: err2 }) => {
                self.unify(ok1, ok2, span)?;
                self.unify(err1, err2, span)
            }

            // Tuple types: unify element-wise
            (TypeKind::Tuple(elems1), TypeKind::Tuple(elems2)) => {
                if elems1.len() != elems2.len() {
                    return Err(TypeError::new(
                        TypeErrorKind::TypeMismatch {
                            expected: t1.clone(),
                            found: t2.clone(),
                        },
                        span,
                    ));
                }
                for (e1, e2) in elems1.iter().zip(elems2.iter()) {
                    self.unify(e1, e2, span)?;
                }
                Ok(())
            }

            // Atomic/Context wrappers: unify inner
            (TypeKind::Atomic(i1), TypeKind::Atomic(i2)) => self.unify(i1, i2, span),
            (TypeKind::Context(i1), TypeKind::Context(i2)) => self.unify(i1, i2, span),

            // VM types: must match exactly
            (TypeKind::Vm(v1), TypeKind::Vm(v2)) if v1 == v2 => Ok(()),

            // Named types: must match exactly (or look up)
            (TypeKind::Named(n1), TypeKind::Named(n2)) if n1 == n2 => Ok(()),

            // Agent types: must match by name
            (TypeKind::Agent(a1), TypeKind::Agent(a2)) if a1.name == a2.name => Ok(()),

            // Error type unifies with anything (to allow error recovery)
            (TypeKind::Error, _) | (_, TypeKind::Error) => Ok(()),

            // Never type is a subtype of everything
            (TypeKind::Never, _) => Ok(()),

            // Otherwise: unification failure
            _ => Err(TypeError::new(
                TypeErrorKind::UnificationFailure {
                    type1: t1,
                    type2: t2,
                },
                span,
            )),
        }
    }

    /// Infer the result type of a binary operation.
    pub fn infer_binary_op(
        &mut self,
        op: &str,
        left: &Type,
        right: &Type,
        span: Span,
    ) -> Result<Type, TypeError> {
        let left = self.env.apply_substitutions(left);
        let right = self.env.apply_substitutions(right);

        match op {
            // Arithmetic operators (AST enum names)
            "Add" | "+" | "Sub" | "-" | "Mul" | "*" | "Div" | "/" | "Mod" | "%" | "Pow" => {
                if left.is_numeric() && right.is_numeric() {
                    // Ensure same type
                    self.unify(&left, &right, span)?;
                    Ok(left)
                } else {
                    Err(TypeError::invalid_binary_op(op, left, right, span))
                }
            }

            // Equality operators
            "Equal" | "==" | "NotEqual" | "!=" => {
                if left.is_equatable() {
                    self.unify(&left, &right, span)?;
                    Ok(Type::bool())
                } else {
                    Err(TypeError::invalid_binary_op(op, left, right, span))
                }
            }

            // Comparison operators
            "Less" | "<" | "Greater" | ">" | "LessEqual" | "<=" | "GreaterEqual" | ">=" => {
                if left.is_orderable() {
                    self.unify(&left, &right, span)?;
                    Ok(Type::bool())
                } else {
                    Err(TypeError::invalid_binary_op(op, left, right, span))
                }
            }

            // Logical operators
            "LogicalAnd" | "&&" | "LogicalOr" | "||" => {
                if left.is_bool() && right.is_bool() {
                    Ok(Type::bool())
                } else {
                    Err(TypeError::invalid_binary_op(op, left, right, span))
                }
            }

            // Bitwise operators
            "&" | "|" | "^" | "<<" | ">>" => {
                if left.supports_bitwise() && right.supports_bitwise() {
                    self.unify(&left, &right, span)?;
                    Ok(left)
                } else {
                    Err(TypeError::invalid_binary_op(op, left, right, span))
                }
            }

            _ => Err(TypeError::invalid_binary_op(op, left, right, span)),
        }
    }

    /// Infer the result type of a unary operation.
    pub fn infer_unary_op(
        &mut self,
        op: &str,
        operand: &Type,
        span: Span,
    ) -> Result<Type, TypeError> {
        let operand = self.env.apply_substitutions(operand);

        match op {
            // Negation operator (AST: Negate, symbol: -)
            "Negate" | "-" => {
                if operand.is_numeric() {
                    Ok(operand)
                } else {
                    Err(TypeError::invalid_operation(op, operand, span))
                }
            }
            // Logical not operator (AST: Not, symbol: !)
            "Not" | "!" => {
                if operand.is_bool() {
                    Ok(Type::bool())
                } else {
                    Err(TypeError::invalid_operation(op, operand, span))
                }
            }
            // Bitwise not
            "~" => {
                if operand.supports_bitwise() {
                    Ok(operand)
                } else {
                    Err(TypeError::invalid_operation(op, operand, span))
                }
            }
            _ => Err(TypeError::invalid_operation(op, operand, span)),
        }
    }

    /// Infer type from an integer literal, defaulting to i64 if no context.
    pub fn infer_integer_literal(&self, value: i128) -> Type {
        // Default to i64 for negative, u64 for positive
        if value < 0 {
            Type::i64()
        } else {
            Type::u64()
        }
    }

    /// Check if a value fits in a given integer type.
    pub fn check_integer_bounds(&self, value: i128, ty: &Type) -> bool {
        match &ty.kind {
            TypeKind::Primitive(PrimitiveType::U8) => value >= 0 && value <= u8::MAX as i128,
            TypeKind::Primitive(PrimitiveType::U16) => value >= 0 && value <= u16::MAX as i128,
            TypeKind::Primitive(PrimitiveType::U32) => value >= 0 && value <= u32::MAX as i128,
            TypeKind::Primitive(PrimitiveType::U64) => value >= 0 && value <= u64::MAX as i128,
            TypeKind::Primitive(PrimitiveType::U128) => value >= 0,
            TypeKind::Primitive(PrimitiveType::I8) => {
                value >= i8::MIN as i128 && value <= i8::MAX as i128
            }
            TypeKind::Primitive(PrimitiveType::I16) => {
                value >= i16::MIN as i128 && value <= i16::MAX as i128
            }
            TypeKind::Primitive(PrimitiveType::I32) => {
                value >= i32::MIN as i128 && value <= i32::MAX as i128
            }
            TypeKind::Primitive(PrimitiveType::I64) => {
                value >= i64::MIN as i128 && value <= i64::MAX as i128
            }
            TypeKind::Primitive(PrimitiveType::I128) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_primitives() {
        let mut env = TypeEnv::new();
        let mut infer = TypeInference::new(&mut env);

        // Same types unify
        assert!(infer
            .unify(&Type::u64(), &Type::u64(), Span::default())
            .is_ok());

        // Different types don't unify
        assert!(infer
            .unify(&Type::u64(), &Type::bool(), Span::default())
            .is_err());
    }

    #[test]
    fn test_unify_type_vars() {
        let mut env = TypeEnv::new();
        let var = env.fresh_type_var();

        {
            let mut infer = TypeInference::new(&mut env);
            assert!(infer.unify(&var, &Type::u64(), Span::default()).is_ok());
        }

        // After unification, var should resolve to u64
        let resolved = env.apply_substitutions(&var);
        assert!(!resolved.is_type_var());
    }

    #[test]
    fn test_binary_op_inference() {
        let mut env = TypeEnv::new();
        let mut infer = TypeInference::new(&mut env);

        // u64 + u64 = u64
        let result = infer.infer_binary_op("+", &Type::u64(), &Type::u64(), Span::default());
        assert!(result.is_ok());

        // u64 < u64 = bool
        let result = infer.infer_binary_op("<", &Type::u64(), &Type::u64(), Span::default());
        assert!(result.is_ok());
        assert!(result.unwrap().is_bool());

        // bool && bool = bool
        let result = infer.infer_binary_op("&&", &Type::bool(), &Type::bool(), Span::default());
        assert!(result.is_ok());

        // bool + bool = error
        let result = infer.infer_binary_op("+", &Type::bool(), &Type::bool(), Span::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_unary_op_inference() {
        let mut env = TypeEnv::new();
        let mut infer = TypeInference::new(&mut env);

        // -u64 = u64
        let result = infer.infer_unary_op("-", &Type::u64(), Span::default());
        assert!(result.is_ok());

        // !bool = bool
        let result = infer.infer_unary_op("!", &Type::bool(), Span::default());
        assert!(result.is_ok());

        // !u64 = error
        let result = infer.infer_unary_op("!", &Type::u64(), Span::default());
        assert!(result.is_err());
    }
}
