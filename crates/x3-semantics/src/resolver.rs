//! Name resolution and semantic analysis.
//!
//! The resolver walks the AST and:
//! 1. Builds the scope tree
//! 2. Populates the symbol table
//! 3. Resolves identifier references to their definitions
//! 4. Validates semantic constraints (break/continue/return contexts)

use serde::{Deserialize, Serialize};

use x3_ast::{
    Agent, AssignExpression, AtomicBlock, BinaryExpression, Block, CallExpression, Const,
    EmitStatement, Expression, FieldAccessExpression, ForLoopKind, ForStatement, Function,
    GlobalLet, Identifier, IfStatement, Item, LetStatement, LoopStatement, Module, RangeExpression,
    Statement, UnaryExpression, WhileStatement,
};
use x3_common::Span;

use crate::error::SemanticError;
use crate::scope::{ScopeId, ScopeKind, ScopeTree};
use crate::symbol::{SymbolId, SymbolKind, SymbolTable};

/// The result of semantic analysis on a module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedModule {
    /// The scope tree built during analysis.
    pub scopes: ScopeTree,
    /// The symbol table with all definitions.
    pub symbols: SymbolTable,
    /// Mapping from identifier spans to their resolved symbols.
    #[serde(skip)]
    pub resolutions: Vec<(Span, SymbolId)>,
}

impl ResolvedModule {
    pub fn new(
        scopes: ScopeTree,
        symbols: SymbolTable,
        resolutions: Vec<(Span, SymbolId)>,
    ) -> Self {
        Self {
            scopes,
            symbols,
            resolutions,
        }
    }

    /// Look up the symbol for a given identifier span.
    pub fn resolve_span(&self, span: Span) -> Option<SymbolId> {
        self.resolutions
            .iter()
            .find(|(s, _)| *s == span)
            .map(|(_, id)| *id)
    }
}

/// The semantic resolver - performs name resolution and scope analysis.
pub struct Resolver {
    /// The scope tree being built.
    scopes: ScopeTree,
    /// The symbol table being populated.
    symbols: SymbolTable,
    /// Current scope we're analyzing.
    current_scope: ScopeId,
    /// Collected errors during analysis.
    errors: Vec<SemanticError>,
    /// Mapping from identifier spans to resolved symbols.
    resolutions: Vec<(Span, SymbolId)>,
}

impl Resolver {
    /// Create a new resolver.
    pub fn new() -> Self {
        Self {
            scopes: ScopeTree::new(),
            symbols: SymbolTable::new(),
            current_scope: ScopeId::ROOT,
            errors: Vec::new(),
            resolutions: Vec::new(),
        }
    }

    /// Resolve a module, returning the resolved module or errors.
    pub fn resolve(mut self, module: &Module) -> Result<ResolvedModule, Vec<SemanticError>> {
        // Update global scope span
        if let Some(scope) = self.scopes.get_mut(ScopeId::ROOT) {
            scope.span = module.span;
        }

        // First pass: collect all top-level declarations (forward declarations)
        self.collect_top_level_declarations(module);

        // Second pass: resolve bodies
        self.resolve_module(module);

        if self.errors.is_empty() {
            Ok(ResolvedModule::new(
                self.scopes,
                self.symbols,
                self.resolutions,
            ))
        } else {
            Err(self.errors)
        }
    }

    /// Collect top-level declarations first (for forward references).
    fn collect_top_level_declarations(&mut self, module: &Module) {
        for item in &module.items {
            match item {
                Item::Function(func) => {
                    self.declare_function(func);
                }
                Item::GlobalLet(global) => {
                    self.declare_global_let(global);
                }
                Item::Const(const_item) => {
                    self.declare_const(const_item);
                }
                Item::Agent(agent) => {
                    self.declare_agent(agent);
                }
            }
        }
    }

    fn declare_function(&mut self, func: &Function) {
        let existing = self
            .scopes
            .get(self.current_scope)
            .and_then(|s| s.lookup_local(&func.name.name));

        if let Some(existing_id) = existing {
            let original_span = self
                .symbols
                .get(existing_id)
                .map(|s| s.def_span)
                .unwrap_or_default();
            self.errors.push(SemanticError::duplicate_name(
                &func.name.name,
                func.name.span,
                original_span,
            ));
            return;
        }

        let symbol_id = self.symbols.create(
            func.name.name.clone(),
            SymbolKind::Function {
                param_count: func.params.len(),
                has_return_type: func.ret_ty.is_some(),
            },
            self.current_scope,
            func.name.span,
        );
        self.scopes
            .define(self.current_scope, func.name.name.clone(), symbol_id);
    }

