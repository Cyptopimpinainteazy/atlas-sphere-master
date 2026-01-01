//! Layout Computation and Jump Patching
//!
//! Computes final byte offsets for basic blocks and patches jump targets.

use std::collections::HashMap;

use crate::bc_format::FunctionEntry;
use crate::emit::Label;
use crate::error::{BackendError, BackendErrorKind, BackendResult};
use crate::opcode::Register;
use x3_common::Span;
use x3_hir::hir::SymbolId;

/// Tracks layout information during compilation.
#[derive(Debug)]
pub struct LayoutComputer {
    /// Function entry points (symbol → byte offset).
    function_offsets: HashMap<SymbolId, u32>,
    /// Global variable slots.
    global_slots: HashMap<SymbolId, u32>,
    /// Current function being compiled.
    current_function: Option<FunctionLayout>,
    /// All compiled functions.
    functions: Vec<CompiledFunction>,
}

/// Layout information for a single function.
#[derive(Debug, Clone)]
pub struct FunctionLayout {
    /// Symbol ID.
    pub symbol: SymbolId,
    /// Function name.
    pub name: String,
    /// Entry point (byte offset in code stream).
    pub entry_point: u32,
    /// Number of parameters.
    pub param_count: u8,
    /// Local variable slots (symbol → register).
    pub locals: HashMap<SymbolId, Register>,
    /// Next available register.
    pub next_register: u16,
    /// Maximum register used (for stack allocation).
    pub max_register: u16,
    /// Loop context stack for break/continue.
    pub loop_stack: Vec<LoopContext>,
}

/// Context for compiling a loop (for break/continue).
#[derive(Debug, Clone)]
pub struct LoopContext {
    /// Label to jump to for `continue`.
    pub continue_label: Label,
    /// Label to jump to for `break`.
    pub break_label: Label,
}

/// A fully compiled function.
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub symbol: SymbolId,
    pub name: String,
    pub entry_point: u32,
    pub param_count: u8,
    pub local_count: u16,
    pub max_stack: u16,
    pub return_type_tag: u8,
}

impl LayoutComputer {
    pub fn new() -> Self {
        Self {
            function_offsets: HashMap::new(),
            global_slots: HashMap::new(),
            current_function: None,
            functions: Vec::new(),
        }
    }

    /// Register a global variable slot.
    pub fn register_global(&mut self, symbol: SymbolId) -> u32 {
        let slot = self.global_slots.len() as u32;
        self.global_slots.insert(symbol, slot);
        slot
    }

    /// Get global variable slot.
    pub fn get_global_slot(&self, symbol: SymbolId) -> Option<u32> {
        self.global_slots.get(&symbol).copied()
    }

    /// Start compiling a new function.
    pub fn begin_function(
        &mut self,
        symbol: SymbolId,
        name: String,
        entry_point: u32,
        params: &[(SymbolId, String)],
    ) -> BackendResult<()> {
        if params.len() > 255 {
            return Err(BackendError::without_span(
                BackendErrorKind::TooManyParameters {
                    count: params.len(),
                    max: 255,
                },
            ));
        }

        let mut layout = FunctionLayout {
            symbol,
            name,
            entry_point,
            param_count: params.len() as u8,
            locals: HashMap::new(),
            next_register: 1, // r0 is return value
            max_register: 0,
            loop_stack: Vec::new(),
        };

        // Allocate registers for parameters
        for (param_symbol, _name) in params {
            let reg = layout.alloc_register();
            layout.locals.insert(*param_symbol, reg);
        }

        self.function_offsets.insert(symbol, entry_point);
        self.current_function = Some(layout);
        Ok(())
    }

    /// Finish compiling current function.
    pub fn end_function(&mut self, return_type_tag: u8) -> Option<CompiledFunction> {
        let layout = self.current_function.take()?;
        let compiled = CompiledFunction {
            symbol: layout.symbol,
            name: layout.name,
            entry_point: layout.entry_point,
            param_count: layout.param_count,
            local_count: layout
                .next_register
                .saturating_sub(1 + layout.param_count as u16),
            max_stack: layout.max_register + 1,
            return_type_tag,
        };
        self.functions.push(compiled.clone());
        Some(compiled)
    }

    /// Get current function layout (mutable).
    pub fn current_function_mut(&mut self) -> Option<&mut FunctionLayout> {
        self.current_function.as_mut()
    }

    /// Get current function layout.
    pub fn current_function(&self) -> Option<&FunctionLayout> {
        self.current_function.as_ref()
    }

    /// Lookup function entry point by symbol.
    pub fn get_function_offset(&self, symbol: SymbolId) -> Option<u32> {
        self.function_offsets.get(&symbol).copied()
    }

    /// Get all compiled functions as FunctionEntry records.
    pub fn get_function_entries(&self) -> Vec<FunctionEntry> {
        self.functions
            .iter()
            .map(|f| FunctionEntry {
                name: f.name.clone(),
                entry_point: f.entry_point,
                param_count: f.param_count,
                local_count: f.local_count,
                max_stack: f.max_stack,
                return_type_tag: f.return_type_tag,
            })
            .collect()
    }

    /// Get function index by symbol.
    pub fn get_function_index(&self, symbol: SymbolId) -> Option<u32> {
        self.functions
            .iter()
            .position(|f| f.symbol == symbol)
            .map(|i| i as u32)
    }
}

impl Default for LayoutComputer {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionLayout {
    /// Allocate a new register.
    pub fn alloc_register(&mut self) -> Register {
        let reg = Register(self.next_register);
        self.next_register += 1;
        if self.next_register > self.max_register {
            self.max_register = self.next_register;
        }
        reg
    }

