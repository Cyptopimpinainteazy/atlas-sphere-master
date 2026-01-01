//! Type checker - walks the AST and performs type checking.
//!
//! This is the main entry point for type checking. It combines:
//! - Type environment from name resolution
//! - Type inference for expressions
//! - Constraint solving via unification
//! - Error collection and reporting

use serde::{Deserialize, Serialize};

use x3_ast::{
    Agent, AssignExpression, AtomicBlock, BinaryExpression, Block, CallExpression, Const,
    Expression, FieldAccessExpression, ForLoopKind, ForStatement, Function, GlobalLet, Identifier,
    IfStatement, Item, LetStatement, LiteralExpression, LoopStatement, Module, RangeExpression,
    Statement, UnaryExpression, WhileStatement,
};
use x3_common::{Literal, Span};
use x3_semantics::{ResolvedModule, ScopeId, SymbolId};

use crate::env::TypeEnv;
use crate::error::{TypeError, TypeErrorKind, TypeResult};
use crate::infer::TypeInference;
use crate::types::{AgentType, FunctionSignature, PrimitiveType, Type, TypeKind};

/// The result of type checking a module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypedModule {
    /// Type environment with all type bindings.
    #[serde(skip)]
    pub env: TypeEnv,
    /// Expression types indexed by span start position.
    pub expr_types: Vec<(Span, Type)>,
}

impl TypedModule {
    /// Get the type of an expression at the given span.
    pub fn type_at(&self, span: Span) -> Option<&Type> {
        self.expr_types
            .iter()
            .find(|(s, _)| s.start == span.start)
            .map(|(_, t)| t)
    }
}

