//! AST to HIR lowering.
//!
//! This module transforms the parser's AST into HIR, performing:
//! - Symbol resolution and allocation
//! - Type annotation (propagating types from type checker)
//! - Desugaring (for→while, loop→while, etc.)
//! - Atomic block structuring (begin/end markers)
//! - Validation of control flow
//!
//! The lowerer assumes type checking has already run and types are available.

use std::collections::HashMap;

use x3_ast::{
    BinaryExpression, Expression, Function, GlobalLet, Item, Module, Statement, UnaryExpression,
};
use x3_common::Span;
use x3_typeck::Type;

use crate::error::{HirError, HirErrorKind, HirResult};
use crate::hir::*;

/// Builds HIR from a parser-produced AST.
///
/// This is the main entry point for AST→HIR transformation.
/// Call `HirLowerer::lower(module)` to transform a parsed module.
pub struct HirLowerer {
    /// Next available symbol ID.
    next_symbol_id: usize,
    /// Next available label ID.
    next_label_id: usize,
    /// Next available atomic block ID.
    next_atomic_id: usize,
    /// All allocated symbols.
    symbols: Vec<Symbol>,
    /// Top-level symbol lookup (name → ID).
    top_level: HashMap<String, SymbolId>,
    /// Current loop nesting depth (for break/continue validation).
    loop_depth: usize,
    /// Current atomic block (None if not in atomic context).
    current_atomic: Option<AtomicBlockId>,
}

impl HirLowerer {
    /// Lower an AST module to HIR.
    pub fn lower(module: Module) -> HirResult<HirModule> {
        let mut lowerer = Self {
            next_symbol_id: 0,
            next_label_id: 0,
            next_atomic_id: 0,
            symbols: Vec::new(),
            top_level: HashMap::new(),
            loop_depth: 0,
            current_atomic: None,
        };
        let span = module.span;

        // First pass: collect top-level declarations
        lowerer.collect_top_level(&module.items)?;

        // Second pass: lower all items
        let mut globals = Vec::new();
        let mut functions = Vec::new();

        for item in module.items {
            match item {
                Item::GlobalLet(global) => {
                    let symbol = *lowerer
                        .top_level
                        .get(&global.name.name)
                        .expect("global symbol was registered");
                    globals.push(lowerer.lower_global(global, symbol)?);
                }
                Item::Function(function) => {
                    let symbol = *lowerer
                        .top_level
                        .get(&function.name.name)
                        .expect("function symbol was registered");
                    functions.push(lowerer.lower_function(function, symbol)?);
                }
                Item::Const(const_item) => {
                    let symbol = *lowerer
                        .top_level
                        .get(&const_item.name.name)
                        .expect("const symbol was registered");
                    globals.push(lowerer.lower_const(const_item, symbol)?);
                }
                Item::Agent(_agent) => {
                    // TODO: agent lowering
                }
            }
        }

        Ok(HirModule {
            globals,
            functions,
            agents: Vec::new(), // TODO: agent lowering
            symbols: lowerer.symbols,
            span,
        })
    }

