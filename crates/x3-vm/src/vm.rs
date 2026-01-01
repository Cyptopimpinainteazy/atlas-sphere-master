//! X3 Virtual Machine - Deterministic Interpreter
//!
//! A register-based bytecode interpreter for X3BC modules.
//!
//! # Features
//!
//! - **Deterministic execution**: Same inputs always produce same outputs
//! - **Gas metering**: Configurable gas limits for bounded execution
//! - **Hostcall interface**: Extensible external function hooks
//! - **Atomic windows**: Track atomic begin/end for transaction safety
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                      VM                         │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
//! │  │ Module   │  │ Registers│  │ Call Stack   │  │
//! │  │ (code,   │  │ (256 max)│  │ (64 depth)   │  │
//! │  │  consts) │  │          │  │              │  │
//! │  └──────────┘  └──────────┘  └──────────────┘  │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
//! │  │ Operand  │  │ Gas      │  │ Atomic       │  │
//! │  │ Stack    │  │ Counter  │  │ Depth        │  │
//! │  └──────────┘  └──────────┘  └──────────────┘  │
//! └─────────────────────────────────────────────────┘
//! ```

use x3_backend::bc_format::{BytecodeModule, ConstValue};
use x3_backend::opcode::Opcode;

use crate::error::{VMError, VMErrorKind, VMResult};
use crate::hostcall::HostcallRegistry;

/// Maximum register count.
pub const MAX_REGISTERS: usize = 256;

/// Maximum call stack depth.
pub const MAX_CALL_DEPTH: usize = 64;

/// Maximum operand stack size.
pub const MAX_STACK_SIZE: usize = 1024;

/// Default gas limit.
pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;

/// VM configuration.
#[derive(Clone, Debug)]
pub struct VMConfig {
    /// Maximum gas allowed.
    pub gas_limit: u64,
    /// Maximum call stack depth.
    pub max_call_depth: usize,
    /// Maximum operand stack size.
    pub max_stack_size: usize,
    /// Enable debug tracing.
    pub trace: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            gas_limit: DEFAULT_GAS_LIMIT,
            max_call_depth: MAX_CALL_DEPTH,
            max_stack_size: MAX_STACK_SIZE,
            trace: false,
        }
    }
}

/// Runtime value in the VM.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// 64-bit signed integer.
    I64(i64),
    /// 64-bit floating point.
    F64(f64),
    /// Boolean.
    Bool(bool),
    /// String (heap allocated).
    String(String),
    /// Byte array.
    Bytes(Vec<u8>),
    /// Address/pointer.
    Addr(u64),
    /// Unit (void/null).
    Unit,
}

impl Value {
    /// Convert constant value to runtime value.
    pub fn from_const(c: &ConstValue) -> Self {
        match c {
            ConstValue::Integer(i) => Value::I64(*i),
            ConstValue::Float(f) => Value::F64(*f),
            ConstValue::String(s) => Value::String(s.clone()),
            ConstValue::Bool(b) => Value::Bool(*b),
            ConstValue::Bytes(b) => Value::Bytes(b.clone()),
        }
    }

    /// Get as i64.
    pub fn as_i64(&self) -> VMResult<i64> {
        match self {
            Value::I64(v) => Ok(*v),
            _ => Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                "i64".to_string(),
                format!("{:?}", self),
            ))),
        }
    }

    /// Get as f64.
    pub fn as_f64(&self) -> VMResult<f64> {
        match self {
            Value::F64(v) => Ok(*v),
            _ => Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                "f64".to_string(),
                format!("{:?}", self),
            ))),
        }
    }

    /// Get as bool.
    pub fn as_bool(&self) -> VMResult<bool> {
        match self {
            Value::Bool(v) => Ok(*v),
            // Truthy conversion
            Value::I64(v) => Ok(*v != 0),
            _ => Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                "bool".to_string(),
                format!("{:?}", self),
            ))),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Unit
    }
}

