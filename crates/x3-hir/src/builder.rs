//! HIR Builder API for ergonomic construction.
//!
//! This module provides a fluent API for building HIR structures,
//! primarily used for:
//! - AI mutation engine (agents create/modify HIR nodes)
//! - Testing (construct HIR without going through parser)
//! - Code generation (synthesize HIR from templates)
//!
//! The builder tracks symbol allocation and ensures HIR integrity.

use x3_ast::{BinaryOp, UnaryOp};
use x3_common::{Literal, Span};
use x3_typeck::Type;

use crate::hir::*;

/// Builder for constructing HIR modules.
pub struct HirBuilder {
    next_symbol_id: usize,
    next_label_id: usize,
    next_atomic_id: usize,
    symbols: Vec<Symbol>,
    globals: Vec<HirGlobal>,
    functions: Vec<HirFunction>,
    agents: Vec<HirAgent>,
    span: Span,
}

impl HirBuilder {
    /// Create a new HIR builder.
    pub fn new() -> Self {
        Self {
            next_symbol_id: 0,
            next_label_id: 0,
            next_atomic_id: 0,
            symbols: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
            agents: Vec::new(),
            span: Span::new(0, 0),
        }
    }

    /// Set the module span.
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    /// Allocate a new symbol ID.
    pub fn alloc_symbol(
        &mut self,
        name: impl Into<String>,
        kind: SymbolKind,
        ty: Type,
        span: Span,
    ) -> SymbolId {
        let id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;
        self.symbols.push(Symbol {
            id,
            name: name.into(),
            kind,
            ty,
            span,
        });
        id
    }

    /// Allocate a new label ID.
    pub fn alloc_label(&mut self) -> LabelId {
        let id = LabelId(self.next_label_id);
        self.next_label_id += 1;
        id
    }

    /// Allocate a new atomic block ID.
    pub fn alloc_atomic(&mut self) -> AtomicBlockId {
        let id = AtomicBlockId(self.next_atomic_id);
        self.next_atomic_id += 1;
        id
    }

    /// Add a global to the module.
    pub fn add_global(&mut self, global: HirGlobal) {
        self.globals.push(global);
    }

    /// Add a function to the module.
    pub fn add_function(&mut self, function: HirFunction) {
        self.functions.push(function);
    }

    /// Add an agent to the module.
    pub fn add_agent(&mut self, agent: HirAgent) {
        self.agents.push(agent);
    }

    /// Build the HIR module.
    pub fn build(self) -> HirModule {
        HirModule {
            globals: self.globals,
            functions: self.functions,
            agents: self.agents,
            symbols: self.symbols,
            span: self.span,
        }
    }

    // === Function Builder ===

    /// Start building a function.
    pub fn function(&mut self, name: impl Into<String>, span: Span) -> FunctionBuilder {
        let name = name.into();
        FunctionBuilder::new(self, name, span)
    }
}

impl Default for HirBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing HIR functions.
pub struct FunctionBuilder<'a> {
    builder: &'a mut HirBuilder,
    name: String,
    params: Vec<HirParam>,
    body: Vec<HirStmt>,
    return_ty: Type,
    attrs: FunctionAttrs,
    span: Span,
}

impl<'a> FunctionBuilder<'a> {
    fn new(builder: &'a mut HirBuilder, name: String, span: Span) -> Self {
        Self {
            builder,
            name,
            params: Vec::new(),
            body: Vec::new(),
            return_ty: Type::unit(),
            attrs: FunctionAttrs::default(),
            span,
        }
    }

    /// Add a parameter.
    pub fn param(mut self, name: impl Into<String>, ty: Type, mutable: bool, span: Span) -> Self {
        let name = name.into();
        let symbol =
            self.builder
                .alloc_symbol(&name, SymbolKind::Param { mutable }, ty.clone(), span);
        self.params.push(HirParam {
            symbol,
            name,
            ty,
            mutable,
            span,
        });
        self
    }

    /// Set return type.
    pub fn returns(mut self, ty: Type) -> Self {
        self.return_ty = ty;
        self
    }

    /// Add a statement to the body.
    pub fn stmt(mut self, stmt: HirStmt) -> Self {
        self.body.push(stmt);
        self
    }