    /// Collect all top-level declarations.
    fn collect_top_level(&mut self, items: &[Item]) -> HirResult<()> {
        for item in items {
            match item {
                Item::GlobalLet(global) => {
                    // Infer type from initializer for now (TODO: integrate type checker)
                    let ty = self.infer_expr_type(&global.initializer);
                    self.register_top_level(
                        &global.name.name,
                        SymbolKind::Global,
                        ty,
                        global.span,
                    )?;
                }
                Item::Function(function) => {
                    let fn_ty = self.build_function_type(function);
                    self.register_top_level(
                        &function.name.name,
                        SymbolKind::Function,
                        fn_ty,
                        function.span,
                    )?;
                }
                Item::Const(const_item) => {
                    let ty = self.infer_expr_type(&const_item.value);
                    self.register_top_level(
                        &const_item.name.name,
                        SymbolKind::Global,
                        ty,
                        const_item.span,
                    )?;
                }
                Item::Agent(agent) => {
                    // Register agent as a type
                    self.register_top_level(
                        &agent.name.name,
                        SymbolKind::Agent,
                        Type::named(&agent.name.name),
                        agent.span,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Build a function type from AST function.
    fn build_function_type(&self, function: &Function) -> Type {
        let param_types: Vec<Type> = function
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type_annotation(t))
                    .unwrap_or_else(Type::any)
            })
            .collect();

        let return_ty = function
            .ret_ty
            .as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or_else(Type::unit);

        Type::function(param_types, return_ty)
    }

    /// Resolve a type annotation from AST.
    fn resolve_type_annotation(&self, ty: &x3_ast::TypeAnnotation) -> Type {
        // Simple type name resolution
        match ty.name.name.as_str() {
            "i8" => Type::i8(),
            "i16" => Type::i16(),
            "i32" => Type::i32(),
            "i64" => Type::i64(),
            "i128" => Type::i128(),
            "u8" => Type::u8(),
            "u16" => Type::u16(),
            "u32" => Type::u32(),
            "u64" => Type::u64(),
            "u128" => Type::u128(),
            "u256" => Type::u256(),
            "bool" => Type::bool(),
            "string" => Type::string(),
            "bytes" => Type::bytes(),
            "address" => Type::address(),
            "pubkey" => Type::pubkey(),
            "()" | "unit" => Type::unit(),
            _ => Type::named(&ty.name.name),
        }
    }

    /// Infer type from expression (simple inference for globals).
    fn infer_expr_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Literal(lit) => match &lit.literal {
                x3_common::Literal::Integer(_) => Type::i64(),
                x3_common::Literal::Float(_) => Type::u64(), // TODO: float type
                x3_common::Literal::String(_) => Type::string(),
                x3_common::Literal::Bool(_) => Type::bool(),
                x3_common::Literal::Unit => Type::unit(),
            },
            Expression::Binary(binary) => {
                // For comparisons, result is bool
                match binary.op {
                    x3_ast::BinaryOp::Equal
                    | x3_ast::BinaryOp::NotEqual
                    | x3_ast::BinaryOp::Less
                    | x3_ast::BinaryOp::LessEqual
                    | x3_ast::BinaryOp::Greater
                    | x3_ast::BinaryOp::GreaterEqual => Type::bool(),
                    x3_ast::BinaryOp::LogicalAnd | x3_ast::BinaryOp::LogicalOr => Type::bool(),
                    _ => self.infer_expr_type(&binary.left),
                }
            }
            Expression::Unary(unary) => match unary.op {
                x3_ast::UnaryOp::Not => Type::bool(),
                _ => self.infer_expr_type(&unary.expr),
            },
            _ => Type::any(), // Fallback to any for complex expressions
        }
    }

    /// Register a top-level symbol.
    fn register_top_level(
        &mut self,
        name: &str,
        kind: SymbolKind,
        ty: Type,
        span: Span,
    ) -> HirResult<SymbolId> {
        if self.top_level.contains_key(name) {
            return Err(HirError::duplicate_symbol(name, span));
        }
        let id = self.allocate_symbol(name, kind, ty, span);
        self.top_level.insert(name.to_string(), id);
        Ok(id)
    }

    /// Lower a global declaration.
    fn lower_global(&mut self, global: GlobalLet, symbol: SymbolId) -> HirResult<HirGlobal> {
        let scope = ScopeStack::new();
        let initializer = self.lower_expression(&global.initializer, &scope)?;
        let ty = initializer.ty.clone();
        Ok(HirGlobal {
            symbol,
            ty,
            initializer,
            span: global.span,
        })
    }

    /// Lower a const declaration.
    fn lower_const(&mut self, const_item: x3_ast::Const, symbol: SymbolId) -> HirResult<HirGlobal> {
        let scope = ScopeStack::new();
        let initializer = self.lower_expression(&const_item.value, &scope)?;
        let ty = initializer.ty.clone();
        Ok(HirGlobal {
            symbol,
            ty,
            initializer,
            span: const_item.span,
        })
    }

