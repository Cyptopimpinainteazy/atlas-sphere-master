//! HIR → Bytecode Lowering
//!
//! Transforms typed HIR into executable bytecode.

use std::collections::HashMap;

use x3_ast::BinaryOp;
use x3_common::{Literal, Span};
use x3_hir::hir::{
    AssignTarget, ContextField, HirExpr, HirExprKind, HirFunction, HirGlobal, HirModule, HirParam,
    HirStmt, SymbolId, VmIntrinsic,
};
use x3_typeck::Type;

use crate::bc_format::{BytecodeModule, GlobalEntry, ModuleFlags};
use crate::emit::{BytecodeEmitter, Label};
use crate::error::{BackendError, BackendErrorKind, BackendErrors, BackendResult};
use crate::layout::{is_float_type, type_to_tag, FunctionLayout, LayoutComputer};
use crate::opcode::{AtomicId, ConstIdx, FuncIdx, Register};

/// Compiles HIR modules to bytecode.
pub struct BytecodeCompiler {
    /// Bytecode emitter.
    emitter: BytecodeEmitter,
    /// Layout tracking.
    layout: LayoutComputer,
    /// Accumulated errors.
    errors: BackendErrors,
    /// Function symbol → index mapping.
    function_indices: HashMap<SymbolId, FuncIdx>,
    /// Event name → ID mapping.
    event_ids: HashMap<String, u32>,
    /// Next event ID.
    next_event_id: u32,
    /// Debug mode.
    debug: bool,
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        Self {
            emitter: BytecodeEmitter::new(),
            layout: LayoutComputer::new(),
            errors: BackendErrors::new(),
            function_indices: HashMap::new(),
            event_ids: HashMap::new(),
            next_event_id: 0,
            debug: false,
        }
    }

    /// Enable debug info generation.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Compile a HIR module to bytecode.
    pub fn compile(hir: &HirModule) -> BackendResult<BytecodeModule> {
        let mut compiler = Self::new();
        compiler.compile_module(hir)
    }

    /// Compile with options.
    pub fn compile_with_options(hir: &HirModule, debug: bool) -> BackendResult<BytecodeModule> {
        let mut compiler = Self::new().with_debug(debug);
        compiler.compile_module(hir)
    }

    fn compile_module(&mut self, hir: &HirModule) -> BackendResult<BytecodeModule> {
        // Phase 1: Collect all function symbols and assign indices
        for (idx, func) in hir.functions.iter().enumerate() {
            self.function_indices
                .insert(func.symbol, FuncIdx(idx as u32));
        }

        // Phase 2: Register globals
        let globals = self.compile_globals(&hir.globals)?;

        // Phase 3: Compile all functions
        for func in &hir.functions {
            self.compile_function(hir, func)?;
        }

        // Phase 4: Finalize (patch forward references)
        self.emitter.finalize()?;

        // Phase 5: Build module
        let mut module = BytecodeModule::new();
        module.const_pool = std::mem::take(&mut self.emitter.const_pool);
        module.functions = self.layout.get_function_entries();
        module.globals = globals;
        module.code = std::mem::take(&mut self.emitter).take_code();

        // Set flags
        if self.debug {
            module.flags.set(ModuleFlags::DEBUG_INFO);
        }

        Ok(module)
    }

    fn compile_globals(&mut self, globals: &[HirGlobal]) -> BackendResult<Vec<GlobalEntry>> {
        let mut entries = Vec::new();

        for global in globals {
            let slot = self.layout.register_global(global.symbol);

            // Evaluate initializer to constant
            let init_const = self.eval_const_expr(&global.initializer)?;

            entries.push(GlobalEntry {
                name: format!("global_{}", slot),
                type_tag: type_to_tag(&global.ty),
                mutable: true, // TODO: track mutability
                init_const,
            });
        }

        Ok(entries)
    }

    fn compile_function(&mut self, hir: &HirModule, func: &HirFunction) -> BackendResult<()> {
        let entry_point = self.emitter.current_offset();

        // Get function name
        let name = hir
            .symbol_name(func.symbol)
            .unwrap_or("anonymous")
            .to_string();

        // Collect parameter info
        let params: Vec<(SymbolId, String)> = func
            .params
            .iter()
            .map(|p| (p.symbol, p.name.clone()))
            .collect();

        // Begin function
        self.layout
            .begin_function(func.symbol, name, entry_point, &params)?;

        // Compile function body
        for stmt in &func.body {
            self.compile_stmt(hir, stmt)?;
        }

        // Ensure function ends with return
        // (If body doesn't end with return, add implicit void return)
        if !self.ends_with_return(&func.body) {
            self.emitter.emit_ret_void();
        }

        // End function
        let return_tag = type_to_tag(&func.return_ty);
        self.layout.end_function(return_tag);

        Ok(())
    }

    fn compile_stmt(&mut self, hir: &HirModule, stmt: &HirStmt) -> BackendResult<()> {
        match stmt {
            HirStmt::Let {
                symbol,
                ty,
                value,
                span,
                ..
            } => {
                self.emitter.set_span(*span);

                // Allocate register for local
                let layout = self.layout.current_function_mut().unwrap();
                let dst = layout.alloc_local(*symbol);

                // Compile initializer
                self.compile_expr_into(hir, value, dst)?;
                Ok(())
            }

            HirStmt::Assign {
                target,
                value,
                span,
            } => {
                self.emitter.set_span(*span);
                self.compile_assign(hir, target, value)?;
                Ok(())
            }

            HirStmt::Expr(expr) => {
                // Compile expression for side effects, discard result
                let dst = self.alloc_temp();
                self.compile_expr_into(hir, expr, dst)?;
                Ok(())
            }

            HirStmt::Return { value, span } => {
                self.emitter.set_span(*span);
                if let Some(expr) = value {
                    let result = self.compile_expr(hir, expr)?;
                    self.emitter.emit_ret(result);
                } else {
                    self.emitter.emit_ret_void();
                }
                Ok(())
            }

            HirStmt::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                self.emitter.set_span(*span);
                self.compile_if(hir, condition, then_block, else_block)?;
                Ok(())
            }

            HirStmt::While {
                label,
                condition,
                body,
                span,
            } => {
                self.emitter.set_span(*span);
                self.compile_while(hir, condition, body)?;
                Ok(())
            }

            HirStmt::Break { label, span } => {
                self.emitter.set_span(*span);
                let layout = self.layout.current_function().unwrap();
                if let Some(loop_ctx) = layout.current_loop() {
                    let break_label = loop_ctx.break_label;
                    self.emitter.emit_jump(break_label, *span);
                } else {
                    return Err(BackendError::new(BackendErrorKind::BreakOutsideLoop, *span));
                }
                Ok(())
            }

            HirStmt::Continue { label, span } => {
                self.emitter.set_span(*span);
                let layout = self.layout.current_function().unwrap();
                if let Some(loop_ctx) = layout.current_loop() {
                    let continue_label = loop_ctx.continue_label;
                    self.emitter.emit_jump(continue_label, *span);
                } else {
                    return Err(BackendError::new(
                        BackendErrorKind::ContinueOutsideLoop,
                        *span,
                    ));
                }
                Ok(())
            }

            HirStmt::AtomicBegin { block_id, span } => {
                self.emitter.set_span(*span);
                self.emitter.emit_atomic_begin(AtomicId(block_id.0 as u16));
                Ok(())
            }

            HirStmt::AtomicEnd {
                block_id,
                commit,
                span,
            } => {
                self.emitter.set_span(*span);
                if *commit {
                    self.emitter.emit_atomic_commit(AtomicId(block_id.0 as u16));
                } else {
                    self.emitter
                        .emit_atomic_rollback(AtomicId(block_id.0 as u16));
                }
                Ok(())
            }

            HirStmt::Emit {
                event_name,
                args,
                span,
            } => {
                self.emitter.set_span(*span);

                // Get or create event ID
                let event_id = *self.event_ids.entry(event_name.clone()).or_insert_with(|| {
                    let id = self.next_event_id;
                    self.next_event_id += 1;
                    id
                });

                // Compile arguments
                let arg_regs: Vec<Register> = args
                    .iter()
                    .map(|arg| self.compile_expr(hir, arg))
                    .collect::<BackendResult<_>>()?;

                self.emitter.emit_emit(event_id, &arg_regs);
                Ok(())
            }

            HirStmt::AgentInit {
                agent_symbol,
                field_values,
                span,
            } => {
                self.emitter.set_span(*span);
                // TODO: implement agent initialization
                Err(BackendError::new(
                    BackendErrorKind::NotImplemented("agent init".to_string()),
                    *span,
                ))
            }
        }
    }

    fn compile_if(
        &mut self,
        hir: &HirModule,
        condition: &HirExpr,
        then_block: &[HirStmt],
        else_block: &[HirStmt],
    ) -> BackendResult<()> {
        let span = condition.span;

        // Compile condition
        let cond_reg = self.compile_expr(hir, condition)?;

        let else_label = self.emitter.create_label();
        let end_label = self.emitter.create_label();

        // Jump to else if condition is false
        self.emitter.emit_jump_unless(cond_reg, else_label, span);

        // Then block
        for stmt in then_block {
            self.compile_stmt(hir, stmt)?;
        }

        // Jump over else block
        if !else_block.is_empty() {
            self.emitter.emit_jump(end_label, span);
        }

        // Else block
        self.emitter.define_label(else_label);
        for stmt in else_block {
            self.compile_stmt(hir, stmt)?;
        }

        self.emitter.define_label(end_label);
        Ok(())
    }

    fn compile_while(
        &mut self,
        hir: &HirModule,
        condition: &HirExpr,
        body: &[HirStmt],
    ) -> BackendResult<()> {
        let span = condition.span;

        let loop_start = self.emitter.create_label();
        let loop_end = self.emitter.create_label();

        // Push loop context
        {
            let layout = self.layout.current_function_mut().unwrap();
            layout.push_loop(loop_start, loop_end);
        }

        // Loop start
        self.emitter.define_label(loop_start);

        // Condition
        let cond_reg = self.compile_expr(hir, condition)?;
        self.emitter.emit_jump_unless(cond_reg, loop_end, span);

        // Body
        for stmt in body {
            self.compile_stmt(hir, stmt)?;
        }

        // Jump back to start
        self.emitter.emit_jump(loop_start, span);

        // Loop end
        self.emitter.define_label(loop_end);

        // Pop loop context
        {
            let layout = self.layout.current_function_mut().unwrap();
            layout.pop_loop();
        }

        Ok(())
    }

    fn compile_assign(
        &mut self,
        hir: &HirModule,
        target: &AssignTarget,
        value: &HirExpr,
    ) -> BackendResult<()> {
        match target {
            AssignTarget::Variable(symbol) => {
                // Get register for variable
                let layout = self.layout.current_function().unwrap();
                if let Some(reg) = layout.get_local(*symbol) {
                    self.compile_expr_into(hir, value, reg)?;
                } else if let Some(slot) = self.layout.get_global_slot(*symbol) {
                    let val_reg = self.compile_expr(hir, value)?;
                    self.emitter.emit_store_global(slot, val_reg);
                } else {
                    return Err(BackendError::new(
                        BackendErrorKind::UnknownSymbol(*symbol),
                        value.span,
                    ));
                }
            }

            AssignTarget::Field {
                object,
                field,
                field_ty,
            } => {
                let obj_reg = self.compile_expr(hir, object)?;
                let val_reg = self.compile_expr(hir, value)?;
                // TODO: resolve field index from type
                let field_idx = 0u16; // Placeholder
                self.emitter.emit_store_field(obj_reg, field_idx, val_reg);
            }

            AssignTarget::Index {
                array,
                index,
                element_ty,
            } => {
                let arr_reg = self.compile_expr(hir, array)?;
                let idx_reg = self.compile_expr(hir, index)?;
                let val_reg = self.compile_expr(hir, value)?;
                self.emitter.emit_store_index(arr_reg, idx_reg, val_reg);
            }
        }
        Ok(())
    }

    /// Compile expression and return the register containing the result.
    fn compile_expr(&mut self, hir: &HirModule, expr: &HirExpr) -> BackendResult<Register> {
        let dst = self.alloc_temp();
        self.compile_expr_into(hir, expr, dst)?;
        Ok(dst)
    }

    /// Compile expression into a specific register.
    fn compile_expr_into(
        &mut self,
        hir: &HirModule,
        expr: &HirExpr,
        dst: Register,
    ) -> BackendResult<()> {
        self.emitter.set_span(expr.span);

        match &expr.kind {
            HirExprKind::Literal(lit) => {
                self.compile_literal(lit, &expr.ty, dst)?;
            }

            HirExprKind::Var(symbol) => {
                // Look up in locals first, then globals
                let layout = self.layout.current_function().unwrap();
                if let Some(src) = layout.get_local(*symbol) {
                    if src != dst {
                        self.emitter.emit_mov(dst, src);
                    }
                } else if let Some(slot) = self.layout.get_global_slot(*symbol) {
                    self.emitter.emit_load_global(dst, slot);
                } else {
                    return Err(BackendError::new(
                        BackendErrorKind::UnknownSymbol(*symbol),
                        expr.span,
                    ));
                }
            }

            HirExprKind::Binary { op, left, right } => {
                let left_reg = self.compile_expr(hir, left)?;
                let right_reg = self.compile_expr(hir, right)?;
                self.compile_binary_op(*op, dst, left_reg, right_reg, &left.ty)?;
            }

            HirExprKind::Unary { op, operand } => {
                let src = self.compile_expr(hir, operand)?;
                self.compile_unary_op(*op, dst, src, &operand.ty)?;
            }

            HirExprKind::Call { callee, args } => {
                // Get function index - copy to avoid borrow conflict
                let func_idx = *self.function_indices.get(callee).ok_or_else(|| {
                    BackendError::new(BackendErrorKind::UnknownSymbol(*callee), expr.span)
                })?;

                // Compile arguments
                let arg_regs: Vec<Register> = args
                    .iter()
                    .map(|arg| self.compile_expr(hir, arg))
                    .collect::<BackendResult<_>>()?;

                self.emitter.emit_call(dst, func_idx, &arg_regs);
            }

            HirExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                // TODO: resolve method to function
                return Err(BackendError::new(
                    BackendErrorKind::NotImplemented("method calls".to_string()),
                    expr.span,
                ));
            }

            HirExprKind::Field { object, field } => {
                let obj_reg = self.compile_expr(hir, object)?;
                // TODO: resolve field index
                let field_idx = 0u16;
                self.emitter.emit_load_field(dst, obj_reg, field_idx);
            }

            HirExprKind::Index { array, index } => {
                let arr_reg = self.compile_expr(hir, array)?;
                let idx_reg = self.compile_expr(hir, index)?;
                self.emitter.emit_load_index(dst, arr_reg, idx_reg);
            }

            HirExprKind::Array(elements) => {
                self.emitter.emit_new_array(dst, elements.len() as u16);
                for elem in elements {
                    let elem_reg = self.compile_expr(hir, elem)?;
                    self.emitter.emit_array_push(dst, elem_reg);
                }
            }

            HirExprKind::Tuple(elements) => {
                let elem_regs: Vec<Register> = elements
                    .iter()
                    .map(|e| self.compile_expr(hir, e))
                    .collect::<BackendResult<_>>()?;
                self.emitter.emit_new_tuple(dst, &elem_regs);
            }

            HirExprKind::Block { stmts, expr } => {
                for stmt in stmts {
                    self.compile_stmt(hir, stmt)?;
                }
                if let Some(final_expr) = expr {
                    self.compile_expr_into(hir, final_expr, dst)?;
                }
            }

            HirExprKind::IfExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_reg = self.compile_expr(hir, condition)?;

                let else_label = self.emitter.create_label();
                let end_label = self.emitter.create_label();

                self.emitter
                    .emit_jump_unless(cond_reg, else_label, expr.span);

                // Then branch
                self.compile_expr_into(hir, then_expr, dst)?;
                self.emitter.emit_jump(end_label, expr.span);

                // Else branch
                self.emitter.define_label(else_label);
                self.compile_expr_into(hir, else_expr, dst)?;

                self.emitter.define_label(end_label);
            }

            HirExprKind::Cast {
                expr: inner,
                target_ty,
            } => {
                let src = self.compile_expr(hir, inner)?;
                self.compile_cast(dst, src, &inner.ty, target_ty)?;
            }

            HirExprKind::ContextAccess(field) => match field {
                ContextField::Sender => self.emitter.emit_ctx_sender(dst),
                ContextField::BlockHeight => self.emitter.emit_ctx_block_height(dst),
                ContextField::Timestamp => self.emitter.emit_ctx_timestamp(dst),
                ContextField::Value => self.emitter.emit_ctx_value(dst),
                ContextField::GasRemaining => self.emitter.emit_ctx_gas(dst),
                ContextField::ChainId => self.emitter.emit_ctx_chain_id(dst),
            },

            HirExprKind::VmIntrinsic {
                vm,
                intrinsic,
                args,
            } => {
                return Err(BackendError::new(
                    BackendErrorKind::NotImplemented(format!("VM intrinsic: {:?}", intrinsic)),
                    expr.span,
                ));
            }

            HirExprKind::SelfRef => {
                self.emitter.emit_agent_self(dst);
            }
        }

        Ok(())
    }

    fn compile_literal(&mut self, lit: &Literal, ty: &Type, dst: Register) -> BackendResult<()> {
        match lit {
            Literal::Integer(v) => {
                self.emitter.emit_int(dst, *v)?;
            }
            Literal::Float(v) => {
                self.emitter.emit_float(dst, *v)?;
            }
            Literal::String(s) => {
                self.emitter.emit_string(dst, s)?;
            }
            Literal::Bool(b) => {
                if *b {
                    self.emitter.emit_load_true(dst);
                } else {
                    self.emitter.emit_load_false(dst);
                }
            }
            Literal::Unit => {
                // Unit is represented as integer 0 (like void/null)
                self.emitter.emit_int(dst, 0)?;
            }
        }
        Ok(())
    }

    fn compile_binary_op(
        &mut self,
        op: BinaryOp,
        dst: Register,
        left: Register,
        right: Register,
        left_ty: &Type,
    ) -> BackendResult<()> {
        let is_float = is_float_type(left_ty);

        match op {
            BinaryOp::Add => {
                if is_float {
                    self.emitter.emit_add_f(dst, left, right);
                } else {
                    self.emitter.emit_add_i(dst, left, right);
                }
            }
            BinaryOp::Sub => {
                if is_float {
                    self.emitter.emit_sub_f(dst, left, right);
                } else {
                    self.emitter.emit_sub_i(dst, left, right);
                }
            }
            BinaryOp::Mul => {
                if is_float {
                    self.emitter.emit_mul_f(dst, left, right);
                } else {
                    self.emitter.emit_mul_i(dst, left, right);
                }
            }
            BinaryOp::Div => {
                if is_float {
                    self.emitter.emit_div_f(dst, left, right);
                } else {
                    self.emitter.emit_div_i(dst, left, right);
                }
            }
            BinaryOp::Mod => {
                self.emitter.emit_mod_i(dst, left, right);
            }
            BinaryOp::Equal => {
                if is_float {
                    self.emitter.emit_eq_f(dst, left, right);
                } else {
                    self.emitter.emit_eq_i(dst, left, right);
                }
            }
            BinaryOp::NotEqual => {
                if is_float {
                    // ne_f doesn't exist, use eq_f + lnot
                    self.emitter.emit_eq_f(dst, left, right);
                    self.emitter.emit_lnot(dst, dst);
                } else {
                    self.emitter.emit_ne_i(dst, left, right);
                }
            }
            BinaryOp::Less => {
                if is_float {
                    self.emitter.emit_lt_f(dst, left, right);
                } else {
                    self.emitter.emit_lt_i(dst, left, right);
                }
            }
            BinaryOp::LessEqual => {
                if is_float {
                    self.emitter.emit_le_f(dst, left, right);
                } else {
                    self.emitter.emit_le_i(dst, left, right);
                }
            }
            BinaryOp::Greater => {
                if is_float {
                    self.emitter.emit_gt_f(dst, left, right);
                } else {
                    self.emitter.emit_gt_i(dst, left, right);
                }
            }
            BinaryOp::GreaterEqual => {
                if is_float {
                    self.emitter.emit_ge_f(dst, left, right);
                } else {
                    self.emitter.emit_ge_i(dst, left, right);
                }
            }
            BinaryOp::LogicalAnd => {
                self.emitter.emit_land(dst, left, right);
            }
            BinaryOp::LogicalOr => {
                self.emitter.emit_lor(dst, left, right);
            }
            BinaryOp::Pow => {
                // Power is not a native opcode - use integer multiply loop or call
                // For now, not implemented
                return Err(BackendError::without_span(
                    BackendErrorKind::NotImplemented("power operator".to_string()),
                ));
            }
        }
        Ok(())
    }

    fn compile_unary_op(
        &mut self,
        op: x3_ast::UnaryOp,
        dst: Register,
        src: Register,
        src_ty: &Type,
    ) -> BackendResult<()> {
        match op {
            x3_ast::UnaryOp::Negate => {
                if is_float_type(src_ty) {
                    self.emitter.emit_neg_f(dst, src);
                } else {
                    self.emitter.emit_neg_i(dst, src);
                }
            }
            x3_ast::UnaryOp::Not => {
                self.emitter.emit_lnot(dst, src);
            }
        }
        Ok(())
    }

    fn compile_cast(
        &mut self,
        dst: Register,
        src: Register,
        from_ty: &Type,
        to_ty: &Type,
    ) -> BackendResult<()> {
        use x3_typeck::{PrimitiveType, TypeKind};

        // Helper to check if types are the same primitive
        let same_type = from_ty.kind == to_ty.kind;
        if same_type {
            if dst != src {
                self.emitter.emit_mov(dst, src);
            }
            return Ok(());
        }

        // Get primitive types if both are primitives
        let from_prim = match &from_ty.kind {
            TypeKind::Primitive(p) => Some(p),
            _ => None,
        };
        let to_prim = match &to_ty.kind {
            TypeKind::Primitive(p) => Some(p),
            _ => None,
        };

        // Handle integer width conversions
        match (from_prim, to_prim) {
            (Some(PrimitiveType::I32), Some(PrimitiveType::I64)) => {
                self.emitter.emit_i32_to_i64(dst, src);
            }
            (Some(PrimitiveType::I64), Some(PrimitiveType::I32)) => {
                self.emitter.emit_i64_to_i32(dst, src);
            }
            // For other conversions, just move (truncation/extension handled at runtime)
            _ => {
                if dst != src {
                    self.emitter.emit_mov(dst, src);
                }
            }
        }

        Ok(())
    }

    /// Evaluate a constant expression to a constant pool index.
    fn eval_const_expr(&mut self, expr: &HirExpr) -> BackendResult<ConstIdx> {
        match &expr.kind {
            HirExprKind::Literal(lit) => match lit {
                Literal::Integer(v) => self.emitter.const_pool.add_integer(*v),
                Literal::Float(v) => self.emitter.const_pool.add_float(*v),
                Literal::String(s) => self.emitter.const_pool.add_string(s.clone()),
                Literal::Bool(b) => self.emitter.const_pool.add_bool(*b),
                Literal::Unit => self.emitter.const_pool.add_integer(0),
            },
            _ => {
                // Non-constant expression - emit a zero placeholder
                self.emitter.const_pool.add_integer(0)
            }
        }
    }

    /// Allocate a temporary register.
    fn alloc_temp(&mut self) -> Register {
        let layout = self.layout.current_function_mut().unwrap();
        layout.alloc_register()
    }

    /// Check if a statement list ends with a return.
    fn ends_with_return(&self, stmts: &[HirStmt]) -> bool {
        stmts
            .last()
            .map_or(false, |stmt| matches!(stmt, HirStmt::Return { .. }))
    }
}

