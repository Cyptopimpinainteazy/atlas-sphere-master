//! Bytecode Emitter
//!
//! Builds raw bytecode from instructions, managing labels and forward references.

use std::collections::HashMap;

use crate::bc_format::{ConstPool, ConstValue, FunctionEntry};
use crate::error::{BackendError, BackendErrorKind, BackendResult};
use crate::opcode::{AtomicId, ConstIdx, FuncIdx, Instruction, JumpTarget, Opcode, Register};
use x3_common::Span;

/// Label for forward/backward jumps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

/// A forward reference that needs patching.
#[derive(Clone, Debug)]
struct ForwardRef {
    /// Byte offset where the target should be written.
    patch_offset: usize,
    /// Label being referenced.
    label: Label,
    /// Source span for error reporting.
    span: Span,
}

/// Bytecode emitter that builds instruction streams.
#[derive(Debug)]
pub struct BytecodeEmitter {
    /// Raw bytecode bytes.
    code: Vec<u8>,
    /// Constant pool.
    pub const_pool: ConstPool,
    /// Label definitions: label → byte offset.
    labels: HashMap<Label, u32>,
    /// Forward references to patch.
    forward_refs: Vec<ForwardRef>,
    /// Next label ID.
    next_label: u32,
    /// Current source span for debug info.
    current_span: Option<Span>,
    /// Source map entries (code_offset, line, column).
    source_map: Vec<(u32, u16, u16)>,
}