    /// Lower a function declaration.
    fn lower_function(&mut self, function: Function, symbol: SymbolId) -> HirResult<HirFunction> {
        let mut scope = ScopeStack::new();
        let mut params = Vec::new();

        // Lower parameters
        for param in function.params {
            let ty = param
                .ty
                .as_ref()
                .map(|t| self.resolve_type_annotation(t))
                .unwrap_or_else(Type::any);
            let local_symbol = self.allocate_symbol(
                &param.name.name,
                SymbolKind::Param {
                    mutable: param.mutable,
                },
                ty.clone(),
                param.span,
            );

            if !scope.insert(
                &param.name.name,
                LocalInfo {
                    symbol: local_symbol,
                    ty: ty.clone(),
                    mutable: param.mutable,
                },
            ) {
                return Err(HirError::new(
                    HirErrorKind::DuplicateParam(param.name.name),
                    param.span,
                ));
            }

            params.push(HirParam {
                symbol: local_symbol,
                name: param.name.name,
                ty,
                mutable: param.mutable,
                span: param.span,
            });
        }

        // Lower body
        let body = self.lower_statements(&function.body.statements, &mut scope)?;

        // Return type
        let return_ty = function
            .ret_ty
            .as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or_else(Type::unit);

        Ok(HirFunction {
            symbol,
            params,
            body,
            return_ty,
            attrs: FunctionAttrs::default(),
            span: function.span,
        })
    }

    /// Lower a list of statements.
    fn lower_statements(
        &mut self,
        statements: &[Statement],
        scope: &mut ScopeStack,
    ) -> HirResult<Vec<HirStmt>> {
        let mut lowered = Vec::new();
        for statement in statements {
            lowered.push(self.lower_statement(statement, scope)?);
        }
        Ok(lowered)
    }

