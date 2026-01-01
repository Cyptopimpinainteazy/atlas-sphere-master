//! MIR → Bytecode Lowering
//!
//! Transforms optimized MIR into executable bytecode.
//! This is the final stage after MIR optimization passes have run.

use std::collections::HashMap;

use x3_ast::{BinaryOp, UnaryOp};
use x3_common::{Literal, Span};
use x3_mir::{
    memory::MemoryModel, MirBlock, MirBlockId, MirFunction, MirModule, MirRhs, MirStatement,
    MirTerminator, MirValue, SymbolId,
};

use crate::bc_format::{BytecodeModule, ModuleFlags};
use crate::emit::BytecodeEmitter;
use crate::error::{BackendError, BackendErrorKind, BackendResult};
use crate::layout::LayoutComputer;
use crate::opcode::{FuncIdx, Register};

/// Compiles MIR modules to bytecode.
///
/// This compiler takes optimized MIR and emits bytecode directly,
/// allowing optimization passes to actually affect the output.
pub struct MirBytecodeCompiler {
    /// Bytecode emitter.
    emitter: BytecodeEmitter,
    /// Layout tracking.
    layout: LayoutComputer,
    /// Function symbol → index mapping.
    function_indices: HashMap<SymbolId, FuncIdx>,
    /// MIR value → register mapping (per function).
    value_regs: HashMap<MirValue, Register>,
    /// Block → label mapping (per function).
    block_labels: HashMap<MirBlockId, crate::emit::Label>,
    /// Next available register.
    next_reg: u16,
    /// Debug mode.
    debug: bool,
    /// Current span for error reporting.
    current_span: Span,
}

impl MirBytecodeCompiler {
    pub fn new() -> Self {
        Self {
            emitter: BytecodeEmitter::new(),
            layout: LayoutComputer::new(),
            function_indices: HashMap::new(),
            value_regs: HashMap::new(),
            block_labels: HashMap::new(),
            next_reg: 0,
            debug: false,
            current_span: Span::dummy(),
        }
    }

    /// Enable debug info generation.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Compile a MIR module to bytecode.
    pub fn compile(mir: &MirModule) -> BackendResult<BytecodeModule> {
        let mut compiler = Self::new();
        compiler.compile_module(mir)
    }

    /// Compile with debug info.
    pub fn compile_with_options(mir: &MirModule, debug: bool) -> BackendResult<BytecodeModule> {
        let mut compiler = Self::new().with_debug(debug);
        compiler.compile_module(mir)
    }

    fn compile_module(&mut self, mir: &MirModule) -> BackendResult<BytecodeModule> {
        // Phase 1: Collect all function symbols and assign indices
        for (idx, func) in mir.functions.iter().enumerate() {
            self.function_indices
                .insert(func.symbol, FuncIdx(idx as u32));
        }

        // Phase 2: Compile all functions
        for func in &mir.functions {
            self.compile_function(func)?;
        }

        // Phase 3: Finalize (patch forward references)
        self.emitter.finalize()?;

        // Phase 4: Build module
        let mut module = BytecodeModule::new();
        module.const_pool = std::mem::take(&mut self.emitter.const_pool);
        module.functions = self.layout.get_function_entries();
        module.globals = Vec::new(); // TODO: handle globals from MIR
        module.code = std::mem::take(&mut self.emitter).take_code();

        // Set flags
        if self.debug {
            module.flags.set(ModuleFlags::DEBUG_INFO);
        }

        Ok(module)
    }