    fn declare_global_let(&mut self, global: &GlobalLet) {
        let existing = self
            .scopes
            .get(self.current_scope)
            .and_then(|s| s.lookup_local(&global.name.name));

        if let Some(existing_id) = existing {
            let original_span = self
                .symbols
                .get(existing_id)
                .map(|s| s.def_span)
                .unwrap_or_default();
            self.errors.push(SemanticError::duplicate_name(
                &global.name.name,
                global.name.span,
                original_span,
            ));
            return;
        }

        let symbol_id = self.symbols.create(
            global.name.name.clone(),
            SymbolKind::GlobalVariable {
                mutable: global.mutable,
            },
            self.current_scope,
            global.name.span,
        );
        self.scopes
            .define(self.current_scope, global.name.name.clone(), symbol_id);
    }

    fn declare_const(&mut self, const_item: &Const) {
        let existing = self
            .scopes
            .get(self.current_scope)
            .and_then(|s| s.lookup_local(&const_item.name.name));

        if let Some(existing_id) = existing {
            let original_span = self
                .symbols
                .get(existing_id)
                .map(|s| s.def_span)
                .unwrap_or_default();
            self.errors.push(SemanticError::duplicate_name(
                &const_item.name.name,
                const_item.name.span,
                original_span,
            ));
            return;
        }

        let symbol_id = self.symbols.create(
            const_item.name.name.clone(),
            SymbolKind::Constant,
            self.current_scope,
            const_item.name.span,
        );
        self.scopes
            .define(self.current_scope, const_item.name.name.clone(), symbol_id);
    }

    fn declare_agent(&mut self, agent: &Agent) {
        let existing = self
            .scopes
            .get(self.current_scope)
            .and_then(|s| s.lookup_local(&agent.name.name));

        if let Some(existing_id) = existing {
            let original_span = self
                .symbols
                .get(existing_id)
                .map(|s| s.def_span)
                .unwrap_or_default();
            self.errors.push(SemanticError::duplicate_name(
                &agent.name.name,
                agent.name.span,
                original_span,
            ));
            return;
        }

        let symbol_id = self.symbols.create(
            agent.name.name.clone(),
            SymbolKind::Agent,
            self.current_scope,
            agent.name.span,
        );
        self.scopes
            .define(self.current_scope, agent.name.name.clone(), symbol_id);
    }

    /// Resolve the module bodies.
    fn resolve_module(&mut self, module: &Module) {
        for item in &module.items {
            self.resolve_item(item);
        }
    }

    fn resolve_item(&mut self, item: &Item) {
        match item {
            Item::Function(func) => self.resolve_function(func),
            Item::GlobalLet(global) => self.resolve_global_let(global),
            Item::Const(const_item) => self.resolve_const(const_item),
            Item::Agent(agent) => self.resolve_agent(agent),
        }
    }

    fn resolve_function(&mut self, func: &Function) {
        // Create function scope
        let func_scope = self.scopes.create_scope(
            ScopeKind::Function,
            Some(self.current_scope),
            func.body.span,
        );

        let prev_scope = self.current_scope;
        self.current_scope = func_scope;

        // Declare parameters
        for (index, param) in func.params.iter().enumerate() {
            let existing = self
                .scopes
                .get(self.current_scope)
                .and_then(|s| s.lookup_local(&param.name.name));

            if let Some(existing_id) = existing {
                let original_span = self
                    .symbols
                    .get(existing_id)
                    .map(|s| s.def_span)
                    .unwrap_or_default();
                self.errors.push(SemanticError::duplicate_name(
                    &param.name.name,
                    param.name.span,
                    original_span,
                ));
                continue;
            }

            let symbol_id = self.symbols.create(
                param.name.name.clone(),
                SymbolKind::Parameter {
                    mutable: param.mutable,
                    index,
                },
                self.current_scope,
                param.name.span,
            );
            self.scopes
                .define(self.current_scope, param.name.name.clone(), symbol_id);
        }

        // Resolve function body
        self.resolve_block(&func.body);

        self.current_scope = prev_scope;
    }