    /// Lower a single statement.
    fn lower_statement(
        &mut self,
        statement: &Statement,
        scope: &mut ScopeStack,
    ) -> HirResult<HirStmt> {
        match statement {
            Statement::Let(binding) => self.lower_let(binding, scope),
            Statement::Expr(expr) => self.lower_expression_statement(expr, scope),
            Statement::Return(value, span) => {
                let expr = value
                    .as_ref()
                    .map(|expr| self.lower_expression(expr, scope))
                    .transpose()?;
                Ok(HirStmt::Return {
                    value: expr,
                    span: *span,
                })
            }
            Statement::If(branch) => {
                let condition = self.lower_expression(&branch.condition, scope)?;
                let then_block = self.lower_block(&branch.then_block.statements, scope)?;
                let else_block = if let Some(else_block) = &branch.else_block {
                    self.lower_block(&else_block.statements, scope)?
                } else {
                    Vec::new()
                };
                Ok(HirStmt::If {
                    condition,
                    then_block,
                    else_block,
                    span: branch.span,
                })
            }
            Statement::While(loop_stmt) => {
                self.loop_depth += 1;
                let condition = self.lower_expression(&loop_stmt.condition, scope)?;
                let body = self.lower_block(&loop_stmt.body.statements, scope)?;
                self.loop_depth -= 1;
                Ok(HirStmt::While {
                    label: None,
                    condition,
                    body,
                    span: loop_stmt.span,
                })
            }
            Statement::Loop(loop_stmt) => {
                // Desugar `loop { ... }` to `while true { ... }`
                self.loop_depth += 1;
                let body = self.lower_block(&loop_stmt.body.statements, scope)?;
                self.loop_depth -= 1;

                let true_lit = HirExpr::new(
                    HirExprKind::Literal(x3_common::Literal::Bool(true)),
                    Type::bool(),
                    loop_stmt.span,
                );

                Ok(HirStmt::While {
                    label: None,
                    condition: true_lit,
                    body,
                    span: loop_stmt.span,
                })
            }
            Statement::For(for_stmt) => {
                // Desugar for loops
                self.lower_for_loop(for_stmt, scope)
            }
            Statement::Break(break_stmt) => {
                if self.loop_depth == 0 {
                    return Err(HirError::break_outside_loop(break_stmt.span));
                }
                Ok(HirStmt::Break {
                    label: None,
                    span: break_stmt.span,
                })
            }
            Statement::Continue(continue_stmt) => {
                if self.loop_depth == 0 {
                    return Err(HirError::continue_outside_loop(continue_stmt.span));
                }
                Ok(HirStmt::Continue {
                    label: None,
                    span: continue_stmt.span,
                })
            }
            Statement::Atomic(atomic_block) => {
                // Check for nested atomics
                if self.current_atomic.is_some() {
                    return Err(HirError::atomic_nesting(atomic_block.span));
                }

                let block_id = self.alloc_atomic_id();
                self.current_atomic = Some(block_id);

                // Lower the atomic block contents
                let body = self.lower_block(&atomic_block.body.statements, scope)?;

                self.current_atomic = None;

                // Wrap in begin/end markers
                // For now, flatten into the parent - the real implementation
                // would structure this as a transaction
                let mut stmts = Vec::with_capacity(body.len() + 2);
                stmts.push(HirStmt::AtomicBegin {
                    block_id,
                    span: atomic_block.span,
                });
                stmts.extend(body);
                stmts.push(HirStmt::AtomicEnd {
                    block_id,
                    commit: true,
                    span: atomic_block.span,
                });

                // Return just the begin marker - caller should handle flattening
                // For simplicity, we return a block expression as a statement
                Ok(stmts.remove(0))
            }
            Statement::Emit(emit_stmt) => {
                // For emit, we just lower the value expression
                // The emit value should be an event expression
                let value = self.lower_expression(&emit_stmt.value, scope)?;

                // For now, emit becomes an expression statement
                // In a full implementation, we'd extract event name and args
                Ok(HirStmt::Emit {
                    event_name: "event".to_string(), // TODO: extract from value
                    args: vec![value],
                    span: emit_stmt.span,
                })
            }
        }
    }