    /// Set multiple statements as body.
    pub fn body(mut self, stmts: Vec<HirStmt>) -> Self {
        self.body = stmts;
        self
    }

    /// Mark as init function.
    pub fn is_init(mut self) -> Self {
        self.attrs.is_init = true;
        self
    }

    /// Mark as view function.
    pub fn is_view(mut self) -> Self {
        self.attrs.is_view = true;
        self
    }

    /// Mark as payable function.
    pub fn is_payable(mut self) -> Self {
        self.attrs.is_payable = true;
        self
    }

    /// Mark as external function.
    pub fn is_external(mut self) -> Self {
        self.attrs.is_external = true;
        self
    }

    /// Set target VM.
    pub fn target_vm(mut self, vm: TargetVm) -> Self {
        self.attrs.target_vm = Some(vm);
        self
    }

    /// Build the function and add it to the module.
    pub fn build(self) -> SymbolId {
        let fn_ty = Type::function(
            self.params.iter().map(|p| p.ty.clone()).collect(),
            self.return_ty.clone(),
        );
        let symbol = self
            .builder
            .alloc_symbol(&self.name, SymbolKind::Function, fn_ty, self.span);
        let function = HirFunction {
            symbol,
            params: self.params,
            body: self.body,
            return_ty: self.return_ty,
            attrs: self.attrs,
            span: self.span,
        };
        self.builder.add_function(function);
        symbol
    }
}

// ============================================================================
// Statement Builders
// ============================================================================

/// Builder for HIR statements.
pub struct StmtBuilder;

impl StmtBuilder {
    /// Create a let binding.
    pub fn let_binding(
        symbol: SymbolId,
        ty: Type,
        value: HirExpr,
        mutable: bool,
        span: Span,
    ) -> HirStmt {
        HirStmt::Let {
            symbol,
            ty,
            value,
            mutable,
            span,
        }
    }

    /// Create an assignment.
    pub fn assign(target: AssignTarget, value: HirExpr, span: Span) -> HirStmt {
        HirStmt::Assign {
            target,
            value,
            span,
        }
    }

    /// Create a simple variable assignment.
    pub fn assign_var(symbol: SymbolId, value: HirExpr, span: Span) -> HirStmt {
        HirStmt::Assign {
            target: AssignTarget::Variable(symbol),
            value,
            span,
        }
    }

    /// Create an expression statement.
    pub fn expr(expr: HirExpr) -> HirStmt {
        HirStmt::Expr(expr)
    }

    /// Create a return statement.
    pub fn ret(value: Option<HirExpr>, span: Span) -> HirStmt {
        HirStmt::Return { value, span }
    }

    /// Create an if statement.
    pub fn if_stmt(
        condition: HirExpr,
        then_block: Vec<HirStmt>,
        else_block: Vec<HirStmt>,
        span: Span,
    ) -> HirStmt {
        HirStmt::If {
            condition,
            then_block,
            else_block,
            span,
        }
    }

    /// Create a while loop.
    pub fn while_loop(
        label: Option<LabelId>,
        condition: HirExpr,
        body: Vec<HirStmt>,
        span: Span,
    ) -> HirStmt {
        HirStmt::While {
            label,
            condition,
            body,
            span,
        }
    }

    /// Create an infinite loop (while true).
    pub fn loop_stmt(label: Option<LabelId>, body: Vec<HirStmt>, span: Span) -> HirStmt {
        HirStmt::While {
            label,
            condition: ExprBuilder::bool_lit(true, span),
            body,
            span,
        }
    }

    /// Create a break statement.
    pub fn break_stmt(label: Option<LabelId>, span: Span) -> HirStmt {
        HirStmt::Break { label, span }
    }

    /// Create a continue statement.
    pub fn continue_stmt(label: Option<LabelId>, span: Span) -> HirStmt {
        HirStmt::Continue { label, span }
    }

    /// Create an atomic begin marker.
    pub fn atomic_begin(block_id: AtomicBlockId, span: Span) -> HirStmt {
        HirStmt::AtomicBegin { block_id, span }
    }