impl Default for BytecodeCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_hir::hir::{FunctionAttrs, HirFunction, HirModule, HirParam, Symbol, SymbolKind};
    use x3_typeck::{FunctionSignature, Type, TypeKind};

    fn make_test_module() -> HirModule {
        let span = Span { start: 0, end: 100 };

        let mut module = HirModule::new(span);

        // Add a symbol for the function
        module.symbols.push(Symbol {
            id: SymbolId(0),
            name: "add".to_string(),
            kind: SymbolKind::Function,
            ty: Type::function(vec![Type::i64(), Type::i64()], Type::i64()),
            span,
        });

        // Add parameter symbols
        module.symbols.push(Symbol {
            id: SymbolId(1),
            name: "a".to_string(),
            kind: SymbolKind::Param { mutable: false },
            ty: Type::i64(),
            span,
        });
        module.symbols.push(Symbol {
            id: SymbolId(2),
            name: "b".to_string(),
            kind: SymbolKind::Param { mutable: false },
            ty: Type::i64(),
            span,
        });

        // Simple function: fn add(a: i64, b: i64) -> i64 { return a + b; }
        let func = HirFunction {
            symbol: SymbolId(0),
            params: vec![
                HirParam {
                    symbol: SymbolId(1),
                    name: "a".to_string(),
                    ty: Type::i64(),
                    mutable: false,
                    span,
                },
                HirParam {
                    symbol: SymbolId(2),
                    name: "b".to_string(),
                    ty: Type::i64(),
                    mutable: false,
                    span,
                },
            ],
            body: vec![HirStmt::Return {
                value: Some(HirExpr::binary(
                    BinaryOp::Add,
                    HirExpr::var(SymbolId(1), Type::i64(), span),
                    HirExpr::var(SymbolId(2), Type::i64(), span),
                    Type::i64(),
                    span,
                )),
                span,
            }],
            return_ty: Type::i64(),
            attrs: FunctionAttrs::default(),
            span,
        };

        module.functions.push(func);
        module
    }

    #[test]
    fn compile_simple_function() {
        let module = make_test_module();
        let bytecode = BytecodeCompiler::compile(&module).unwrap();

        assert_eq!(bytecode.functions.len(), 1);
        assert_eq!(bytecode.functions[0].name, "add");
        assert_eq!(bytecode.functions[0].param_count, 2);
        assert!(!bytecode.code.is_empty());
    }

    #[test]
    fn bytecode_roundtrip() {
        let module = make_test_module();
        let bytecode = BytecodeCompiler::compile(&module).unwrap();

        // Serialize and deserialize
        let bytes = bytecode.to_bytes();
        let decoded = BytecodeModule::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.functions.len(), bytecode.functions.len());
        assert_eq!(decoded.functions[0].name, bytecode.functions[0].name);
        assert_eq!(decoded.code.len(), bytecode.code.len());
    }

    #[test]
    fn compile_with_locals() {
        let span = Span { start: 0, end: 100 };
        let mut module = HirModule::new(span);

        // Symbols
        module.symbols.push(Symbol {
            id: SymbolId(0),
            name: "test".to_string(),
            kind: SymbolKind::Function,
            ty: Type::function(vec![], Type::i64()),
            span,
        });
        module.symbols.push(Symbol {
            id: SymbolId(1),
            name: "x".to_string(),
            kind: SymbolKind::Local { mutable: false },
            ty: Type::i64(),
            span,
        });

        // fn test() -> i64 { let x = 42; return x; }
        let func = HirFunction {
            symbol: SymbolId(0),
            params: vec![],
            body: vec![
                HirStmt::Let {
                    symbol: SymbolId(1),
                    ty: Type::i64(),
                    value: HirExpr::literal(Literal::Integer(42), Type::i64(), span),
                    mutable: false,
                    span,
                },
                HirStmt::Return {
                    value: Some(HirExpr::var(SymbolId(1), Type::i64(), span)),
                    span,
                },
            ],
            return_ty: Type::i64(),
            attrs: FunctionAttrs::default(),
            span,
        };

        module.functions.push(func);

        let bytecode = BytecodeCompiler::compile(&module).unwrap();
        assert!(!bytecode.code.is_empty());
    }
}