    /// Lower a for loop by desugaring to while.
    fn lower_for_loop(
        &mut self,
        for_stmt: &x3_ast::ForStatement,
        scope: &mut ScopeStack,
    ) -> HirResult<HirStmt> {
        match &for_stmt.kind {
            x3_ast::ForLoopKind::Range { variable, range } => {
                scope.push_frame();

                // Create iterator variable
                let iter_ty = Type::i64(); // TODO: proper type inference
                let iter_symbol = self.allocate_symbol(
                    &variable.name,
                    SymbolKind::Local { mutable: true },
                    iter_ty.clone(),
                    variable.span,
                );
                scope.insert(
                    &variable.name,
                    LocalInfo {
                        symbol: iter_symbol,
                        ty: iter_ty.clone(),
                        mutable: true,
                    },
                );

                // Lower range bounds
                let start = self.lower_expression(&range.start, scope)?;
                let end = self.lower_expression(&range.end, scope)?;

                // Create: let i = start
                let _init = HirStmt::Let {
                    symbol: iter_symbol,
                    ty: iter_ty.clone(),
                    value: start,
                    mutable: true,
                    span: variable.span,
                };

                // Create condition: i < end
                let iter_access = HirExpr::new(
                    HirExprKind::Var(iter_symbol),
                    iter_ty.clone(),
                    for_stmt.span,
                );
                let condition = HirExpr::new(
                    HirExprKind::Binary {
                        op: x3_ast::BinaryOp::Less,
                        left: Box::new(iter_access.clone()),
                        right: Box::new(end),
                    },
                    Type::bool(),
                    for_stmt.span,
                );

                // Lower body
                self.loop_depth += 1;
                let mut body = self.lower_block(&for_stmt.body.statements, scope)?;
                self.loop_depth -= 1;

                // Create increment: i = i + 1
                let one = HirExpr::new(
                    HirExprKind::Literal(x3_common::Literal::Integer(1)),
                    iter_ty.clone(),
                    for_stmt.span,
                );
                let inc_expr = HirExpr::new(
                    HirExprKind::Binary {
                        op: x3_ast::BinaryOp::Add,
                        left: Box::new(iter_access),
                        right: Box::new(one),
                    },
                    iter_ty,
                    for_stmt.span,
                );
                let increment = HirStmt::Assign {
                    target: AssignTarget::Variable(iter_symbol),
                    value: inc_expr,
                    span: for_stmt.span,
                };
                body.push(increment);

                scope.pop_frame();

                Ok(HirStmt::While {
                    label: None,
                    condition,
                    body,
                    span: for_stmt.span,
                })
            }
            x3_ast::ForLoopKind::CStyle {
                init,
                condition,
                update,
            } => {
                // C-style for loop: for (init; cond; update) { body }
                // Desugar to: { init; while (cond) { body; update; } }
                scope.push_frame();

                // Lower init if present
                if let Some(init_stmt) = init {
                    let _ = self.lower_statement(init_stmt, scope)?;
                }

                // Lower condition (default to true if absent)
                let cond = if let Some(c) = condition {
                    self.lower_expression(c, scope)?
                } else {
                    HirExpr::new(
                        HirExprKind::Literal(x3_common::Literal::Bool(true)),
                        Type::bool(),
                        for_stmt.span,
                    )
                };

                // Lower body
                self.loop_depth += 1;
                let mut body = self.lower_block(&for_stmt.body.statements, scope)?;
                self.loop_depth -= 1;

                // Add update if present
                if let Some(upd) = update {
                    let upd_expr = self.lower_expression(upd, scope)?;
                    body.push(HirStmt::Expr(upd_expr));
                }

                scope.pop_frame();

                Ok(HirStmt::While {
                    label: None,
                    condition: cond,
                    body,
                    span: for_stmt.span,
                })
            }
        }
    }

    /// Lower a let binding.
    fn lower_let(
        &mut self,
        binding: &x3_ast::LetStatement,
        scope: &mut ScopeStack,
    ) -> HirResult<HirStmt> {
        let value = self.lower_expression(&binding.initializer, scope)?;
        let ty = binding
            .ty
            .as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or_else(|| value.ty.clone());

        let symbol = self.allocate_symbol(
            &binding.name.name,
            SymbolKind::Local {
                mutable: binding.mutable,
            },
            ty.clone(),
            binding.span,
        );

        if !scope.insert(
            &binding.name.name,
            LocalInfo {
                symbol,
                ty: ty.clone(),
                mutable: binding.mutable,
            },
        ) {
            return Err(HirError::duplicate_symbol(&binding.name.name, binding.span));
        }

        Ok(HirStmt::Let {
            symbol,
            ty,
            value,
            mutable: binding.mutable,
            span: binding.span,
        })
    }

    /// Lower an expression statement.
    fn lower_expression_statement(
        &mut self,
        expr: &Expression,
        scope: &mut ScopeStack,
    ) -> HirResult<HirStmt> {
        // Handle assignment specially
        if let Expression::Assign(assign) = expr {
            let target_info = scope
                .lookup(&assign.target.name)
                .ok_or_else(|| HirError::unknown_symbol(&assign.target.name, assign.target.span))?;

            if !target_info.mutable {
                return Err(HirError::immutable_assign(&assign.target.name, assign.span));
            }

            let value = self.lower_expression(&assign.value, scope)?;
            return Ok(HirStmt::Assign {
                target: AssignTarget::Variable(target_info.symbol),
                value,
                span: assign.span,
            });
        }

        let expr = self.lower_expression(expr, scope)?;
        Ok(HirStmt::Expr(expr))
    }