    /// Create an atomic end marker (commit).
    pub fn atomic_commit(block_id: AtomicBlockId, span: Span) -> HirStmt {
        HirStmt::AtomicEnd {
            block_id,
            commit: true,
            span,
        }
    }

    /// Create an atomic end marker (rollback).
    pub fn atomic_rollback(block_id: AtomicBlockId, span: Span) -> HirStmt {
        HirStmt::AtomicEnd {
            block_id,
            commit: false,
            span,
        }
    }

    /// Create an emit statement.
    pub fn emit(event_name: impl Into<String>, args: Vec<HirExpr>, span: Span) -> HirStmt {
        HirStmt::Emit {
            event_name: event_name.into(),
            args,
            span,
        }
    }
}

// ============================================================================
// Expression Builders
// ============================================================================

/// Builder for HIR expressions.
pub struct ExprBuilder;

impl ExprBuilder {
    /// Create a literal expression.
    pub fn literal(lit: Literal, ty: Type, span: Span) -> HirExpr {
        HirExpr::new(HirExprKind::Literal(lit), ty, span)
    }

    /// Create an integer literal.
    pub fn int_lit(value: i64, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Literal(Literal::Integer(value)),
            Type::i64(),
            span,
        )
    }

    /// Create a u64 literal.
    pub fn u64_lit(value: u64, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Literal(Literal::Integer(value as i64)),
            Type::u64(),
            span,
        )
    }

    /// Create a boolean literal.
    pub fn bool_lit(value: bool, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Literal(Literal::Bool(value)),
            Type::bool(),
            span,
        )
    }

    /// Create a string literal.
    pub fn string_lit(value: impl Into<String>, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Literal(Literal::String(value.into())),
            Type::string(),
            span,
        )
    }

    /// Create a variable access.
    pub fn var(symbol: SymbolId, ty: Type, span: Span) -> HirExpr {
        HirExpr::new(HirExprKind::Var(symbol), ty, span)
    }

    /// Create a binary expression.
    pub fn binary(op: BinaryOp, left: HirExpr, right: HirExpr, ty: Type, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            ty,
            span,
        )
    }

    /// Create an add expression.
    pub fn add(left: HirExpr, right: HirExpr, ty: Type, span: Span) -> HirExpr {
        Self::binary(BinaryOp::Add, left, right, ty, span)
    }

    /// Create a subtract expression.
    pub fn sub(left: HirExpr, right: HirExpr, ty: Type, span: Span) -> HirExpr {
        Self::binary(BinaryOp::Sub, left, right, ty, span)
    }

    /// Create a multiply expression.
    pub fn mul(left: HirExpr, right: HirExpr, ty: Type, span: Span) -> HirExpr {
        Self::binary(BinaryOp::Mul, left, right, ty, span)
    }

    /// Create a comparison expression.
    pub fn eq(left: HirExpr, right: HirExpr, span: Span) -> HirExpr {
        Self::binary(BinaryOp::Equal, left, right, Type::bool(), span)
    }

    /// Create a less-than expression.
    pub fn lt(left: HirExpr, right: HirExpr, span: Span) -> HirExpr {
        Self::binary(BinaryOp::Less, left, right, Type::bool(), span)
    }

    /// Create a unary expression.
    pub fn unary(op: UnaryOp, operand: HirExpr, ty: Type, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
            ty,
            span,
        )
    }

    /// Create a negation expression.
    pub fn neg(operand: HirExpr, ty: Type, span: Span) -> HirExpr {
        Self::unary(UnaryOp::Negate, operand, ty, span)
    }

    /// Create a logical not expression.
    pub fn not(operand: HirExpr, span: Span) -> HirExpr {
        Self::unary(UnaryOp::Not, operand, Type::bool(), span)
    }

    /// Create a function call.
    pub fn call(callee: SymbolId, args: Vec<HirExpr>, ty: Type, span: Span) -> HirExpr {
        HirExpr::new(HirExprKind::Call { callee, args }, ty, span)
    }

    /// Create a method call.
    pub fn method_call(
        receiver: HirExpr,
        method: impl Into<String>,
        args: Vec<HirExpr>,
        ty: Type,
        span: Span,
    ) -> HirExpr {
        HirExpr::new(
            HirExprKind::MethodCall {
                receiver: Box::new(receiver),
                method: method.into(),
                args,
            },
            ty,
            span,
        )
    }

    /// Create a field access.
    pub fn field(object: HirExpr, field: impl Into<String>, ty: Type, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Field {
                object: Box::new(object),
                field: field.into(),
            },
            ty,
            span,
        )
    }

    /// Create an index access.
    pub fn index(array: HirExpr, index: HirExpr, element_ty: Type, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Index {
                array: Box::new(array),
                index: Box::new(index),
            },
            element_ty,
            span,
        )
    }

    /// Create an array literal.
    pub fn array(elements: Vec<HirExpr>, element_ty: Type, span: Span) -> HirExpr {
        let len = elements.len();
        HirExpr::new(
            HirExprKind::Array(elements),
            Type::array(element_ty, len),
            span,
        )
    }

    /// Create a tuple literal.
    pub fn tuple(elements: Vec<HirExpr>, span: Span) -> HirExpr {
        let types: Vec<Type> = elements.iter().map(|e| e.ty.clone()).collect();
        HirExpr::new(HirExprKind::Tuple(elements), Type::tuple(types), span)
    }

    /// Create a block expression.
    pub fn block(stmts: Vec<HirStmt>, expr: Option<HirExpr>, ty: Type, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Block {
                stmts,
                expr: expr.map(Box::new),
            },
            ty,
            span,
        )
    }

    /// Create an if expression.
    pub fn if_expr(
        condition: HirExpr,
        then_expr: HirExpr,
        else_expr: HirExpr,
        ty: Type,
        span: Span,
    ) -> HirExpr {
        HirExpr::new(
            HirExprKind::IfExpr {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            },
            ty,
            span,
        )
    }

    /// Create a type cast.
    pub fn cast(expr: HirExpr, target_ty: Type, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Cast {
                expr: Box::new(expr),
                target_ty: target_ty.clone(),
            },
            target_ty,
            span,
        )
    }

    /// Create a context access.
    pub fn context(field: ContextField, ty: Type, span: Span) -> HirExpr {
        HirExpr::new(HirExprKind::ContextAccess(field), ty, span)
    }

    /// Create a self reference.
    pub fn self_ref(ty: Type, span: Span) -> HirExpr {
        HirExpr::new(HirExprKind::SelfRef, ty, span)
    }

    /// Create a VM intrinsic call.
    pub fn vm_intrinsic(
        vm: TargetVm,
        intrinsic: VmIntrinsic,
        args: Vec<HirExpr>,
        ty: Type,
        span: Span,
    ) -> HirExpr {
        HirExpr::new(
            HirExprKind::VmIntrinsic {
                vm,
                intrinsic,
                args,
            },
            ty,
            span,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_simple_function() {
        let mut builder = HirBuilder::new();
        let span = Span::new(0, 100);

        let fn_symbol = builder
            .function("add", span)
            .param("x", Type::i64(), false, span)
            .param("y", Type::i64(), false, span)
            .returns(Type::i64())
            .build();

        let module = builder.build();
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.symbol_name(fn_symbol), Some("add"));
    }

    #[test]
    fn build_expressions() {
        let span = Span::new(0, 10);

        let lit = ExprBuilder::int_lit(42, span);
        assert!(matches!(
            lit.kind,
            HirExprKind::Literal(Literal::Integer(42))
        ));

        let bool_lit = ExprBuilder::bool_lit(true, span);
        assert!(matches!(
            bool_lit.kind,
            HirExprKind::Literal(Literal::Bool(true))
        ));

        let add = ExprBuilder::add(
            ExprBuilder::int_lit(1, span),
            ExprBuilder::int_lit(2, span),
            Type::i64(),
            span,
        );
        assert!(matches!(
            add.kind,
            HirExprKind::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn build_statements() {
        let span = Span::new(0, 10);

        let ret = StmtBuilder::ret(Some(ExprBuilder::int_lit(0, span)), span);
        assert!(matches!(ret, HirStmt::Return { .. }));

        let break_stmt = StmtBuilder::break_stmt(None, span);
        assert!(matches!(break_stmt, HirStmt::Break { .. }));
    }
}