    fn compile_function(&mut self, func: &MirFunction) -> BackendResult<()> {
        // Reset per-function state
        self.value_regs.clear();
        self.block_labels.clear();
        self.next_reg = 0;
        self.current_span = func.span;

        let entry_point = self.emitter.current_offset();

        // Register function parameters
        let params: Vec<(SymbolId, String)> = func
            .params
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let reg = self.allocate_reg();
                self.value_regs.insert(*v, reg);
                (SymbolId(i), format!("param_{}", i))
            })
            .collect();

        // Begin function in layout
        let name = format!("fn_{}", func.symbol.0);
        self.layout
            .begin_function(func.symbol, name, entry_point, &params)?;

        // Create labels for all blocks upfront (for forward jumps)
        for block in &func.blocks {
            let label = self.emitter.create_label();
            self.block_labels.insert(block.id, label);
        }

        // Compile all blocks
        for block in &func.blocks {
            self.compile_block(block)?;
        }

        // End function - use TYPE_TAG_INT for simplicity (void=0, int=1)
        let return_type_tag = self.infer_return_type(func);
        self.layout.end_function(return_type_tag);

        Ok(())
    }

    /// Infer return type tag from function terminator.
    fn infer_return_type(&self, func: &MirFunction) -> u8 {
        for block in &func.blocks {
            if let Some(MirTerminator::Return(Some(_))) = &block.terminator {
                return 1; // TYPE_TAG_INT
            }
        }
        0 // TYPE_TAG_VOID
    }

    fn compile_block(&mut self, block: &MirBlock) -> BackendResult<()> {
        // Define label at block start
        let label = self.block_labels[&block.id];
        self.emitter.define_label(label);

        // Compile all statements
        for stmt in &block.statements {
            self.compile_statement(stmt)?;
        }

        // Compile terminator
        if let Some(ref term) = block.terminator {
            self.compile_terminator(term)?;
        }

        Ok(())
    }

    fn compile_statement(&mut self, stmt: &MirStatement) -> BackendResult<()> {
        let dst = self.get_or_alloc_reg(stmt.target);

        match &stmt.rhs {
            MirRhs::Literal(lit) => {
                self.compile_literal(lit, dst)?;
            }
            MirRhs::Unary(op, val) => {
                let src = self.get_reg(*val)?;
                self.compile_unary(*op, dst, src)?;
            }
            MirRhs::Binary(op, left, right) => {
                let left_reg = self.get_reg(*left)?;
                let right_reg = self.get_reg(*right)?;
                self.compile_binary(*op, dst, left_reg, right_reg)?;
            }
            MirRhs::Call { target, args } => {
                let arg_regs: Vec<Register> = args
                    .iter()
                    .map(|a| self.get_reg(*a))
                    .collect::<BackendResult<Vec<_>>>()?;

                let func_idx = self.function_indices.get(target).copied().ok_or_else(|| {
                    BackendError::new(
                        BackendErrorKind::UnknownFunction {
                            name: format!("{:?}", target),
                        },
                        self.current_span,
                    )
                })?;

                self.emitter.emit_call(dst, func_idx, &arg_regs);
            }
            MirRhs::Load { model, addr } => {
                let addr_reg = self.get_reg(*addr)?;
                match model {
                    MemoryModel::Register => {
                        // Register-to-register move (pure operation)
                        self.emitter.emit_load_register(dst, addr_reg);
                    }
                    MemoryModel::Stack => {
                        // Load from function-local stack slot
                        self.emitter.emit_load_stack(dst, addr_reg);
                    }
                    MemoryModel::Heap => {
                        // Load from heap memory (may alias, bounds-checked)
                        self.emitter.emit_load_heap(dst, addr_reg);
                    }
                    MemoryModel::GlobalStorage => {
                        // Load from on-chain persistent storage
                        // For now, interpret addr_reg as a constant pool index
                        // (In production, would encode the address differently)
                        if let Some(idx) = self.extract_constant_index(addr_reg) {
                            self.emitter.emit_load_global_storage(dst, idx);
                        } else {
                            // Fallback: treat as heap for addresses not in const pool
                            self.emitter.emit_load_heap(dst, addr_reg);
                        }
                    }
                }
            }
            MirRhs::Store { model, addr, val } => {
                let addr_reg = self.get_reg(*addr)?;
                let val_reg = self.get_reg(*val)?;
                match model {
                    MemoryModel::Register => {
                        // Register-to-register move (pure operation)
                        self.emitter.emit_store_register(addr_reg, val_reg);
                    }
                    MemoryModel::Stack => {
                        // Store to function-local stack slot
                        self.emitter.emit_store_stack(addr_reg, val_reg);
                    }
                    MemoryModel::Heap => {
                        // Store to heap memory (may alias, bounds-checked)
                        self.emitter.emit_store_heap(addr_reg, val_reg);
                    }
                    MemoryModel::GlobalStorage => {
                        // Store to on-chain persistent storage (side-effecting)
                        if let Some(idx) = self.extract_constant_index(addr_reg) {
                            self.emitter.emit_store_global_storage(idx, val_reg);
                        } else {
                            // Fallback: treat as heap for addresses not in const pool
                            self.emitter.emit_store_heap(addr_reg, val_reg);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn compile_terminator(&mut self, term: &MirTerminator) -> BackendResult<()> {
        match term {
            MirTerminator::Return(None) => {
                self.emitter.emit_ret_void();
            }
            MirTerminator::Return(Some(val)) => {
                let reg = self.get_reg(*val)?;
                self.emitter.emit_ret(reg);
            }
            MirTerminator::Goto(target) => {
                let label = self.block_labels[target];
                self.emitter.emit_jump(label, self.current_span);
            }
            MirTerminator::Branch {
                cond,
                then_block,
                else_block,
            } => {
                let cond_reg = self.get_reg(*cond)?;
                let then_label = self.block_labels[then_block];
                let else_label = self.block_labels[else_block];

                // jump_if cond then_block
                // jump else_block
                self.emitter
                    .emit_jump_if(cond_reg, then_label, self.current_span);
                self.emitter.emit_jump(else_label, self.current_span);
            }
        }
        Ok(())
    }

    fn compile_literal(&mut self, lit: &Literal, dst: Register) -> BackendResult<()> {
        match lit {
            Literal::Integer(v) => {
                self.emitter.emit_int(dst, *v)?;
            }
            Literal::Float(v) => {
                self.emitter.emit_float(dst, *v)?;
            }
            Literal::Bool(true) => {
                self.emitter.emit_load_true(dst);
            }
            Literal::Bool(false) => {
                self.emitter.emit_load_false(dst);
            }
            Literal::String(s) => {
                self.emitter.emit_string(dst, s)?;
            }
            Literal::Unit => {
                self.emitter.emit_int(dst, 0)?;
            }
        }
        Ok(())
    }

    fn compile_unary(&mut self, op: UnaryOp, dst: Register, src: Register) -> BackendResult<()> {
        match op {
            UnaryOp::Negate => {
                // Assume integer for now - could track types
                self.emitter.emit_neg_i(dst, src);
            }
            UnaryOp::Not => {
                self.emitter.emit_lnot(dst, src);
            }
        }
        Ok(())
    }

    fn compile_binary(
        &mut self,
        op: BinaryOp,
        dst: Register,
        left: Register,
        right: Register,
    ) -> BackendResult<()> {
        // For now assume integer operations - a real compiler would track types
        match op {
            BinaryOp::Add => self.emitter.emit_add_i(dst, left, right),
            BinaryOp::Sub => self.emitter.emit_sub_i(dst, left, right),
            BinaryOp::Mul => self.emitter.emit_mul_i(dst, left, right),
            BinaryOp::Div => self.emitter.emit_div_i(dst, left, right),
            BinaryOp::Mod => self.emitter.emit_mod_i(dst, left, right),
            BinaryOp::Equal => self.emitter.emit_eq_i(dst, left, right),
            BinaryOp::NotEqual => self.emitter.emit_ne_i(dst, left, right),
            BinaryOp::Less => self.emitter.emit_lt_i(dst, left, right),
            BinaryOp::LessEqual => self.emitter.emit_le_i(dst, left, right),
            BinaryOp::Greater => self.emitter.emit_gt_i(dst, left, right),
            BinaryOp::GreaterEqual => self.emitter.emit_ge_i(dst, left, right),
            BinaryOp::LogicalAnd => self.emitter.emit_land(dst, left, right),
            BinaryOp::LogicalOr => self.emitter.emit_lor(dst, left, right),
            BinaryOp::Pow => {
                // Pow is not a single opcode - would need a runtime call
                // For now, emit a placeholder (mul for x^2 patterns could be optimized)
                self.emitter.emit_mul_i(dst, left, right); // Placeholder
            }
        }
        Ok(())
    }

    /// Allocate a new register.
    fn allocate_reg(&mut self) -> Register {
        let reg = Register(self.next_reg);
        self.next_reg += 1;
        reg
    }

    /// Get or allocate register for a MIR value.
    fn get_or_alloc_reg(&mut self, val: MirValue) -> Register {
        if let Some(&reg) = self.value_regs.get(&val) {
            reg
        } else {
            let reg = self.allocate_reg();
            self.value_regs.insert(val, reg);
            reg
        }
    }

    /// Get register for an existing MIR value.
    fn get_reg(&self, val: MirValue) -> BackendResult<Register> {
        self.value_regs.get(&val).copied().ok_or_else(|| {
            BackendError::new(
                BackendErrorKind::Internal(format!(
                    "MIR value {:?} not found in register map",
                    val
                )),
                self.current_span,
            )
        })
    }

    /// Extract constant pool index from a register, if available.
    /// This is a heuristic for GlobalStorage accesses that are loaded from constants.
    /// For a fully general solution, the MIR would annotate the address explicitly.
    fn extract_constant_index(&self, _reg: Register) -> Option<u32> {
        // In a full implementation, we would:
        // 1. Track which registers hold constant indices
        // 2. Look up the constant value for GlobalStorage keys
        // For now, we return None to fall back to heap-like access.
        // TODO: Implement full constant tracking for GlobalStorage optimization.
        None
    }
}

impl Default for MirBytecodeCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::Span;

    fn make_simple_mir() -> MirModule {
        // fn main() -> i64 { return 42; }
        MirModule {
            functions: vec![MirFunction {
                symbol: SymbolId(0),
                params: vec![],
                entry: MirBlockId(0),
                blocks: vec![MirBlock {
                    id: MirBlockId(0),
                    statements: vec![MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Integer(42)),
                    }],
                    terminator: Some(MirTerminator::Return(Some(MirValue(0)))),
                }],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    fn make_arithmetic_mir() -> MirModule {
        // fn main() -> i64 { let a = 10; let b = 20; return a + b; }
        MirModule {
            functions: vec![MirFunction {
                symbol: SymbolId(0),
                params: vec![],
                entry: MirBlockId(0),
                blocks: vec![MirBlock {
                    id: MirBlockId(0),
                    statements: vec![
                        MirStatement {
                            target: MirValue(0),
                            rhs: MirRhs::Literal(Literal::Integer(10)),
                        },
                        MirStatement {
                            target: MirValue(1),
                            rhs: MirRhs::Literal(Literal::Integer(20)),
                        },
                        MirStatement {
                            target: MirValue(2),
                            rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(1)),
                        },
                    ],
                    terminator: Some(MirTerminator::Return(Some(MirValue(2)))),
                }],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    fn make_optimized_mir() -> MirModule {
        // After constant folding: fn main() -> i64 { return 30; }
        MirModule {
            functions: vec![MirFunction {
                symbol: SymbolId(0),
                params: vec![],
                entry: MirBlockId(0),
                blocks: vec![MirBlock {
                    id: MirBlockId(0),
                    statements: vec![MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Integer(30)), // 10 + 20 folded
                    }],
                    terminator: Some(MirTerminator::Return(Some(MirValue(0)))),
                }],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    #[test]
    fn compile_simple_mir() {
        let mir = make_simple_mir();
        let result = MirBytecodeCompiler::compile(&mir);
        assert!(result.is_ok(), "Failed: {:?}", result.err());

        let module = result.unwrap();
        assert!(!module.code.is_empty());
    }

    #[test]
    fn compile_arithmetic_mir() {
        let mir = make_arithmetic_mir();
        let result = MirBytecodeCompiler::compile(&mir);
        assert!(result.is_ok(), "Failed: {:?}", result.err());

        let module = result.unwrap();
        assert!(!module.code.is_empty());
    }

    #[test]
    fn optimized_smaller_than_unoptimized() {
        let unopt_mir = make_arithmetic_mir();
        let opt_mir = make_optimized_mir();

        let unopt_bc = MirBytecodeCompiler::compile(&unopt_mir).unwrap();
        let opt_bc = MirBytecodeCompiler::compile(&opt_mir).unwrap();

        // Optimized should have fewer instructions
        assert!(
            opt_bc.code.len() < unopt_bc.code.len(),
            "Optimized ({}) should be smaller than unoptimized ({})",
            opt_bc.code.len(),
            unopt_bc.code.len()
        );
    }
}