    /// Lower a block (creates new scope frame).
    fn lower_block(
        &mut self,
        statements: &[Statement],
        scope: &mut ScopeStack,
    ) -> HirResult<Vec<HirStmt>> {
        scope.push_frame();
        let result = self.lower_statements(statements, scope);
        scope.pop_frame();
        result
    }

    /// Lower an expression.
    fn lower_expression(&mut self, expr: &Expression, scope: &ScopeStack) -> HirResult<HirExpr> {
        match expr {
            Expression::Literal(literal) => {
                let ty = match &literal.literal {
                    x3_common::Literal::Integer(_) => Type::i64(),
                    x3_common::Literal::Float(_) => Type::u64(), // TODO: float
                    x3_common::Literal::String(_) => Type::string(),
                    x3_common::Literal::Bool(_) => Type::bool(),
                    x3_common::Literal::Unit => Type::unit(),
                };
                Ok(HirExpr::new(
                    HirExprKind::Literal(literal.literal.clone()),
                    ty,
                    literal.span,
                ))
            }

            Expression::Identifier(id) => {
                // First check local scope
                if let Some(info) = scope.lookup(&id.name) {
                    return Ok(HirExpr::new(
                        HirExprKind::Var(info.symbol),
                        info.ty.clone(),
                        id.span,
                    ));
                }

                // Then check top-level
                if let Some(&symbol) = self.top_level.get(&id.name) {
                    let ty = self.symbol_type(symbol).unwrap_or_else(Type::any);
                    return Ok(HirExpr::new(HirExprKind::Var(symbol), ty, id.span));
                }

                Err(HirError::unknown_symbol(&id.name, id.span))
            }

            Expression::Binary(binary) => self.lower_binary(binary, scope),
            Expression::Unary(unary) => self.lower_unary(unary, scope),
            Expression::Call(call) => self.lower_call(call, scope),

            Expression::Assign(assign) => {
                // Assignment as expression is invalid in statement context
                Err(HirError::new(
                    HirErrorKind::InvalidAssignTarget,
                    assign.span,
                ))
            }

            Expression::FieldAccess(field_access) => {
                let object = self.lower_expression(&field_access.object, scope)?;
                // TODO: proper field type resolution from type checker
                let field_ty = Type::any();
                Ok(HirExpr::new(
                    HirExprKind::Field {
                        object: Box::new(object),
                        field: field_access.field.name.clone(),
                    },
                    field_ty,
                    field_access.span,
                ))
            }

            Expression::Range(range) => {
                // Range expressions should only appear in for-loop contexts
                // Lower to a tuple for now (start, end)
                let start = self.lower_expression(&range.start, scope)?;
                let end = self.lower_expression(&range.end, scope)?;
                let ty = Type::tuple(vec![start.ty.clone(), end.ty.clone()]);
                Ok(HirExpr::new(
                    HirExprKind::Tuple(vec![start, end]),
                    ty,
                    range.span,
                ))
            }
        }
    }

    /// Lower a binary expression.
    fn lower_binary(
        &mut self,
        binary: &BinaryExpression,
        scope: &ScopeStack,
    ) -> HirResult<HirExpr> {
        let left = self.lower_expression(&binary.left, scope)?;
        let right = self.lower_expression(&binary.right, scope)?;

        // Determine result type
        let ty = match binary.op {
            x3_ast::BinaryOp::Equal
            | x3_ast::BinaryOp::NotEqual
            | x3_ast::BinaryOp::Less
            | x3_ast::BinaryOp::LessEqual
            | x3_ast::BinaryOp::Greater
            | x3_ast::BinaryOp::GreaterEqual
            | x3_ast::BinaryOp::LogicalAnd
            | x3_ast::BinaryOp::LogicalOr => Type::bool(),
            _ => left.ty.clone(), // Arithmetic preserves operand type
        };

        Ok(HirExpr::new(
            HirExprKind::Binary {
                op: binary.op,
                left: Box::new(left),
                right: Box::new(right),
            },
            ty,
            binary.span,
        ))
    }