    /// Allocate a register for a local variable.
    pub fn alloc_local(&mut self, symbol: SymbolId) -> Register {
        let reg = self.alloc_register();
        self.locals.insert(symbol, reg);
        reg
    }

    /// Get register for a local variable or parameter.
    pub fn get_local(&self, symbol: SymbolId) -> Option<Register> {
        self.locals.get(&symbol).copied()
    }

    /// Push a loop context.
    pub fn push_loop(&mut self, continue_label: Label, break_label: Label) {
        self.loop_stack.push(LoopContext {
            continue_label,
            break_label,
        });
    }

    /// Pop a loop context.
    pub fn pop_loop(&mut self) -> Option<LoopContext> {
        self.loop_stack.pop()
    }

    /// Get current loop context (for break/continue).
    pub fn current_loop(&self) -> Option<&LoopContext> {
        self.loop_stack.last()
    }

    /// Check if we're inside a loop.
    pub fn in_loop(&self) -> bool {
        !self.loop_stack.is_empty()
    }
}

/// Return type tags for function entries.
pub mod type_tags {
    pub const VOID: u8 = 0;
    pub const INT: u8 = 1;
    pub const FLOAT: u8 = 2;
    pub const BOOL: u8 = 3;
    pub const STRING: u8 = 4;
    pub const ARRAY: u8 = 5;
    pub const TUPLE: u8 = 6;
    pub const AGENT: u8 = 7;
    pub const OTHER: u8 = 255;
}

/// Convert x3_typeck Type to type tag.
pub fn type_to_tag(ty: &x3_typeck::Type) -> u8 {
    use x3_typeck::{PrimitiveType, TypeKind};

    match &ty.kind {
        TypeKind::Unit => type_tags::VOID,
        TypeKind::Primitive(PrimitiveType::Bool) => type_tags::BOOL,
        TypeKind::Primitive(p) if p.is_integer() => type_tags::INT,
        TypeKind::String => type_tags::STRING,
        TypeKind::Array { .. } => type_tags::ARRAY,
        TypeKind::Tuple(_) => type_tags::TUPLE,
        TypeKind::Agent(_) => type_tags::AGENT,
        _ => type_tags::OTHER,
    }
}

/// Check if a type is numeric (integer - x3 doesn't have floats in primitives).
/// Note: X3 type system doesn't have float primitives; this is for future extension.
pub fn is_float_type(_ty: &x3_typeck::Type) -> bool {
    // X3 type system currently doesn't have float primitives
    // All numeric ops use integer arithmetic
    false
}

/// Check if a type is integer.
pub fn is_int_type(ty: &x3_typeck::Type) -> bool {
    use x3_typeck::{PrimitiveType, TypeKind};
    match &ty.kind {
        TypeKind::Primitive(p) => p.is_integer(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_layout_registers() {
        let mut layout = FunctionLayout {
            symbol: SymbolId(0),
            name: "test".to_string(),
            entry_point: 0,
            param_count: 2,
            locals: HashMap::new(),
            next_register: 1,
            max_register: 0,
            loop_stack: Vec::new(),
        };

        // Allocate params
        let p0 = layout.alloc_register();
        let p1 = layout.alloc_register();
        assert_eq!(p0, Register(1));
        assert_eq!(p1, Register(2));

        // Allocate locals
        let local_sym = SymbolId(100);
        let local_reg = layout.alloc_local(local_sym);
        assert_eq!(local_reg, Register(3));
        assert_eq!(layout.get_local(local_sym), Some(Register(3)));
    }

    #[test]
    fn loop_context_stack() {
        let mut layout = FunctionLayout {
            symbol: SymbolId(0),
            name: "test".to_string(),
            entry_point: 0,
            param_count: 0,
            locals: HashMap::new(),
            next_register: 1,
            max_register: 0,
            loop_stack: Vec::new(),
        };

        assert!(!layout.in_loop());

        layout.push_loop(Label(0), Label(1));
        assert!(layout.in_loop());
        assert_eq!(layout.current_loop().unwrap().break_label, Label(1));

        layout.push_loop(Label(2), Label(3));
        assert_eq!(layout.current_loop().unwrap().break_label, Label(3));

        layout.pop_loop();
        assert_eq!(layout.current_loop().unwrap().break_label, Label(1));

        layout.pop_loop();
        assert!(!layout.in_loop());
    }

    #[test]
    fn layout_computer_functions() {
        let mut computer = LayoutComputer::new();

        let sym1 = SymbolId(1);
        let params = vec![(SymbolId(10), "x".to_string())];
        computer
            .begin_function(sym1, "foo".to_string(), 0, &params)
            .unwrap();
        computer.end_function(type_tags::INT);

        let sym2 = SymbolId(2);
        computer
            .begin_function(sym2, "bar".to_string(), 100, &[])
            .unwrap();
        computer.end_function(type_tags::VOID);

        let entries = computer.get_function_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "foo");
        assert_eq!(entries[0].entry_point, 0);
        assert_eq!(entries[1].name, "bar");
        assert_eq!(entries[1].entry_point, 100);
    }

    #[test]
    fn type_tags() {
        use x3_typeck::Type;

        assert_eq!(type_to_tag(&Type::unit()), type_tags::VOID);
        assert_eq!(type_to_tag(&Type::i64()), type_tags::INT);
        assert_eq!(type_to_tag(&Type::bool()), type_tags::BOOL);
        assert_eq!(type_to_tag(&Type::string()), type_tags::STRING);
    }
}