    fn resolve_global_let(&mut self, global: &GlobalLet) {
        self.resolve_expression(&global.initializer);
    }

    fn resolve_const(&mut self, const_item: &Const) {
        self.resolve_expression(&const_item.value);
    }

    fn resolve_agent(&mut self, agent: &Agent) {
        // Create agent scope
        let agent_scope =
            self.scopes
                .create_scope(ScopeKind::Agent, Some(self.current_scope), agent.span);

        let prev_scope = self.current_scope;
        self.current_scope = agent_scope;

        // First pass: collect declarations within agent
        for item in &agent.items {
            match item {
                Item::Function(func) => self.declare_function(func),
                Item::GlobalLet(global) => self.declare_global_let(global),
                Item::Const(const_item) => self.declare_const(const_item),
                Item::Agent(_) => {
                    self.errors.push(SemanticError::new(
                        crate::error::SemanticErrorKind::NestedAgent,
                        agent.span,
                    ));
                }
            }
        }

        // Second pass: resolve bodies
        for item in &agent.items {
            self.resolve_item(item);
        }

        self.current_scope = prev_scope;
    }

    fn resolve_block(&mut self, block: &Block) {
        for statement in &block.statements {
            self.resolve_statement(statement);
        }
    }

    fn resolve_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Let(let_stmt) => self.resolve_let_statement(let_stmt),
            Statement::Expr(expr) => self.resolve_expression(expr),
            Statement::Return(value, span) => self.resolve_return(value.as_ref(), *span),
            Statement::If(if_stmt) => self.resolve_if_statement(if_stmt),
            Statement::While(while_stmt) => self.resolve_while_statement(while_stmt),
            Statement::Loop(loop_stmt) => self.resolve_loop_statement(loop_stmt),
            Statement::For(for_stmt) => self.resolve_for_statement(for_stmt),
            Statement::Atomic(atomic) => self.resolve_atomic_block(atomic),
            Statement::Emit(emit) => self.resolve_emit_statement(emit),
            Statement::Break(break_stmt) => self.resolve_break(break_stmt.span),
            Statement::Continue(continue_stmt) => self.resolve_continue(continue_stmt.span),
        }
    }

    fn resolve_let_statement(&mut self, let_stmt: &LetStatement) {
        // First resolve the initializer (before adding the variable to scope)
        self.resolve_expression(&let_stmt.initializer);

        // Check for duplicate in current scope
        let existing = self
            .scopes
            .get(self.current_scope)
            .and_then(|s| s.lookup_local(&let_stmt.name.name));

        if let Some(existing_id) = existing {
            let original_span = self
                .symbols
                .get(existing_id)
                .map(|s| s.def_span)
                .unwrap_or_default();
            self.errors.push(SemanticError::duplicate_name(
                &let_stmt.name.name,
                let_stmt.name.span,
                original_span,
            ));
            return;
        }

        // Add variable to current scope
        let symbol_id = self.symbols.create(
            let_stmt.name.name.clone(),
            SymbolKind::Variable {
                mutable: let_stmt.mutable,
            },
            self.current_scope,
            let_stmt.name.span,
        );
        self.scopes
            .define(self.current_scope, let_stmt.name.name.clone(), symbol_id);
    }

    fn resolve_return(&mut self, value: Option<&Expression>, span: Span) {
        if !self.scopes.is_in_function(self.current_scope) {
            self.errors.push(SemanticError::invalid_return(span));
        }
        if let Some(expr) = value {
            self.resolve_expression(expr);
        }
    }

    fn resolve_break(&mut self, span: Span) {
        if !self.scopes.is_in_loop(self.current_scope) {
            self.errors.push(SemanticError::invalid_break(span));
        }
    }

    fn resolve_continue(&mut self, span: Span) {
        if !self.scopes.is_in_loop(self.current_scope) {
            self.errors.push(SemanticError::invalid_continue(span));
        }
    }

    fn resolve_if_statement(&mut self, if_stmt: &IfStatement) {
        self.resolve_expression(&if_stmt.condition);

        // Create block scope for then branch
        let then_scope = self.scopes.create_scope(
            ScopeKind::Block,
            Some(self.current_scope),
            if_stmt.then_block.span,
        );
        let prev_scope = self.current_scope;
        self.current_scope = then_scope;
        self.resolve_block(&if_stmt.then_block);
        self.current_scope = prev_scope;

        // Create block scope for else branch if present
        if let Some(else_block) = &if_stmt.else_block {
            let else_scope = self.scopes.create_scope(
                ScopeKind::Block,
                Some(self.current_scope),
                else_block.span,
            );
            self.current_scope = else_scope;
            self.resolve_block(else_block);
            self.current_scope = prev_scope;
        }
    }

    fn resolve_while_statement(&mut self, while_stmt: &WhileStatement) {
        self.resolve_expression(&while_stmt.condition);

        // Create loop scope for body
        let loop_scope = self.scopes.create_scope(
            ScopeKind::Loop,
            Some(self.current_scope),
            while_stmt.body.span,
        );
        let prev_scope = self.current_scope;
        self.current_scope = loop_scope;
        self.resolve_block(&while_stmt.body);
        self.current_scope = prev_scope;
    }

    fn resolve_loop_statement(&mut self, loop_stmt: &LoopStatement) {
        // Create loop scope for body
        let loop_scope = self.scopes.create_scope(
            ScopeKind::Loop,
            Some(self.current_scope),
            loop_stmt.body.span,
        );
        let prev_scope = self.current_scope;
        self.current_scope = loop_scope;
        self.resolve_block(&loop_stmt.body);
        self.current_scope = prev_scope;
    }

    fn resolve_for_statement(&mut self, for_stmt: &ForStatement) {
        // Create loop scope for the entire for statement
        let loop_scope =
            self.scopes
                .create_scope(ScopeKind::Loop, Some(self.current_scope), for_stmt.span);
        let prev_scope = self.current_scope;
        self.current_scope = loop_scope;

        match &for_stmt.kind {
            ForLoopKind::CStyle {
                init,
                condition,
                update,
            } => {
                // Resolve init
                if let Some(init_stmt) = init {
                    self.resolve_statement(init_stmt);
                }
                // Resolve condition
                if let Some(cond) = condition {
                    self.resolve_expression(cond);
                }
                // Resolve update
                if let Some(update_expr) = update {
                    self.resolve_expression(update_expr);
                }
            }
            ForLoopKind::Range { variable, range } => {
                // Declare loop variable
                let symbol_id = self.symbols.create(
                    variable.name.clone(),
                    SymbolKind::LoopVariable,
                    self.current_scope,
                    variable.span,
                );
                self.scopes
                    .define(self.current_scope, variable.name.clone(), symbol_id);

                // Resolve range expression
                self.resolve_range_expression(range);
            }
        }

        // Resolve body
        self.resolve_block(&for_stmt.body);

        self.current_scope = prev_scope;
    }

    fn resolve_atomic_block(&mut self, atomic: &AtomicBlock) {
        // Check for nested atomic
        if self.scopes.is_in_atomic(self.current_scope) {
            self.errors.push(SemanticError::new(
                crate::error::SemanticErrorKind::NestedAtomic,
                atomic.span,
            ));
        }

        // Resolve metadata if present
        if let Some(metadata) = &atomic.metadata {
            self.resolve_expression(metadata);
        }

        // Create atomic scope for body
        let atomic_scope = self.scopes.create_scope(
            ScopeKind::Atomic,
            Some(self.current_scope),
            atomic.body.span,
        );
        let prev_scope = self.current_scope;
        self.current_scope = atomic_scope;
        self.resolve_block(&atomic.body);
        self.current_scope = prev_scope;
    }

    fn resolve_emit_statement(&mut self, emit: &EmitStatement) {
        self.resolve_expression(&emit.value);
    }

    fn resolve_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Literal(_) => {} // Nothing to resolve
            Expression::Identifier(ident) => self.resolve_identifier(ident),
            Expression::Binary(binary) => self.resolve_binary_expression(binary),
            Expression::Unary(unary) => self.resolve_unary_expression(unary),
            Expression::Call(call) => self.resolve_call_expression(call),
            Expression::Assign(assign) => self.resolve_assign_expression(assign),
            Expression::FieldAccess(field_access) => self.resolve_field_access(field_access),
            Expression::Range(range) => self.resolve_range_expression(range),
        }
    }

    fn resolve_identifier(&mut self, ident: &Identifier) {
        if let Some(symbol_id) = self.scopes.lookup(self.current_scope, &ident.name) {
            // Record the resolution
            self.resolutions.push((ident.span, symbol_id));
            // Add reference to symbol
            self.symbols.add_reference(symbol_id, ident.span);
        } else {
            self.errors
                .push(SemanticError::undefined_variable(&ident.name, ident.span));
        }
    }

    fn resolve_binary_expression(&mut self, binary: &BinaryExpression) {
        self.resolve_expression(&binary.left);
        self.resolve_expression(&binary.right);
    }

    fn resolve_unary_expression(&mut self, unary: &UnaryExpression) {
        self.resolve_expression(&unary.expr);
    }

    fn resolve_call_expression(&mut self, call: &CallExpression) {
        // Resolve the callee
        self.resolve_expression(&call.callee);

        // Resolve arguments
        for arg in &call.args {
            self.resolve_expression(arg);
        }
    }

    fn resolve_assign_expression(&mut self, assign: &AssignExpression) {
        // First resolve the value
        self.resolve_expression(&assign.value);

        // Then resolve the target and check mutability
        if let Some(symbol_id) = self.scopes.lookup(self.current_scope, &assign.target.name) {
            self.resolutions.push((assign.target.span, symbol_id));
            self.symbols.add_reference(symbol_id, assign.target.span);

            // Check mutability
            if let Some(symbol) = self.symbols.get(symbol_id) {
                if !symbol.kind.is_mutable() {
                    self.errors.push(SemanticError::assignment_to_immutable(
                        &assign.target.name,
                        assign.target.span,
                    ));
                }
            }
        } else {
            self.errors.push(SemanticError::undefined_variable(
                &assign.target.name,
                assign.target.span,
            ));
        }
    }

    fn resolve_field_access(&mut self, field_access: &FieldAccessExpression) {
        self.resolve_expression(&field_access.object);
        // Field name is not resolved here - it's resolved during type checking
    }

    fn resolve_range_expression(&mut self, range: &RangeExpression) {
        self.resolve_expression(&range.start);
        self.resolve_expression(&range.end);
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::{Block, Function, Identifier, Item, LiteralExpression, Module};
    use x3_common::Literal;

    fn make_simple_module() -> Module {
        Module {
            items: vec![Item::Function(Function {
                name: Identifier {
                    name: "main".to_string(),
                    span: Span::new(3, 7),
                },
                params: vec![],
                ret_ty: None,
                body: Block {
                    statements: vec![Statement::Return(
                        Some(Expression::Literal(LiteralExpression {
                            literal: Literal::Integer(0),
                            span: Span::new(20, 21),
                        })),
                        Span::new(13, 22),
                    )],
                    span: Span::new(10, 25),
                },
                span: Span::new(0, 25),
            })],
            span: Span::new(0, 25),
        }
    }

    #[test]
    fn test_simple_function_resolution() {
        let module = make_simple_module();
        let resolver = Resolver::new();
        let result = resolver.resolve(&module);

        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.symbols.len(), 1); // Just the function
        assert_eq!(resolved.scopes.len(), 2); // Global + function
    }

    #[test]
    fn test_undefined_variable_error() {
        let module = Module {
            items: vec![Item::Function(Function {
                name: Identifier {
                    name: "test".to_string(),
                    span: Span::new(3, 7),
                },
                params: vec![],
                ret_ty: None,
                body: Block {
                    statements: vec![Statement::Return(
                        Some(Expression::Identifier(Identifier {
                            name: "x".to_string(),
                            span: Span::new(20, 21),
                        })),
                        Span::new(13, 22),
                    )],
                    span: Span::new(10, 25),
                },
                span: Span::new(0, 25),
            })],
            span: Span::new(0, 25),
        };

        let resolver = Resolver::new();
        let result = resolver.resolve(&module);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            crate::error::SemanticErrorKind::UndefinedVariable(_)
        ));
    }
}