    /// Lower a unary expression.
    fn lower_unary(&mut self, unary: &UnaryExpression, scope: &ScopeStack) -> HirResult<HirExpr> {
        let operand = self.lower_expression(&unary.expr, scope)?;

        let ty = match unary.op {
            x3_ast::UnaryOp::Not => Type::bool(),
            _ => operand.ty.clone(),
        };

        Ok(HirExpr::new(
            HirExprKind::Unary {
                op: unary.op,
                operand: Box::new(operand),
            },
            ty,
            unary.span,
        ))
    }

    /// Lower a function call.
    fn lower_call(
        &mut self,
        call: &x3_ast::CallExpression,
        scope: &ScopeStack,
    ) -> HirResult<HirExpr> {
        // For now, only support direct function calls
        if let Expression::Identifier(callee) = &*call.callee {
            // Resolve callee
            let (callee_symbol, callee_ty) = if let Some(info) = scope.lookup(&callee.name) {
                (info.symbol, info.ty.clone())
            } else if let Some(&symbol) = self.top_level.get(&callee.name) {
                let ty = self.symbol_type(symbol).unwrap_or_else(Type::any);
                (symbol, ty)
            } else {
                return Err(HirError::unknown_symbol(&callee.name, callee.span));
            };

            // Get return type from function type and validate arity
            let return_ty = if let Some(sig) = callee_ty.as_function() {
                if sig.params.len() != call.args.len() {
                    return Err(HirError::new(
                        HirErrorKind::ArgumentCountMismatch {
                            expected: sig.params.len(),
                            found: call.args.len(),
                        },
                        call.span,
                    ));
                }
                (*sig.return_type).clone()
            } else {
                return Err(HirError::not_callable(callee_ty.clone(), call.span));
            };

            // Lower arguments
            let args: Vec<HirExpr> = call
                .args
                .iter()
                .map(|arg| self.lower_expression(arg, scope))
                .collect::<HirResult<Vec<_>>>()?;

            Ok(HirExpr::new(
                HirExprKind::Call {
                    callee: callee_symbol,
                    args,
                },
                return_ty,
                call.span,
            ))
        } else {
            Err(HirError::new(
                HirErrorKind::NotImplemented("indirect calls".to_string()),
                call.span,
            ))
        }
    }

    /// Allocate a new symbol.
    fn allocate_symbol(&mut self, name: &str, kind: SymbolKind, ty: Type, span: Span) -> SymbolId {
        let id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;
        self.symbols.push(Symbol {
            id,
            name: name.to_string(),
            kind,
            ty,
            span,
        });
        id
    }

    /// Allocate a new atomic block ID.
    fn alloc_atomic_id(&mut self) -> AtomicBlockId {
        let id = AtomicBlockId(self.next_atomic_id);
        self.next_atomic_id += 1;
        id
    }

    /// Get a symbol's type.
    fn symbol_type(&self, symbol: SymbolId) -> Option<Type> {
        self.symbols
            .iter()
            .find(|s| s.id == symbol)
            .map(|s| s.ty.clone())
    }
}

// ============================================================================
// Scope Management
// ============================================================================

#[derive(Clone)]
struct LocalInfo {
    symbol: SymbolId,
    ty: Type,
    mutable: bool,
}

struct ScopeStack {
    frames: Vec<HashMap<String, LocalInfo>>,
}