/// Call frame on the call stack.
#[derive(Clone, Debug)]
pub struct Frame {
    /// Instruction pointer (offset in code).
    pub ip: usize,
    /// Base register index for this frame.
    pub base: usize,
    /// Return address (IP to return to).
    pub ret_addr: usize,
    /// Function index.
    pub func_idx: usize,
}

/// Execution result.
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    /// Return value (if any).
    pub value: Option<Value>,
    /// Gas consumed.
    pub gas_used: u64,
    /// Number of instructions executed.
    pub instruction_count: u64,
}

/// Snapshot of VM state for rollback support.
#[derive(Clone, Debug)]
pub struct VMSnapshot {
    /// Register state.
    pub regs: Vec<Value>,
    /// Global variables state.
    pub globals: Vec<Value>,
    /// Operand stack state.
    pub stack: Vec<Value>,
    /// Atomic ID that created this snapshot.
    pub atomic_id: u16,
}

/// The X3 Virtual Machine.
pub struct VM {
    /// Loaded module.
    module: BytecodeModule,
    /// Register file.
    regs: Vec<Value>,
    /// Operand stack.
    stack: Vec<Value>,
    /// Call stack.
    call_stack: Vec<Frame>,
    /// Configuration.
    config: VMConfig,
    /// Gas consumed.
    gas_used: u64,
    /// Atomic nesting depth.
    atomic_depth: usize,
    /// Hostcall registry.
    hostcalls: HostcallRegistry,
    /// Instruction count.
    instruction_count: u64,
    /// Global variables storage.
    globals: Vec<Value>,
    /// Maximum number of globals.
    max_globals: usize,
    /// State snapshots for atomic rollback.
    snapshots: Vec<VMSnapshot>,
}

impl VM {
    /// Create a new VM with the given module.
    pub fn new(module: BytecodeModule) -> Self {
        Self::with_config(module, VMConfig::default())
    }

    /// Create a new VM with custom configuration.
    pub fn with_config(module: BytecodeModule, config: VMConfig) -> Self {
        // Determine max globals from module globals table
        let max_globals = std::cmp::max(module.globals.len(), 256);
        
        // Initialize globals from module's global entries
        let mut globals = vec![Value::Unit; max_globals];
        for (i, global) in module.globals.iter().enumerate() {
            // Look up initial value in constant pool
            if let Some(const_val) = module.const_pool.get(global.init_const) {
                globals[i] = Value::from_const(const_val);
            }
        }
        
        Self {
            module,
            regs: vec![Value::Unit; MAX_REGISTERS],
            stack: Vec::with_capacity(config.max_stack_size),
            call_stack: Vec::with_capacity(config.max_call_depth),
            config,
            gas_used: 0,
            atomic_depth: 0,
            hostcalls: HostcallRegistry::with_standard(),
            instruction_count: 0,
            globals,
            max_globals,
            snapshots: Vec::new(),
        }
    }