impl BytecodeEmitter {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            const_pool: ConstPool::new(),
            labels: HashMap::new(),
            forward_refs: Vec::new(),
            next_label: 0,
            current_span: None,
            source_map: Vec::new(),
        }
    }

    /// Create a new label for jumps.
    pub fn create_label(&mut self) -> Label {
        let label = Label(self.next_label);
        self.next_label += 1;
        label
    }

    /// Define a label at the current position.
    pub fn define_label(&mut self, label: Label) {
        let offset = self.code.len() as u32;
        self.labels.insert(label, offset);
    }

    /// Set current source span for debug info.
    pub fn set_span(&mut self, span: Span) {
        self.current_span = Some(span);
    }

    /// Current byte offset in code stream.
    pub fn current_offset(&self) -> u32 {
        self.code.len() as u32
    }

    /// Get the raw bytecode.
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Take ownership of the bytecode.
    pub fn take_code(self) -> Vec<u8> {
        self.code
    }

    /// Take ownership of the constant pool.
    pub fn take_const_pool(self) -> ConstPool {
        self.const_pool
    }

    /// Finalize bytecode: patch all forward references.
    pub fn finalize(&mut self) -> BackendResult<()> {
        for fwd in &self.forward_refs {
            let target = self.labels.get(&fwd.label).ok_or_else(|| {
                BackendError::new(
                    BackendErrorKind::UndefinedLabel(format!("L{}", fwd.label.0)),
                    fwd.span,
                )
            })?;

            // Patch the 4-byte target at patch_offset
            let target_bytes = target.to_le_bytes();
            self.code[fwd.patch_offset] = target_bytes[0];
            self.code[fwd.patch_offset + 1] = target_bytes[1];
            self.code[fwd.patch_offset + 2] = target_bytes[2];
            self.code[fwd.patch_offset + 3] = target_bytes[3];
        }
        self.forward_refs.clear();
        Ok(())
    }

    // ========================================================================
    // Low-level byte emission
    // ========================================================================

    fn emit_byte(&mut self, byte: u8) {
        self.code.push(byte);
    }

    fn emit_u16(&mut self, value: u16) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    fn emit_u32(&mut self, value: u32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    fn emit_i8(&mut self, value: i8) {
        self.code.push(value as u8);
    }

    fn emit_reg(&mut self, reg: Register) {
        self.emit_u16(reg.0);
    }

    fn emit_const(&mut self, idx: ConstIdx) {
        self.emit_u32(idx.0);
    }

    fn emit_func(&mut self, idx: FuncIdx) {
        self.emit_u32(idx.0);
    }

    fn emit_target(&mut self, target: JumpTarget) {
        self.emit_u32(target.0);
    }

    fn emit_label_ref(&mut self, label: Label, span: Span) {
        // If label is already defined, emit directly
        if let Some(&offset) = self.labels.get(&label) {
            self.emit_u32(offset);
        } else {
            // Forward reference: emit placeholder and record for patching
            let patch_offset = self.code.len();
            self.emit_u32(0); // Placeholder
            self.forward_refs.push(ForwardRef {
                patch_offset,
                label,
                span,
            });
        }
    }

    // ========================================================================
    // Instruction emission
    // ========================================================================

    /// Emit a NOP instruction.
    pub fn emit_nop(&mut self) {
        self.emit_byte(Opcode::Nop.to_byte());
    }

    /// Emit unconditional jump.
    pub fn emit_jump(&mut self, label: Label, span: Span) {
        self.emit_byte(Opcode::Jump.to_byte());
        self.emit_label_ref(label, span);
    }

    /// Emit conditional jump if true.
    pub fn emit_jump_if(&mut self, cond: Register, label: Label, span: Span) {
        self.emit_byte(Opcode::JumpIf.to_byte());
        self.emit_reg(cond);
        self.emit_label_ref(label, span);
    }

    /// Emit conditional jump if false.
    pub fn emit_jump_unless(&mut self, cond: Register, label: Label, span: Span) {
        self.emit_byte(Opcode::JumpUnless.to_byte());
        self.emit_reg(cond);
        self.emit_label_ref(label, span);
    }

    /// Emit function call.
    pub fn emit_call(&mut self, dst: Register, func: FuncIdx, args: &[Register]) {
        self.emit_byte(Opcode::Call.to_byte());
        self.emit_reg(dst);
        self.emit_func(func);
        self.emit_u16(args.len() as u16);
        for arg in args {
            self.emit_reg(*arg);
        }
    }

    /// Emit return with value.
    pub fn emit_ret(&mut self, src: Register) {
        self.emit_byte(Opcode::Ret.to_byte());
        self.emit_reg(src);
    }

    /// Emit void return.
    pub fn emit_ret_void(&mut self) {
        self.emit_byte(Opcode::RetVoid.to_byte());
    }

    /// Emit halt.
    pub fn emit_halt(&mut self) {
        self.emit_byte(Opcode::Halt.to_byte());
    }

    /// Emit load constant.
    pub fn emit_load_const(&mut self, dst: Register, idx: ConstIdx) {
        self.emit_byte(Opcode::LoadConst.to_byte());
        self.emit_reg(dst);
        self.emit_const(idx);
    }

    /// Emit move (copy register).
    pub fn emit_mov(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::Mov.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    /// Emit load global.
    pub fn emit_load_global(&mut self, dst: Register, global_idx: u32) {
        self.emit_byte(Opcode::LoadGlobal.to_byte());
        self.emit_reg(dst);
        self.emit_u32(global_idx);
    }

    /// Emit store global.
    pub fn emit_store_global(&mut self, global_idx: u32, src: Register) {
        self.emit_byte(Opcode::StoreGlobal.to_byte());
        self.emit_u32(global_idx);
        self.emit_reg(src);
    }

    /// Emit load from array index.
    pub fn emit_load_index(&mut self, dst: Register, arr: Register, idx: Register) {
        self.emit_byte(Opcode::LoadIndex.to_byte());
        self.emit_reg(dst);
        self.emit_reg(arr);
        self.emit_reg(idx);
    }

    /// Emit store to array index.
    pub fn emit_store_index(&mut self, arr: Register, idx: Register, val: Register) {
        self.emit_byte(Opcode::StoreIndex.to_byte());
        self.emit_reg(arr);
        self.emit_reg(idx);
        self.emit_reg(val);
    }

    /// Emit load field.
    pub fn emit_load_field(&mut self, dst: Register, obj: Register, field: u16) {
        self.emit_byte(Opcode::LoadField.to_byte());
        self.emit_reg(dst);
        self.emit_reg(obj);
        self.emit_u16(field);
    }

    /// Emit store field.
    pub fn emit_store_field(&mut self, obj: Register, field: u16, val: Register) {
        self.emit_byte(Opcode::StoreField.to_byte());
        self.emit_reg(obj);
        self.emit_u16(field);
        self.emit_reg(val);
    }

    /// Emit load immediate (small integer -128 to 127).
    pub fn emit_load_imm(&mut self, dst: Register, val: i8) {
        self.emit_byte(Opcode::LoadImm.to_byte());
        self.emit_reg(dst);
        self.emit_i8(val);
    }

    /// Emit load zero.
    pub fn emit_load_zero(&mut self, dst: Register) {
        self.emit_byte(Opcode::LoadZero.to_byte());
        self.emit_reg(dst);
    }

    /// Emit load true.
    pub fn emit_load_true(&mut self, dst: Register) {
        self.emit_byte(Opcode::LoadTrue.to_byte());
        self.emit_reg(dst);
    }

    /// Emit load false.
    pub fn emit_load_false(&mut self, dst: Register) {
        self.emit_byte(Opcode::LoadFalse.to_byte());
        self.emit_reg(dst);
    }

    // ========================================================================
    // Integer arithmetic
    // ========================================================================

    pub fn emit_add_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::AddI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_sub_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::SubI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_mul_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::MulI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_div_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::DivI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_mod_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::ModI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_neg_i(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::NegI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    // ========================================================================
    // Float arithmetic
    // ========================================================================

    pub fn emit_add_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::AddF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_sub_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::SubF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_mul_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::MulF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_div_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::DivF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_neg_f(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::NegF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    // ========================================================================
    // Comparisons (integer)
    // ========================================================================

    pub fn emit_eq_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::EqI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_ne_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::NeI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_lt_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::LtI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_le_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::LeI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_gt_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::GtI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_ge_i(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::GeI.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    // ========================================================================
    // Comparisons (float)
    // ========================================================================

    pub fn emit_eq_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::EqF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_lt_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::LtF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_le_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::LeF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_gt_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::GtF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_ge_f(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::GeF.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    // ========================================================================
    // Logical operations
    // ========================================================================

    pub fn emit_land(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::LAnd.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_lor(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::LOr.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_lnot(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::LNot.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    // ========================================================================
    // Bitwise operations
    // ========================================================================

    pub fn emit_and(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::And.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_or(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::Or.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_xor(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::Xor.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_not(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::Not.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    pub fn emit_shl(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::Shl.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    pub fn emit_shr(&mut self, dst: Register, a: Register, b: Register) {
        self.emit_byte(Opcode::Shr.to_byte());
        self.emit_reg(dst);
        self.emit_reg(a);
        self.emit_reg(b);
    }

    // ========================================================================
    // Arrays and tuples
    // ========================================================================

    pub fn emit_new_array(&mut self, dst: Register, capacity: u16) {
        self.emit_byte(Opcode::NewArray.to_byte());
        self.emit_reg(dst);
        self.emit_u16(capacity);
    }

    pub fn emit_array_len(&mut self, dst: Register, arr: Register) {
        self.emit_byte(Opcode::ArrayLen.to_byte());
        self.emit_reg(dst);
        self.emit_reg(arr);
    }

    pub fn emit_array_push(&mut self, arr: Register, val: Register) {
        self.emit_byte(Opcode::ArrayPush.to_byte());
        self.emit_reg(arr);
        self.emit_reg(val);
    }

    pub fn emit_new_tuple(&mut self, dst: Register, elements: &[Register]) {
        self.emit_byte(Opcode::NewTuple.to_byte());
        self.emit_reg(dst);
        self.emit_u16(elements.len() as u16);
        for elem in elements {
            self.emit_reg(*elem);
        }
    }

    pub fn emit_tuple_get(&mut self, dst: Register, tuple: Register, idx: u16) {
        self.emit_byte(Opcode::TupleGet.to_byte());
        self.emit_reg(dst);
        self.emit_reg(tuple);
        self.emit_u16(idx);
    }

    // ========================================================================
    // Type conversions
    // ========================================================================

    pub fn emit_i32_to_i64(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::I32ToI64.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    pub fn emit_i64_to_i32(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::I64ToI32.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    pub fn emit_i32_to_f32(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::I32ToF32.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    pub fn emit_i64_to_f64(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::I64ToF64.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    pub fn emit_f32_to_i32(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::F32ToI32.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    pub fn emit_f64_to_i64(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::F64ToI64.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    pub fn emit_f32_to_f64(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::F32ToF64.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    pub fn emit_f64_to_f32(&mut self, dst: Register, src: Register) {
        self.emit_byte(Opcode::F64ToF32.to_byte());
        self.emit_reg(dst);
        self.emit_reg(src);
    }

    // ========================================================================
    // Context operations
    // ========================================================================

    pub fn emit_ctx_sender(&mut self, dst: Register) {
        self.emit_byte(Opcode::CtxSender.to_byte());
        self.emit_reg(dst);
    }

    pub fn emit_ctx_block_height(&mut self, dst: Register) {
        self.emit_byte(Opcode::CtxBlockHeight.to_byte());
        self.emit_reg(dst);
    }

    pub fn emit_ctx_timestamp(&mut self, dst: Register) {
        self.emit_byte(Opcode::CtxTimestamp.to_byte());
        self.emit_reg(dst);
    }

    pub fn emit_ctx_value(&mut self, dst: Register) {
        self.emit_byte(Opcode::CtxValue.to_byte());
        self.emit_reg(dst);
    }

    pub fn emit_ctx_gas(&mut self, dst: Register) {
        self.emit_byte(Opcode::CtxGas.to_byte());
        self.emit_reg(dst);
    }

    pub fn emit_ctx_chain_id(&mut self, dst: Register) {
        self.emit_byte(Opcode::CtxChainId.to_byte());
        self.emit_reg(dst);
    }

    // ========================================================================
    // Atomic operations
    // ========================================================================

    pub fn emit_atomic_begin(&mut self, id: AtomicId) {
        self.emit_byte(Opcode::AtomicBegin.to_byte());
        self.emit_u16(id.0);
    }

    pub fn emit_atomic_commit(&mut self, id: AtomicId) {
        self.emit_byte(Opcode::AtomicCommit.to_byte());
        self.emit_u16(id.0);
    }

    pub fn emit_atomic_rollback(&mut self, id: AtomicId) {
        self.emit_byte(Opcode::AtomicRollback.to_byte());
        self.emit_u16(id.0);
    }

    // ========================================================================
    // Agent operations
    // ========================================================================

    pub fn emit_agent_self(&mut self, dst: Register) {
        self.emit_byte(Opcode::AgentSelf.to_byte());
        self.emit_reg(dst);
    }

    pub fn emit_emit(&mut self, event_id: u32, args: &[Register]) {
        self.emit_byte(Opcode::Emit.to_byte());
        self.emit_u32(event_id);
        self.emit_u16(args.len() as u16);
        for arg in args {
            self.emit_reg(*arg);
        }
    }

    // ========================================================================
    // Debug operations
    // ========================================================================

    pub fn emit_debug_print(&mut self, src: Register) {
        self.emit_byte(Opcode::DebugPrint.to_byte());
        self.emit_reg(src);
    }

    pub fn emit_assert(&mut self, cond: Register, msg_idx: ConstIdx) {
        self.emit_byte(Opcode::Assert.to_byte());
        self.emit_reg(cond);
        self.emit_const(msg_idx);
    }

    pub fn emit_panic(&mut self, msg_idx: ConstIdx) {
        self.emit_byte(Opcode::Panic.to_byte());
        self.emit_const(msg_idx);
    }

    // ========================================================================
    // Utility methods
    // ========================================================================

    /// Add integer constant and emit load.
    pub fn emit_int(&mut self, dst: Register, value: i64) -> BackendResult<()> {
        // Optimize small integers
        if value >= -128 && value <= 127 {
            self.emit_load_imm(dst, value as i8);
            return Ok(());
        }
        if value == 0 {
            self.emit_load_zero(dst);
            return Ok(());
        }

        let idx = self.const_pool.add_integer(value)?;
        self.emit_load_const(dst, idx);
        Ok(())
    }

    /// Add float constant and emit load.
    pub fn emit_float(&mut self, dst: Register, value: f64) -> BackendResult<()> {
        let idx = self.const_pool.add_float(value)?;
        self.emit_load_const(dst, idx);
        Ok(())
    }

    /// Add string constant and emit load.
    pub fn emit_string(&mut self, dst: Register, value: &str) -> BackendResult<()> {
        let idx = self.const_pool.add_string(value.to_string())?;
        self.emit_load_const(dst, idx);
        Ok(())
    }

    // ========================================================================
    // Memory Model Specialized Helpers
    // ========================================================================
    // These methods optimize Load/Store per memory model:
    // - Register: pure register move (no side effects)
    // - Stack: local stack slots (can use implicit frame layout)
    // - Heap: heap-allocated memory (requires address calculation)
    // - GlobalStorage: persistent on-chain storage (uses LoadGlobal/StoreGlobal)

    /// Load from register model (pure register move).
    /// This is the cheapest operation: just a move between registers.
    pub fn emit_load_register(&mut self, dst: Register, src: Register) {
        self.emit_mov(dst, src);
    }

    /// Store to register model (pure register move).
    /// Symmetric to load: move value into destination register.
    pub fn emit_store_register(&mut self, dst: Register, src: Register) {
        self.emit_mov(dst, src);
    }

    /// Load from stack slot.
    /// In X3, stack slots are function-local and accessed via frame layout.
    /// For now, we use LoadIndex into an implicit stack array.
    /// addr: stack slot index (register or immediate encoded in const pool)
    pub fn emit_load_stack(&mut self, dst: Register, addr: Register) {
        // Stack loads are treated like array access on a stack buffer
        // This will be refined by the VM with frame pointer arithmetic
        self.emit_load_index(dst, Register(0), addr); // r0 = implicit stack frame
    }

    /// Store to stack slot.
    /// Symmetric to load: write to implicit stack frame.
    pub fn emit_store_stack(&mut self, addr: Register, src: Register) {
        self.emit_store_index(Register(0), addr, src); // r0 = implicit stack frame
    }

    /// Load from heap memory.
    /// Heap loads require bounds checking and alignment; treated as array access.
    /// addr: heap address (pointer register)
    pub fn emit_load_heap(&mut self, dst: Register, addr: Register) {
        // Heap loads are array-like access on the heap arena
        // TODO: Add bounds checking opcode for production
        self.emit_load_index(dst, Register(1), addr); // r1 = implicit heap base
    }

    /// Store to heap memory.
    /// Symmetric to load: write to heap with bounds checking.
    pub fn emit_store_heap(&mut self, addr: Register, src: Register) {
        self.emit_store_index(Register(1), addr, src); // r1 = implicit heap base
    }

    /// Load from global persistent storage (on-chain state).
    /// GlobalStorage accesses are side-effecting and must not be elided.
    /// Wraps LoadGlobal with explicit global index encoding.
    pub fn emit_load_global_storage(&mut self, dst: Register, global_idx: u32) {
        self.emit_load_global(dst, global_idx);
    }

    /// Store to global persistent storage (on-chain state).
    /// Symmetric to load: persists value to on-chain storage.
    pub fn emit_store_global_storage(&mut self, global_idx: u32, src: Register) {
        self.emit_store_global(global_idx, src);
    }
}

impl Default for BytecodeEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_basic_instructions() {
        let mut emitter = BytecodeEmitter::new();

        emitter.emit_load_imm(Register(0), 42);
        emitter.emit_load_zero(Register(1));
        emitter.emit_add_i(Register(2), Register(0), Register(1));
        emitter.emit_ret(Register(2));

        let code = emitter.code();
        assert!(!code.is_empty());
        assert_eq!(code[0], Opcode::LoadImm.to_byte());
    }

    #[test]
    fn emit_with_labels() {
        let mut emitter = BytecodeEmitter::new();
        let span = Span { start: 0, end: 10 };

        let loop_start = emitter.create_label();
        let loop_end = emitter.create_label();

        emitter.define_label(loop_start);
        emitter.emit_load_true(Register(0));
        emitter.emit_jump_unless(Register(0), loop_end, span);
        emitter.emit_nop();
        emitter.emit_jump(loop_start, span);
        emitter.define_label(loop_end);
        emitter.emit_ret_void();

        emitter.finalize().unwrap();

        let code = emitter.code();
        assert!(!code.is_empty());
    }

    #[test]
    fn emit_constants() {
        let mut emitter = BytecodeEmitter::new();

        emitter.emit_int(Register(0), 42).unwrap();
        emitter.emit_int(Register(1), 0).unwrap();
        emitter.emit_int(Register(2), 1000000).unwrap();
        emitter.emit_float(Register(3), 3.14159).unwrap();
        emitter.emit_string(Register(4), "hello").unwrap();

        // Check constant pool has entries
        assert!(emitter.const_pool.len() >= 2); // Large int and float at minimum
    }

    #[test]
    fn forward_reference_patching() {
        let mut emitter = BytecodeEmitter::new();
        let span = Span { start: 0, end: 10 };

        let end_label = emitter.create_label();

        // Jump forward (before label is defined)
        emitter.emit_jump(end_label, span);
        emitter.emit_nop();
        emitter.emit_nop();
        emitter.define_label(end_label);
        emitter.emit_halt();

        emitter.finalize().unwrap();

        // The jump target should now point to the halt instruction
        let code = emitter.code();
        let target_offset = u32::from_le_bytes([code[1], code[2], code[3], code[4]]);
        // Target should be after jump (5 bytes) + 2 nops (2 bytes) = 7
        assert_eq!(target_offset, 7);
    }
}