/// The type checker.
pub struct TypeChecker {
    /// Type environment.
    env: TypeEnv,
    /// Collected errors.
    errors: Vec<TypeError>,
    /// Expression types.
    expr_types: Vec<(Span, Type)>,
    /// Current function's return type (for checking returns).
    current_return_type: Option<Type>,
    /// Whether we're in an atomic block.
    in_atomic: bool,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    /// Create a new type checker.
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
            expr_types: Vec::new(),
            current_return_type: None,
            in_atomic: false,
        }
    }

    /// Type check a module given the resolved symbol information.
    pub fn check(mut self, module: &Module, resolved: &ResolvedModule) -> TypeResult<TypedModule> {
        // First pass: collect all type declarations
        self.collect_declarations(module, resolved);

        // Second pass: type check all items
        for item in &module.items {
            self.check_item(item, resolved);
        }

        // Return result
        if self.errors.is_empty() {
            Ok(TypedModule {
                env: self.env,
                expr_types: self.expr_types,
            })
        } else {
            Err(self.errors)
        }
    }

    /// Collect type declarations from all items.
    fn collect_declarations(&mut self, module: &Module, resolved: &ResolvedModule) {
        for item in &module.items {
            match item {
                Item::Function(func) => self.collect_function_type(func, resolved),
                Item::Agent(agent) => self.collect_agent_type(agent, resolved),
                Item::GlobalLet(global) => self.collect_global_type(global, resolved),
                Item::Const(const_item) => self.collect_const_type(const_item, resolved),
            }
        }
    }

    /// Collect a function's type signature.
    fn collect_function_type(&mut self, func: &Function, resolved: &ResolvedModule) {
        // Parse parameter types
        let params: Vec<Type> = func
            .params
            .iter()
            .map(|p| {
                if let Some(ref ty) = p.ty {
                    self.resolve_type_annotation(ty)
                } else {
                    // No type annotation - create a type variable
                    self.env.fresh_type_var()
                }
            })
            .collect();

        // Parse return type
        let return_type = if let Some(ref ret) = func.ret_ty {
            self.resolve_type_annotation(ret)
        } else {
            // Default to unit for no return type
            Type::unit()
        };

        let sig = FunctionSignature::new(params, return_type);

        // Look up the function's symbol ID
        if let Some(symbol_id) = self.find_symbol_by_name(&func.name.name, resolved) {
            self.env.register_function(symbol_id, sig.clone());
            self.env.bind(
                ScopeId(0), // Global scope
                symbol_id,
                Type::new(TypeKind::Function(sig)),
            );
        }
    }

    /// Collect an agent's type definition.
    fn collect_agent_type(&mut self, agent: &Agent, resolved: &ResolvedModule) {
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        for item in &agent.items {
            match item {
                Item::GlobalLet(global) => {
                    let ty = if let Some(ref ann) = global.ty {
                        self.resolve_type_annotation(ann)
                    } else {
                        self.infer_expression_type(&global.initializer, resolved)
                    };
                    fields.push((global.name.name.clone(), ty));
                }
                Item::Function(func) => {
                    let params: Vec<Type> = func
                        .params
                        .iter()
                        .map(|p| {
                            if let Some(ref ty) = p.ty {
                                self.resolve_type_annotation(ty)
                            } else {
                                self.env.fresh_type_var()
                            }
                        })
                        .collect();
                    let return_type = if let Some(ref ret) = func.ret_ty {
                        self.resolve_type_annotation(ret)
                    } else {
                        Type::unit()
                    };
                    methods.push((
                        func.name.name.clone(),
                        FunctionSignature::method(params, return_type),
                    ));
                }
                Item::Const(const_item) => {
                    let ty = self.resolve_type_annotation(&const_item.ty);
                    fields.push((const_item.name.name.clone(), ty));
                }
                Item::Agent(_) => {
                    // Nested agents are handled by semantics - skip here
                }
            }
        }

        let agent_type = AgentType {
            name: agent.name.name.clone(),
            fields,
            methods,
        };

        self.env.register_agent(agent.name.name.clone(), agent_type);
    }

    /// Collect a global variable's type.
    fn collect_global_type(&mut self, global: &GlobalLet, resolved: &ResolvedModule) {
        let ty = if let Some(ref ann) = global.ty {
            self.resolve_type_annotation(ann)
        } else {
            self.infer_expression_type(&global.initializer, resolved)
        };

        if let Some(symbol_id) = self.find_symbol_by_name(&global.name.name, resolved) {
            self.env.bind(ScopeId(0), symbol_id, ty);
        }
    }

    /// Collect a constant's type.
    fn collect_const_type(&mut self, const_item: &Const, resolved: &ResolvedModule) {
        let ty = self.resolve_type_annotation(&const_item.ty);

        if let Some(symbol_id) = self.find_symbol_by_name(&const_item.name.name, resolved) {
            self.env.bind(ScopeId(0), symbol_id, ty);
        }
    }

    /// Resolve a type annotation to a Type.
    fn resolve_type_annotation(&mut self, annotation: &x3_ast::TypeAnnotation) -> Type {
        // For now, simple name-based lookup
        if let Some(ty) = self.env.lookup_type(&annotation.name.name) {
            return ty.clone();
        }

        // Handle common generic type patterns by parsing the name
        // e.g., "vec<u64>" would be parsed if TypeAnnotation supports it
        // For now, just return named type since AST doesn't have type_args
        Type::named(&annotation.name.name)
    }

    /// Type check an item.
    fn check_item(&mut self, item: &Item, resolved: &ResolvedModule) {
        match item {
            Item::Function(func) => self.check_function(func, resolved),
            Item::Agent(agent) => self.check_agent(agent, resolved),
            Item::GlobalLet(global) => self.check_global_let(global, resolved),
            Item::Const(const_item) => self.check_const(const_item, resolved),
        }
    }

    /// Type check a function.
    fn check_function(&mut self, func: &Function, resolved: &ResolvedModule) {
        // Get the function's signature
        let sig = if let Some(symbol_id) = self.find_symbol_by_name(&func.name.name, resolved) {
            self.env.get_function_sig(symbol_id).cloned()
        } else {
            None
        };

        let return_type = sig
            .as_ref()
            .map(|s| s.return_type.as_ref().clone())
            .unwrap_or_else(Type::unit);

        // Bind parameter types to the environment
        if let Some(ref sig) = sig {
            for (param, param_ty) in func.params.iter().zip(sig.params.iter()) {
                if let Some(symbol_id) = self.find_symbol_by_name(&param.name.name, resolved) {
                    self.env.bind(ScopeId(0), symbol_id, param_ty.clone());
                }
            }
        }

        // Set current return type for checking return statements
        let prev_return_type = self.current_return_type.take();
        self.current_return_type = Some(return_type.clone());

        // Type check the body
        self.check_block(&func.body, resolved);

        // Restore previous return type
        self.current_return_type = prev_return_type;
    }

    /// Type check an agent.
    fn check_agent(&mut self, agent: &Agent, resolved: &ResolvedModule) {
        for item in &agent.items {
            self.check_item(item, resolved);
        }
    }

    /// Type check a global let.
    fn check_global_let(&mut self, global: &GlobalLet, resolved: &ResolvedModule) {
        let init_type = self.infer_expression_type(&global.initializer, resolved);

        if let Some(ref ann) = global.ty {
            let declared_type = self.resolve_type_annotation(ann);
            self.check_type_compatibility(&declared_type, &init_type, global.span);
        }
    }

    /// Type check a const.
    fn check_const(&mut self, const_item: &Const, resolved: &ResolvedModule) {
        let declared_type = self.resolve_type_annotation(&const_item.ty);
        let init_type = self.infer_expression_type(&const_item.value, resolved);
        self.check_type_compatibility(&declared_type, &init_type, const_item.span);
    }

    /// Type check a block.
    fn check_block(&mut self, block: &Block, resolved: &ResolvedModule) {
        for stmt in &block.statements {
            self.check_statement(stmt, resolved);
        }
    }

    /// Type check a statement.
    fn check_statement(&mut self, stmt: &Statement, resolved: &ResolvedModule) {
        match stmt {
            Statement::Let(let_stmt) => self.check_let_statement(let_stmt, resolved),
            Statement::Expr(expr) => {
                self.infer_expression_type(expr, resolved);
            }
            Statement::Return(value, span) => self.check_return(value.as_ref(), *span, resolved),
            Statement::If(if_stmt) => self.check_if_statement(if_stmt, resolved),
            Statement::While(while_stmt) => self.check_while_statement(while_stmt, resolved),
            Statement::Loop(loop_stmt) => self.check_loop_statement(loop_stmt, resolved),
            Statement::For(for_stmt) => self.check_for_statement(for_stmt, resolved),
            Statement::Atomic(atomic) => self.check_atomic_block(atomic, resolved),
            Statement::Emit(emit) => self.check_emit_statement(emit, resolved),
            Statement::Break(_) | Statement::Continue(_) => {
                // These are validated by semantics, no type checking needed
            }
        }
    }

    /// Type check a let statement.
    fn check_let_statement(&mut self, let_stmt: &LetStatement, resolved: &ResolvedModule) {
        let init_type = self.infer_expression_type(&let_stmt.initializer, resolved);

        if let Some(ref ann) = let_stmt.ty {
            let declared_type = self.resolve_type_annotation(ann);
            self.check_type_compatibility(&declared_type, &init_type, let_stmt.span);
        }

        // Bind the variable's type
        if let Some(symbol_id) = self.find_symbol_by_name(&let_stmt.name.name, resolved) {
            self.env.bind(ScopeId(0), symbol_id, init_type);
        }
    }

    /// Type check a return statement.
    fn check_return(&mut self, value: Option<&Expression>, span: Span, resolved: &ResolvedModule) {
        let expected = self.current_return_type.clone().unwrap_or_else(Type::unit);

        let actual = if let Some(expr) = value {
            self.infer_expression_type(expr, resolved)
        } else {
            Type::unit()
        };

        self.check_type_compatibility(&expected, &actual, span);
    }

    /// Type check an if statement.
    fn check_if_statement(&mut self, if_stmt: &IfStatement, resolved: &ResolvedModule) {
        // Check condition is bool
        let cond_type = self.infer_expression_type(&if_stmt.condition, resolved);
        if !cond_type.is_bool() && !cond_type.is_error() {
            self.errors.push(TypeError::condition_not_bool(
                cond_type,
                if_stmt.condition.span(),
            ));
        }

        // Check branches
        self.check_block(&if_stmt.then_block, resolved);
        if let Some(ref else_block) = if_stmt.else_block {
            self.check_block(else_block, resolved);
        }
    }

    /// Type check a while statement.
    fn check_while_statement(&mut self, while_stmt: &WhileStatement, resolved: &ResolvedModule) {
        let cond_type = self.infer_expression_type(&while_stmt.condition, resolved);
        if !cond_type.is_bool() && !cond_type.is_error() {
            self.errors.push(TypeError::condition_not_bool(
                cond_type,
                while_stmt.condition.span(),
            ));
        }

        self.check_block(&while_stmt.body, resolved);
    }

    /// Type check a loop statement.
    fn check_loop_statement(&mut self, loop_stmt: &LoopStatement, resolved: &ResolvedModule) {
        self.check_block(&loop_stmt.body, resolved);
    }

    /// Type check a for statement.
    fn check_for_statement(&mut self, for_stmt: &ForStatement, resolved: &ResolvedModule) {
        match &for_stmt.kind {
            ForLoopKind::CStyle {
                init,
                condition,
                update,
            } => {
                if let Some(init) = init {
                    self.check_statement(init, resolved);
                }
                if let Some(cond) = condition {
                    let cond_type = self.infer_expression_type(cond, resolved);
                    if !cond_type.is_bool() && !cond_type.is_error() {
                        self.errors
                            .push(TypeError::condition_not_bool(cond_type, cond.span()));
                    }
                }
                if let Some(update) = update {
                    self.infer_expression_type(update, resolved);
                }
            }
            ForLoopKind::Range { range, .. } => {
                // Type check the range bounds
                self.infer_expression_type(&range.start, resolved);
                self.infer_expression_type(&range.end, resolved);
            }
        }

        self.check_block(&for_stmt.body, resolved);
    }

    /// Type check an atomic block.
    fn check_atomic_block(&mut self, atomic: &AtomicBlock, resolved: &ResolvedModule) {
        let prev_in_atomic = self.in_atomic;
        self.in_atomic = true;

        self.check_block(&atomic.body, resolved);

        self.in_atomic = prev_in_atomic;
    }

    /// Type check an emit statement.
    fn check_emit_statement(&mut self, emit: &x3_ast::EmitStatement, resolved: &ResolvedModule) {
        // Type check the emitted value
        self.infer_expression_type(&emit.value, resolved);
    }

    /// Infer the type of an expression.
    fn infer_expression_type(&mut self, expr: &Expression, resolved: &ResolvedModule) -> Type {
        let ty = match expr {
            Expression::Literal(lit) => self.infer_literal_type(lit),
            Expression::Identifier(ident) => self.infer_identifier_type(ident, resolved),
            Expression::Binary(bin) => self.infer_binary_type(bin, resolved),
            Expression::Unary(unary) => self.infer_unary_type(unary, resolved),
            Expression::Call(call) => self.infer_call_type(call, resolved),
            Expression::Assign(assign) => self.infer_assign_type(assign, resolved),
            Expression::FieldAccess(field) => self.infer_field_access_type(field, resolved),
            Expression::Range(range) => self.infer_range_type(range, resolved),
        };

        // Record expression type
        self.expr_types.push((expr.span(), ty.clone()));

        ty
    }

    /// Infer type of a literal.
    fn infer_literal_type(&self, lit: &LiteralExpression) -> Type {
        match &lit.literal {
            Literal::Integer(n) => {
                // Default to u64 for positive, i64 for negative
                if *n < 0 {
                    Type::i64()
                } else {
                    Type::u64()
                }
            }
            Literal::Float(_) => Type::new(TypeKind::Primitive(PrimitiveType::U64)), // TODO: Add float type
            Literal::String(_) => Type::string(),
            Literal::Bool(_) => Type::bool(),
            Literal::Unit => Type::unit(),
        }
    }

    /// Infer type of an identifier.
    fn infer_identifier_type(&mut self, ident: &Identifier, resolved: &ResolvedModule) -> Type {
        // Look up the symbol's type
        if let Some(symbol_id) = self.find_symbol_by_name(&ident.name, resolved) {
            if let Some(ty) = self.env.get(symbol_id) {
                return ty.clone();
            }
        }

        // Not found - return error type and record error
        self.errors
            .push(TypeError::unknown_type(&ident.name, ident.span));
        Type::error()
    }

    /// Infer type of a binary expression.
    fn infer_binary_type(&mut self, bin: &BinaryExpression, resolved: &ResolvedModule) -> Type {
        let left_type = self.infer_expression_type(&bin.left, resolved);
        let right_type = self.infer_expression_type(&bin.right, resolved);

        let op = format!("{:?}", bin.op);
        let mut infer = TypeInference::new(&mut self.env);

        match infer.infer_binary_op(&op, &left_type, &right_type, bin.span) {
            Ok(ty) => ty,
            Err(err) => {
                self.errors.push(err);
                Type::error()
            }
        }
    }

    /// Infer type of a unary expression.
    fn infer_unary_type(&mut self, unary: &UnaryExpression, resolved: &ResolvedModule) -> Type {
        let operand_type = self.infer_expression_type(&unary.expr, resolved);

        let op = format!("{:?}", unary.op);
        let mut infer = TypeInference::new(&mut self.env);

        match infer.infer_unary_op(&op, &operand_type, unary.span) {
            Ok(ty) => ty,
            Err(err) => {
                self.errors.push(err);
                Type::error()
            }
        }
    }

    /// Infer type of a function call.
    fn infer_call_type(&mut self, call: &CallExpression, resolved: &ResolvedModule) -> Type {
        // Get the callee type
        let callee_type = self.infer_expression_type(&call.callee, resolved);

        match &callee_type.kind {
            TypeKind::Function(sig) => {
                // Check argument count
                if call.args.len() != sig.params.len() {
                    self.errors.push(TypeError::wrong_argument_count(
                        sig.params.len(),
                        call.args.len(),
                        call.span,
                    ));
                    return Type::error();
                }

                // Check argument types
                for (i, (arg, param_ty)) in call.args.iter().zip(sig.params.iter()).enumerate() {
                    let arg_type = self.infer_expression_type(arg, resolved);
                    if !self.types_compatible(param_ty, &arg_type) {
                        self.errors.push(TypeError::argument_type_mismatch(
                            i,
                            param_ty.clone(),
                            arg_type,
                            arg.span(),
                        ));
                    }
                }

                sig.return_type.as_ref().clone()
            }
            TypeKind::Error => Type::error(),
            _ => {
                self.errors
                    .push(TypeError::not_callable(callee_type, call.span));
                Type::error()
            }
        }
    }

    /// Infer type of an assignment.
    fn infer_assign_type(&mut self, assign: &AssignExpression, resolved: &ResolvedModule) -> Type {
        let target_type = self.infer_identifier_type(&assign.target, resolved);
        let value_type = self.infer_expression_type(&assign.value, resolved);

        self.check_type_compatibility(&target_type, &value_type, assign.span);

        target_type
    }

    /// Infer type of a field access.
    fn infer_field_access_type(
        &mut self,
        field: &FieldAccessExpression,
        resolved: &ResolvedModule,
    ) -> Type {
        let object_type = self.infer_expression_type(&field.object, resolved);

        // Look up field in agent type
        match &object_type.kind {
            TypeKind::Agent(agent) => {
                if let Some((_, ty)) = agent
                    .fields
                    .iter()
                    .find(|(name, _)| *name == field.field.name)
                {
                    return ty.clone();
                }
                self.errors.push(TypeError::no_field(
                    object_type,
                    &field.field.name,
                    field.span,
                ));
                Type::error()
            }
            TypeKind::Error => Type::error(),
            _ => {
                self.errors.push(TypeError::no_field(
                    object_type,
                    &field.field.name,
                    field.span,
                ));
                Type::error()
            }
        }
    }

    /// Infer type of a range expression.
    fn infer_range_type(&mut self, range: &RangeExpression, resolved: &ResolvedModule) -> Type {
        let start_type = self.infer_expression_type(&range.start, resolved);
        let end_type = self.infer_expression_type(&range.end, resolved);

        if !start_type.is_numeric() || !end_type.is_numeric() {
            self.errors.push(TypeError::new(
                TypeErrorKind::InvalidRangeBounds,
                range.span,
            ));
        }

        // Range type is Range<T> where T is the bound type
        // For now, just return the start type wrapped in a "range"
        Type::named("Range") // TODO: Proper range type
    }

    /// Check if two types are compatible.
    fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
        // Error type is compatible with everything (for error recovery)
        if expected.is_error() || found.is_error() {
            return true;
        }

        // Never type is compatible with everything
        if expected.is_never() || found.is_never() {
            return true;
        }

        // Any type is compatible with everything
        if matches!(expected.kind, TypeKind::Any) || matches!(found.kind, TypeKind::Any) {
            return true;
        }

        // Type variables need unification
        if expected.is_type_var() || found.is_type_var() {
            return true; // Will be resolved by inference
        }

        // Direct comparison
        expected.kind == found.kind
    }

    /// Check type compatibility and report error if incompatible.
    fn check_type_compatibility(&mut self, expected: &Type, found: &Type, span: Span) {
        if !self.types_compatible(expected, found) {
            self.errors.push(TypeError::type_mismatch(
                expected.clone(),
                found.clone(),
                span,
            ));
        }
    }

    /// Find a symbol ID by name in the resolved module.
    fn find_symbol_by_name(&self, name: &str, resolved: &ResolvedModule) -> Option<SymbolId> {
        resolved
            .symbols
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_checker_creation() {
        let checker = TypeChecker::new();
        assert!(checker.errors.is_empty());
    }
}