    /// Create a VM from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> VMResult<Self> {
        let module = BytecodeModule::from_bytes(bytes)
            .map_err(|e| VMError::without_ip(VMErrorKind::ModuleLoadError(format!("{:?}", e))))?;
        Ok(Self::new(module))
    }

    /// Register a hostcall.
    pub fn register_hostcall<F>(
        &mut self,
        id: u8,
        name: impl Into<String>,
        arg_count: usize,
        handler: F,
    ) where
        F: Fn(&[Value]) -> VMResult<Option<Value>> + Send + Sync + 'static,
    {
        self.hostcalls.register(id, name, arg_count, handler);
    }

    /// Get the loaded module.
    pub fn module(&self) -> &BytecodeModule {
        &self.module
    }

    /// Get gas used.
    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    /// Call a function by index.
    pub fn call_function(&mut self, func_idx: usize, args: &[Value]) -> VMResult<ExecutionResult> {
        // Validate function index
        if func_idx >= self.module.functions.len() {
            return Err(VMError::without_ip(VMErrorKind::FunctionNotFound(func_idx)));
        }

        let func = &self.module.functions[func_idx];

        // Validate argument count
        if args.len() != func.param_count as usize {
            return Err(VMError::without_ip(VMErrorKind::ArgumentCountMismatch(
                func.param_count as usize,
                args.len(),
            )));
        }

        // Set up registers with arguments
        for (i, arg) in args.iter().enumerate() {
            self.regs[i] = arg.clone();
        }

        // Push initial frame
        self.call_stack.push(Frame {
            ip: func.entry_point as usize,
            base: 0,
            ret_addr: usize::MAX, // Sentinel for top-level return
            func_idx,
        });

        // Execute
        let result = self.execute()?;

        Ok(ExecutionResult {
            value: result,
            gas_used: self.gas_used,
            instruction_count: self.instruction_count,
        })
    }

    /// Call a function by name.
    pub fn call_function_by_name(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> VMResult<ExecutionResult> {
        let func_idx = self
            .module
            .functions
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| {
                VMError::without_ip(VMErrorKind::FunctionNotFoundByName(name.to_string()))
            })?;
        self.call_function(func_idx, args)
    }

    /// Main execution loop.
    fn execute(&mut self) -> VMResult<Option<Value>> {
        loop {
            // Check gas limit
            if self.gas_used >= self.config.gas_limit {
                return Err(self.error(VMErrorKind::GasLimitExceeded));
            }

            // Get current frame
            let frame = match self.call_stack.last_mut() {
                Some(f) => f,
                None => return Ok(None), // No frames left
            };

            let ip = frame.ip;

            // Bounds check
            if ip >= self.module.code.len() {
                return Err(self.error_at(ip, VMErrorKind::InstructionPointerOutOfBounds));
            }

            // Fetch opcode
            let opcode_byte = self.module.code[ip];
            let opcode = Opcode::from_byte(opcode_byte)
                .ok_or_else(|| self.error_at(ip, VMErrorKind::InvalidOpcode(opcode_byte)))?;

            // Consume gas
            self.gas_used += self.opcode_gas_cost(opcode);
            self.instruction_count += 1;

            // Trace if enabled
            if self.config.trace {
                log::trace!("[VM] IP={:04x} {:?}", ip, opcode);
            }

            // Execute instruction
            match self.execute_instruction(opcode, ip)? {
                StepResult::Continue(next_ip) => {
                    if let Some(f) = self.call_stack.last_mut() {
                        f.ip = next_ip;
                    }
                }
                StepResult::Return(value) => {
                    // Pop frame
                    let frame = self.call_stack.pop().unwrap();
                    if frame.ret_addr == usize::MAX {
                        // Top-level return
                        return Ok(value);
                    }
                    // Set return value in caller's r0
                    if let Some(v) = value {
                        self.regs[0] = v;
                    }
                    // Resume at return address
                    if let Some(f) = self.call_stack.last_mut() {
                        f.ip = frame.ret_addr;
                    }
                }
                StepResult::Halt => {
                    return Ok(None);
                }
            }
        }
    }

    /// Execute a single instruction.
    fn execute_instruction(&mut self, opcode: Opcode, ip: usize) -> VMResult<StepResult> {
        let code = &self.module.code;

        match opcode {
            // ================================================================
            // Control Flow
            // ================================================================
            Opcode::Nop => Ok(StepResult::Continue(ip + 1)),

            Opcode::Jump => {
                let target = self.read_u32(ip + 1)? as usize;
                Ok(StepResult::Continue(target))
            }

            Opcode::JumpIf => {
                let cond_reg = self.read_u8(ip + 1)? as usize;
                let target = self.read_u32(ip + 2)? as usize;
                if self.regs[cond_reg].as_bool()? {
                    Ok(StepResult::Continue(target))
                } else {
                    Ok(StepResult::Continue(ip + 6))
                }
            }

            Opcode::JumpUnless => {
                let cond_reg = self.read_u8(ip + 1)? as usize;
                let target = self.read_u32(ip + 2)? as usize;
                if !self.regs[cond_reg].as_bool()? {
                    Ok(StepResult::Continue(target))
                } else {
                    Ok(StepResult::Continue(ip + 6))
                }
            }

            Opcode::Call => {
                // call dst:reg func:u32 argc:u16 [args:reg...]
                let dst = self.read_u8(ip + 1)? as usize;
                let func_idx = self.read_u32(ip + 2)? as usize;
                let argc = self.read_u16(ip + 6)? as usize;

                if func_idx >= self.module.functions.len() {
                    return Err(self.error_at(ip, VMErrorKind::FunctionNotFound(func_idx)));
                }

                if self.call_stack.len() >= self.config.max_call_depth {
                    return Err(self.error_at(
                        ip,
                        VMErrorKind::StackOverflow(
                            self.call_stack.len(),
                            self.config.max_call_depth,
                        ),
                    ));
                }

                // Read argument registers
                let mut args = Vec::with_capacity(argc);
                for i in 0..argc {
                    let arg_reg = self.read_u8(ip + 8 + i)? as usize;
                    args.push(self.regs[arg_reg].clone());
                }

                let func = &self.module.functions[func_idx];
                let ret_addr = ip + 8 + argc;

                // Calculate base for nested call (preserve caller's registers)
                let current_base = self.call_stack.last().map(|f| f.base).unwrap_or(0);
                let local_slots = if func.local_count > 0 { func.local_count as usize } else { 16 };
                let new_base = current_base + local_slots;
                
                // Set up arguments in callee's register window
                for (i, arg) in args.into_iter().enumerate() {
                    let reg_idx = new_base + i;
                    if reg_idx < MAX_REGISTERS {
                        self.regs[reg_idx] = arg;
                    }
                }

                // Push frame with proper base
                self.call_stack.push(Frame {
                    ip: func.entry_point as usize,
                    base: new_base,
                    ret_addr,
                    func_idx,
                });

                Ok(StepResult::Continue(func.entry_point as usize))
            }

            Opcode::Ret => {
                let src = self.read_u8(ip + 1)? as usize;
                let value = self.regs[src].clone();
                Ok(StepResult::Return(Some(value)))
            }

            Opcode::RetVoid => Ok(StepResult::Return(None)),

            Opcode::Halt => Ok(StepResult::Halt),

            // ================================================================
            // Load/Store
            // ================================================================
            Opcode::LoadConst => {
                let dst = self.read_u8(ip + 1)? as usize;
                let idx = self.read_u32(ip + 2)? as usize;

                if idx >= self.module.const_pool.entries.len() {
                    return Err(self.error_at(ip, VMErrorKind::ConstPoolOutOfBounds(idx)));
                }

                self.regs[dst] = Value::from_const(&self.module.const_pool.entries[idx]);
                Ok(StepResult::Continue(ip + 6))
            }

            Opcode::Mov => {
                let dst = self.read_u8(ip + 1)? as usize;
                let src = self.read_u8(ip + 2)? as usize;
                self.regs[dst] = self.regs[src].clone();
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::LoadGlobal => {
                let dst = self.read_u8(ip + 1)? as usize;
                let idx = self.read_u32(ip + 2)? as usize;
                
                if idx >= self.max_globals {
                    return Err(self.error_at(ip, VMErrorKind::GlobalOutOfBounds(idx, self.max_globals)));
                }
                
                self.regs[dst] = self.globals[idx].clone();
                Ok(StepResult::Continue(ip + 6))
            }

            Opcode::StoreGlobal => {
                let idx = self.read_u32(ip + 1)? as usize;
                let src = self.read_u8(ip + 5)? as usize;
                
                if idx >= self.max_globals {
                    return Err(self.error_at(ip, VMErrorKind::GlobalOutOfBounds(idx, self.max_globals)));
                }
                
                self.globals[idx] = self.regs[src].clone();
                Ok(StepResult::Continue(ip + 6))
            }

            Opcode::LoadImm => {
                let dst = self.read_u8(ip + 1)? as usize;
                let val = self.read_i8(ip + 2)?;
                self.regs[dst] = Value::I64(val as i64);
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::LoadZero => {
                let dst = self.read_u8(ip + 1)? as usize;
                self.regs[dst] = Value::I64(0);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::LoadTrue => {
                let dst = self.read_u8(ip + 1)? as usize;
                self.regs[dst] = Value::Bool(true);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::LoadFalse => {
                let dst = self.read_u8(ip + 1)? as usize;
                self.regs[dst] = Value::Bool(false);
                Ok(StepResult::Continue(ip + 2))
            }

            // ================================================================
            // Integer Arithmetic
            // ================================================================
            Opcode::AddI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va.wrapping_add(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::SubI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va.wrapping_sub(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::MulI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va.wrapping_mul(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::DivI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                if vb == 0 {
                    return Err(self.error_at(ip, VMErrorKind::DivisionByZero));
                }
                self.regs[dst] = Value::I64(va / vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::ModI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                if vb == 0 {
                    return Err(self.error_at(ip, VMErrorKind::DivisionByZero));
                }
                self.regs[dst] = Value::I64(va % vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NegI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let src = self.read_u8(ip + 2)? as usize;
                let v = self.regs[src].as_i64()?;
                self.regs[dst] = Value::I64(v.wrapping_neg());
                Ok(StepResult::Continue(ip + 3))
            }

            // ================================================================
            // Float Arithmetic
            // ================================================================
            Opcode::AddF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::F64(va + vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::SubF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::F64(va - vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::MulF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::F64(va * vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::DivF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                // Float division by zero produces infinity, not error
                self.regs[dst] = Value::F64(va / vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NegF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let src = self.read_u8(ip + 2)? as usize;
                let v = self.regs[src].as_f64()?;
                self.regs[dst] = Value::F64(-v);
                Ok(StepResult::Continue(ip + 3))
            }

            // ================================================================
            // Comparisons
            // ================================================================
            Opcode::EqI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va == vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NeI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va != vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LtI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va < vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LeI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va <= vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GtI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va > vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GeI => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va >= vb);
                Ok(StepResult::Continue(ip + 4))
            }

            // Float comparisons
            Opcode::EqF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va == vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NeF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va != vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LtF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va < vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LeF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va <= vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GtF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va > vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GeF => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va >= vb);
                Ok(StepResult::Continue(ip + 4))
            }

            // ================================================================
            // Bitwise Operations
            // ================================================================
            Opcode::And => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va & vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::Or => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va | vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::Xor => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va ^ vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::Not => {
                let dst = self.read_u8(ip + 1)? as usize;
                let src = self.read_u8(ip + 2)? as usize;
                let v = self.regs[src].as_i64()?;
                self.regs[dst] = Value::I64(!v);
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::Shl => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()? as u32;
                self.regs[dst] = Value::I64(va.wrapping_shl(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::Shr => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()? as u32;
                self.regs[dst] = Value::I64(va.wrapping_shr(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::UShr => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_i64()? as u64;
                let vb = self.regs[b].as_i64()? as u32;
                self.regs[dst] = Value::I64(va.wrapping_shr(vb) as i64);
                Ok(StepResult::Continue(ip + 4))
            }

            // ================================================================
            // Logical Operations
            // ================================================================
            Opcode::LAnd => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_bool()?;
                let vb = self.regs[b].as_bool()?;
                self.regs[dst] = Value::Bool(va && vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LOr => {
                let dst = self.read_u8(ip + 1)? as usize;
                let a = self.read_u8(ip + 2)? as usize;
                let b = self.read_u8(ip + 3)? as usize;
                let va = self.regs[a].as_bool()?;
                let vb = self.regs[b].as_bool()?;
                self.regs[dst] = Value::Bool(va || vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LNot => {
                let dst = self.read_u8(ip + 1)? as usize;
                let src = self.read_u8(ip + 2)? as usize;
                let v = self.regs[src].as_bool()?;
                self.regs[dst] = Value::Bool(!v);
                Ok(StepResult::Continue(ip + 3))
            }

            // ================================================================
            // Atomic Operations
            // ================================================================
            Opcode::AtomicBegin => {
                let atomic_id = self.read_u16(ip + 1)?;
                
                // Create snapshot for potential rollback
                let snapshot = VMSnapshot {
                    regs: self.regs.clone(),
                    globals: self.globals.clone(),
                    stack: self.stack.clone(),
                    atomic_id,
                };
                self.snapshots.push(snapshot);
                
                self.atomic_depth += 1;
                Ok(StepResult::Continue(ip + 3)) // opcode + id:u16
            }

            Opcode::AtomicCommit => {
                let atomic_id = self.read_u16(ip + 1)?;
                
                if self.atomic_depth == 0 {
                    return Err(self.error_at(ip, VMErrorKind::AtomicEndWithoutBegin));
                }
                
                // Find and remove the matching snapshot
                if let Some(pos) = self.snapshots.iter().rposition(|s| s.atomic_id == atomic_id) {
                    self.snapshots.remove(pos);
                }
                
                self.atomic_depth -= 1;
                Ok(StepResult::Continue(ip + 3)) // opcode + id:u16
            }

            Opcode::AtomicRollback => {
                let atomic_id = self.read_u16(ip + 1)?;
                
                if self.atomic_depth == 0 {
                    return Err(self.error_at(ip, VMErrorKind::AtomicRollbackWithoutBegin));
                }
                
                // Find the matching snapshot and restore state
                if let Some(pos) = self.snapshots.iter().rposition(|s| s.atomic_id == atomic_id) {
                    let snapshot = self.snapshots[pos].clone();
                    
                    // Restore state from snapshot
                    self.regs = snapshot.regs;
                    self.globals = snapshot.globals;
                    self.stack = snapshot.stack;
                    
                    // Remove this and all newer snapshots
                    self.snapshots.truncate(pos);
                    
                    // Decrease atomic depth appropriately
                    self.atomic_depth = self.snapshots.len();
                } else {
                    // No matching snapshot found, abort everything
                    self.atomic_depth = 0;
                    self.snapshots.clear();
                }
                
                return Err(self.error_at(ip, VMErrorKind::AtomicAborted));
            }

            // ================================================================
            // Debug Operations (no-op in production)
            // ================================================================
            Opcode::DebugPrint => {
                let src = self.read_u8(ip + 1)? as usize;
                log::debug!("[DEBUG] r{} = {:?}", src, self.regs[src]);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::Breakpoint => {
                log::debug!("[DEBUG] BREAK at IP={}", ip);
                Ok(StepResult::Continue(ip + 1))
            }

            Opcode::Assert => {
                let cond = self.read_u8(ip + 1)? as usize;
                let _msg_idx = self.read_u32(ip + 2)?;
                if !self.regs[cond].as_bool()? {
                    return Err(self.error_at(ip, VMErrorKind::AssertionFailed));
                }
                Ok(StepResult::Continue(ip + 6))
            }

            Opcode::Panic => {
                let msg_idx = self.read_u32(ip + 1)? as usize;
                let msg = if msg_idx < self.module.const_pool.entries.len() {
                    if let ConstValue::String(s) = &self.module.const_pool.entries[msg_idx] {
                        s.clone()
                    } else {
                        "panic".to_string()
                    }
                } else {
                    "panic".to_string()
                };
                return Err(self.error_at(ip, VMErrorKind::UserPanic(msg)));
            }

            // ================================================================
            // Unimplemented opcodes return error
            // ================================================================
            _ => {
                let opc = self.module.code[ip];
                Err(self.error_at(ip, VMErrorKind::UnimplementedOpcode(opc)))
            }
        }
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    fn read_u8(&self, offset: usize) -> VMResult<u8> {
        self.module
            .code
            .get(offset)
            .copied()
            .ok_or_else(|| self.error_at(offset, VMErrorKind::InstructionPointerOutOfBounds))
    }

    fn read_i8(&self, offset: usize) -> VMResult<i8> {
        Ok(self.read_u8(offset)? as i8)
    }

    fn read_u16(&self, offset: usize) -> VMResult<u16> {
        if offset + 2 > self.module.code.len() {
            return Err(self.error_at(offset, VMErrorKind::InstructionPointerOutOfBounds));
        }
        Ok(u16::from_le_bytes([
            self.module.code[offset],
            self.module.code[offset + 1],
        ]))
    }

    fn read_u32(&self, offset: usize) -> VMResult<u32> {
        if offset + 4 > self.module.code.len() {
            return Err(self.error_at(offset, VMErrorKind::InstructionPointerOutOfBounds));
        }
        Ok(u32::from_le_bytes([
            self.module.code[offset],
            self.module.code[offset + 1],
            self.module.code[offset + 2],
            self.module.code[offset + 3],
        ]))
    }

    fn error(&self, kind: VMErrorKind) -> VMError {
        VMError::without_ip(kind)
    }

    fn error_at(&self, ip: usize, kind: VMErrorKind) -> VMError {
        VMError::at_ip(ip, kind)
    }

    fn opcode_gas_cost(&self, opcode: Opcode) -> u64 {
        match opcode {
            Opcode::Nop => 1,
            Opcode::Jump | Opcode::JumpIf | Opcode::JumpUnless => 2,
            Opcode::Call => 10,
            Opcode::Ret | Opcode::RetVoid => 2,
            Opcode::Halt => 1,
            Opcode::LoadConst | Opcode::Mov | Opcode::LoadImm => 1,
            Opcode::LoadGlobal | Opcode::StoreGlobal => 3,
            Opcode::AddI | Opcode::SubI | Opcode::MulI => 1,
            Opcode::DivI | Opcode::ModI => 5,
            Opcode::AddF | Opcode::SubF | Opcode::MulF | Opcode::DivF => 2,
            Opcode::EqI | Opcode::NeI | Opcode::LtI | Opcode::LeI | Opcode::GtI | Opcode::GeI => 1,
            Opcode::And | Opcode::Or | Opcode::Xor | Opcode::Not => 1,
            Opcode::Shl | Opcode::Shr | Opcode::UShr => 1,
            Opcode::AtomicBegin | Opcode::AtomicCommit => 5,
            Opcode::AtomicRollback => 10,
            _ => 1,
        }
    }
}

/// Result of executing one instruction.
enum StepResult {
    /// Continue to next IP.
    Continue(usize),
    /// Return from current function.
    Return(Option<Value>),
    /// Halt execution.
    Halt,
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_backend::bc_format_helpers;

    #[test]
    fn vm_smoke_add() {
        // Use the helper to assemble a simple module
        let bytes = bc_format_helpers::assemble_simple_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        // Call function 0 with no arguments
        let result = vm.call_function(0, &[]).expect("execution should succeed");

        // Should return 42 + 7 = 49
        assert_eq!(result.value, Some(Value::I64(49)));
        assert!(result.gas_used > 0);
        assert!(result.instruction_count > 0);
    }

    #[test]
    fn vm_with_parameters() {
        let bytes = bc_format_helpers::assemble_param_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let result = vm
            .call_function(0, &[Value::I64(10), Value::I64(20)])
            .expect("execution should succeed");

        assert_eq!(result.value, Some(Value::I64(30)));
    }

    #[test]
    fn vm_branch_positive() {
        let bytes = bc_format_helpers::assemble_branch_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        // Positive value: should return the value
        let result = vm
            .call_function(0, &[Value::I64(5)])
            .expect("execution should succeed");

        assert_eq!(result.value, Some(Value::I64(5)));
    }

    #[test]
    fn vm_branch_negative() {
        let bytes = bc_format_helpers::assemble_branch_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        // Negative value: should return 0
        let result = vm
            .call_function(0, &[Value::I64(-5)])
            .expect("execution should succeed");

        assert_eq!(result.value, Some(Value::I64(0)));
    }

    #[test]
    fn vm_halt() {
        let bytes = bc_format_helpers::assemble_halt_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let result = vm.call_function(0, &[]).expect("execution should succeed");

        assert_eq!(result.value, None);
    }

    #[test]
    fn vm_gas_limit() {
        let bytes = bc_format_helpers::assemble_simple_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        // Set very low gas limit
        vm.config.gas_limit = 1;

        let result = vm.call_function(0, &[]);
        assert!(result.is_err());
        match result {
            Err(e) => assert!(matches!(e.kind, VMErrorKind::GasLimitExceeded)),
            _ => panic!("expected gas limit error"),
        }
    }
}