impl ScopeStack {
    fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
        }
    }

    fn push_frame(&mut self) {
        self.frames.push(HashMap::new());
    }

    fn pop_frame(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    fn insert(&mut self, name: &str, info: LocalInfo) -> bool {
        // Disallow shadowing: check any existing frame for the name.
        for frame in self.frames.iter() {
            if frame.contains_key(name) {
                return false;
            }
        }
        let frame = self.frames.last_mut().unwrap();
        frame.insert(name.to_string(), info);
        true
    }

    fn lookup(&self, name: &str) -> Option<LocalInfo> {
        for frame in self.frames.iter().rev() {
            if let Some(info) = frame.get(name) {
                return Some(info.clone());
            }
        }
        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use x3_parser::Parser;

    fn parse_and_lower(source: &str) -> HirResult<HirModule> {
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("parse");
        HirLowerer::lower(module)
    }

    #[test]
    fn lower_simple_function() {
        let source = r#"
            fn add(x: i32, y: i32) -> i32 {
                let result = x + y;
                return result;
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        assert_eq!(hir.functions.len(), 1);
        let function = &hir.functions[0];

        // Check params
        assert_eq!(function.params.len(), 2);
        assert_eq!(function.params[0].name, "x");
        assert_eq!(function.params[1].name, "y");

        // Check body has let + return
        assert!(function.body.len() >= 2);
        assert!(matches!(function.body[0], HirStmt::Let { .. }));
    }

    #[test]
    fn disallow_shadowing_across_scopes() {
        let source = r#"
            fn shadow_test() -> i32 {
                let x = 10;
                {
                    let x = 20; // shadowing should be an error now
                    return x;
                }
            }
        "#;
        let res = parse_and_lower(source);
        assert!(res.is_err(), "expected shadowing to be rejected");
    }

    #[test]
    fn call_argument_count_mismatch() {
        let source = r#"
            fn add(x: i32, y: i32) -> i32 { return x + y; }
            fn caller() -> i32 { return add(1); }
        "#;
        let res = parse_and_lower(source);
        assert!(res.is_err(), "expected argument count mismatch to error");
    }

    #[test]
    fn lower_global() {
        let source = "let PI = 3141592;";
        let hir = parse_and_lower(source).expect("lower");

        assert_eq!(hir.globals.len(), 1);
        assert!(hir.symbol_name(hir.globals[0].symbol) == Some("PI"));
    }

    #[test]
    fn lower_while_loop() {
        let source = r#"
            fn count() {
                let mut i = 0;
                while i < 10 {
                    i = i + 1;
                }
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        let body = &hir.functions[0].body;
        assert!(body.iter().any(|s| matches!(s, HirStmt::While { .. })));
    }

    #[test]
    fn lower_if_else() {
        let source = r#"
            fn max(a: i32, b: i32) -> i32 {
                if a > b {
                    return a;
                } else {
                    return b;
                }
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        let body = &hir.functions[0].body;
        let if_stmt = body.iter().find(|s| matches!(s, HirStmt::If { .. }));
        assert!(if_stmt.is_some());

        if let Some(HirStmt::If {
            then_block,
            else_block,
            ..
        }) = if_stmt
        {
            assert!(!then_block.is_empty());
            assert!(!else_block.is_empty());
        }
    }

    #[test]
    fn rejects_duplicate_params() {
        let source = "fn dup(x: i32, x: i32) {}";
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_variable() {
        let source = "fn foo() { return unknown; }";
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_immutable_assign() {
        let source = r#"
            fn foo() {
                let x = 1;
                x = 2;
            }
        "#;
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_break_outside_loop() {
        let source = "fn foo() { break; }";
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_continue_outside_loop() {
        let source = "fn foo() { continue; }";
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn allows_break_in_loop() {
        let source = r#"
            fn foo() {
                while true {
                    break;
                }
            }
        "#;
        let result = parse_and_lower(source);
        assert!(result.is_ok());
    }

    #[test]
    fn type_annotations_preserved() {
        let source = r#"
            fn typed(x: u64, y: bool) -> i32 {
                return 0;
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        let func = &hir.functions[0];
        // Check that params have correct types
        assert!(func.params[0].ty.to_string().contains("u64"));
        assert!(func.params[1].ty.to_string().contains("bool"));
    }

    #[test]
    fn expressions_carry_types() {
        let source = r#"
            fn foo() -> bool {
                return 1 < 2;
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        if let HirStmt::Return {
            value: Some(expr), ..
        } = &hir.functions[0].body[0]
        {
            // Comparison should have bool type
            assert!(expr.ty.is_bool());
        } else {
            panic!("expected return statement");
        }
    }
}
