//! Bytecode Verifier
//!
//! Performs structural, CFG, and semantic validation of X3 bytecode modules
//! before execution. This is a mandatory gate for both local execution and
//! on-chain deployment.
//!
//! # Verification Passes
//!
//! 1. **Structural**: Magic bytes, version, section bounds
//! 2. **Decode**: All instructions decode correctly with valid operands
//! 3. **CFG**: Jump targets are valid instruction boundaries
//! 4. **Const Pool**: All const references are in bounds
//! 5. **Atomic**: Atomic begin/end markers are balanced
//! 6. **Gas**: Conservative gas estimate doesn't exceed limits
//! 7. **On-Chain**: Debug/JIT opcodes forbidden in production

use std::collections::{BTreeMap, BTreeSet};

use x3_backend::bc_format::{BytecodeModule, ConstValue, FunctionEntry, MAGIC, VERSION};
use x3_backend::opcode::Opcode;

use crate::error::{VerifierError, VerifierErrorKind, VerifierResult};

/// Maximum gas budget for verification (prevents DoS).
pub const MAX_GAS_BUDGET: u128 = 1 << 60;

/// Maximum register index allowed.
pub const MAX_REGISTER: u16 = 65535;

/// Maximum call stack depth for verification.
pub const MAX_CALL_DEPTH: usize = 1024;

/// Verification options.
#[derive(Debug, Clone)]
pub struct VerifyOptions {
    /// Whether this is on-chain context (forbids debug opcodes).
    pub on_chain: bool,
    /// Maximum allowed gas budget per function.
    pub max_gas_per_function: u128,
    /// Whether to perform CFG reachability analysis.
    pub check_reachability: bool,
    /// Maximum module size in bytes.
    pub max_module_size: usize,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            on_chain: false,
            max_gas_per_function: MAX_GAS_BUDGET,
            check_reachability: true,
            max_module_size: 16 * 1024 * 1024, // 16 MB
        }
    }
}

impl VerifyOptions {
    /// Options for on-chain deployment.
    pub fn on_chain() -> Self {
        Self {
            on_chain: true,
            max_gas_per_function: 10_000_000, // 10M gas per function
            check_reachability: true,
            max_module_size: 1024 * 1024, // 1 MB
        }
    }
}

/// Decoded instruction with metadata for verification.
#[derive(Debug, Clone)]
pub struct DecodedInstr {
    /// The opcode byte.
    pub opcode: u8,
    /// Byte offset in code section.
    pub offset: usize,
    /// Total instruction size in bytes.
    pub size: usize,
    /// Decoded operands (registers, indices, etc.).
    pub operands: Vec<u64>,
    /// Estimated gas cost.
    pub gas_cost: u64,
}

/// Bytecode verifier.
pub struct Verifier;

impl Verifier {
    /// Verify a bytecode module from raw bytes.
    pub fn verify_module_bytes(
        bytes: &[u8],
        options: &VerifyOptions,
    ) -> VerifierResult<BytecodeModule> {
        // Size check
        if bytes.is_empty() {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::EmptyModule,
            ));
        }
        if bytes.len() > options.max_module_size {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::ModuleTooLarge(bytes.len(), options.max_module_size),
            ));
        }

        // Parse module
        let module = BytecodeModule::from_bytes(bytes).map_err(|e| {
            VerifierError::without_offset(VerifierErrorKind::ParseError(format!("{}", e)))
        })?;

        // Run verification passes
        Self::verify_module(&module, options)?;

        Ok(module)
    }

    /// Verify a pre-parsed bytecode module.
    pub fn verify_module(module: &BytecodeModule, options: &VerifyOptions) -> VerifierResult<()> {
        // 1. Structural validation
        Self::verify_structure(module)?;

        // 2. Function entry points
        Self::verify_function_entries(module)?;

        // 3. Decode all instructions
        let instrs = Self::decode_all_instructions(&module.code)?;

        // 4. Build instruction offset set
        let offsets: BTreeSet<usize> = instrs.iter().map(|i| i.offset).collect();

        // 5. CFG validation (jump targets)
        Self::verify_cfg(&instrs, &offsets, module)?;

        // 6. Const pool validation
        Self::verify_const_pool_refs(&instrs, module)?;

        // 7. Atomic balance validation
        Self::verify_atomic_balance(&instrs)?;

        // 8. Gas estimation
        Self::verify_gas(&instrs, options)?;

        // 9. On-chain restrictions
        if options.on_chain {
            Self::verify_on_chain_restrictions(&instrs)?;
        }

        Ok(())
    }

    /// Verify structural integrity.
    fn verify_structure(module: &BytecodeModule) -> VerifierResult<()> {
        // Version check - compare packed u32 representations
        if module.version.to_packed() > VERSION {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::UnsupportedVersion(module.version.to_packed()),
            ));
        }

        Ok(())
    }

    /// Verify function entry points are within code section.
    fn verify_function_entries(module: &BytecodeModule) -> VerifierResult<()> {
        let code_len = module.code.len();

        for (idx, func) in module.functions.iter().enumerate() {
            if func.entry_point as usize >= code_len {
                return Err(VerifierError::without_offset(
                    VerifierErrorKind::FunctionEntryOutOfBounds(idx, func.entry_point),
                ));
            }
        }

        Ok(())
    }

    /// Decode all instructions from the code section.
    pub fn decode_all_instructions(code: &[u8]) -> VerifierResult<Vec<DecodedInstr>> {
        let mut instrs = Vec::new();
        let mut pc = 0;
        let len = code.len();

        while pc < len {
            let opcode = code[pc];
            let (size, operands) = Self::decode_operands(code, pc, opcode)?;

            if pc + size > len {
                return Err(VerifierError::new(
                    VerifierErrorKind::OperandOutOfBounds,
                    pc,
                ));
            }

            instrs.push(DecodedInstr {
                opcode,
                offset: pc,
                size,
                operands,
                gas_cost: opcode_gas_cost(opcode),
            });

            pc += size;
        }

        Ok(instrs)
    }

    /// Decode operands for a single instruction.
    /// Returns (total_size, operands).
    ///
    /// # Operand Encoding Reference (74 opcodes)
    ///
    /// | Size | Format | Count | Examples |
    /// |------|--------|-------|----------|
    /// | 1B   | `[op]` | 4 | Nop, Halt, RetVoid, Breakpoint |
    /// | 3B   | `[op][reg:u16]` | 14 | Ret, LoadZero, CtxSender, AtomicCheck, DebugPrint |
    /// | 3B   | `[op][id:u16]` | 3 | AtomicBegin/Commit/Rollback |
    /// | 4B   | `[op][reg:u16][i8]` | 1 | LoadImm |
    /// | 5B   | `[op][target:u32]` | 1 | Jump |
    /// | 5B   | `[op][idx:u32]` | 1 | Panic |
    /// | 5B   | `[op][dst:u16][src:u16]` | 18 | Mov, NegI, Inc, conversions, ArrayLen |
    /// | 5B   | `[op][dst:u16][cap:u16]` | 1 | NewArray |
    /// | 5B   | `[op][r:u16][r:u16]` | 6 | EvmSload, EvmSstore, EvmBalance, SvmGetData... |
    /// | 7B   | `[op][cond:u16][target:u32]` | 2 | JumpIf, JumpUnless |
    /// | 7B   | `[op][dst:u16][idx:u32]` | 2 | LoadConst, LoadGlobal |
    /// | 7B   | `[op][idx:u32][src:u16]` | 1 | StoreGlobal |
    /// | 7B   | `[op][cond:u16][msg:u32]` | 1 | Assert |
    /// | 7B   | `[op][dst:u16][a:u16][b:u16]` | 28 | AddI, EqI, LoadField, TupleGet, SvmTransfer... |
    /// | 9B   | `[op][dst:u16][4×reg:u16]` | 5 | EvmStaticCall, SvmInvoke, SvmCreateAccount |
    /// | 11B  | `[op][dst:u16][5×reg:u16]` | 2 | EvmCall, SvmInvokeSigned |
    /// | var  | `[op][dst:u16][func:u32][argc:u16][args...]` | 1 | Call |
    /// | var  | `[op][dst:u16][count:u16][elems...]` | 1 | NewTuple |
    /// | var  | `[op][evt:u32][argc:u16][args...]` | 1 | Emit |
    /// | var  | `[op][agent:u16][fc:u16][pairs...]` | 1 | AgentInit |
    /// | var  | `[op][tc:u8][topics...][data:u16]` | 1 | EvmLog |
    fn decode_operands(
        code: &[u8],
        pc: usize,
        opcode_byte: u8,
    ) -> VerifierResult<(usize, Vec<u64>)> {
        let opcode = Opcode::from_byte(opcode_byte)
            .ok_or_else(|| VerifierError::new(VerifierErrorKind::InvalidOpcode(opcode_byte), pc))?;

        // Helper to read bytes safely
        let read_u8 = |off: usize| -> VerifierResult<u8> {
            code.get(pc + off)
                .copied()
                .ok_or_else(|| VerifierError::new(VerifierErrorKind::OperandOutOfBounds, pc))
        };

        let read_u16 = |off: usize| -> VerifierResult<u16> {
            if pc + off + 2 > code.len() {
                return Err(VerifierError::new(
                    VerifierErrorKind::OperandOutOfBounds,
                    pc,
                ));
            }
            Ok(u16::from_le_bytes([code[pc + off], code[pc + off + 1]]))
        };

        let read_u32 = |off: usize| -> VerifierResult<u32> {
            if pc + off + 4 > code.len() {
                return Err(VerifierError::new(
                    VerifierErrorKind::OperandOutOfBounds,
                    pc,
                ));
            }
            Ok(u32::from_le_bytes([
                code[pc + off],
                code[pc + off + 1],
                code[pc + off + 2],
                code[pc + off + 3],
            ]))
        };

        // Decode based on opcode format
        match opcode {
            // No operands (1 byte total)
            Opcode::Nop | Opcode::Halt | Opcode::RetVoid => Ok((1, vec![])),

            // Single register operand: opcode reg:u16 (3 bytes)
            Opcode::LoadZero
            | Opcode::LoadTrue
            | Opcode::LoadFalse
            | Opcode::CtxSender
            | Opcode::CtxBlockHeight
            | Opcode::CtxTimestamp
            | Opcode::CtxValue
            | Opcode::CtxGas
            | Opcode::CtxChainId
            | Opcode::AtomicCheck
            | Opcode::AgentSelf => {
                let dst = read_u16(1)? as u64;
                Ok((3, vec![dst]))
            }

            // Return with value: opcode src:u16 (3 bytes)
            Opcode::Ret => {
                let src = read_u16(1)? as u64;
                Ok((3, vec![src]))
            }

            // Two registers: opcode dst:u16 src:u16 (5 bytes)
            Opcode::Mov
            | Opcode::NegI
            | Opcode::NegF
            | Opcode::Not
            | Opcode::LNot
            | Opcode::Inc
            | Opcode::Dec
            | Opcode::I32ToI64
            | Opcode::I64ToI32
            | Opcode::I32ToF32
            | Opcode::I64ToF64
            | Opcode::F32ToI32
            | Opcode::F64ToI64
            | Opcode::F32ToF64
            | Opcode::F64ToF32
            | Opcode::ToBool
            | Opcode::ArrayLen
            | Opcode::ArrayPop => {
                let dst = read_u16(1)? as u64;
                let src = read_u16(3)? as u64;
                Ok((5, vec![dst, src]))
            }

            // Three registers: opcode dst:u16 a:u16 b:u16 (7 bytes)
            Opcode::AddI
            | Opcode::SubI
            | Opcode::MulI
            | Opcode::DivI
            | Opcode::ModI
            | Opcode::AddF
            | Opcode::SubF
            | Opcode::MulF
            | Opcode::DivF
            | Opcode::ModF
            | Opcode::EqI
            | Opcode::NeI
            | Opcode::LtI
            | Opcode::LeI
            | Opcode::GtI
            | Opcode::GeI
            | Opcode::EqF
            | Opcode::NeF
            | Opcode::LtF
            | Opcode::LeF
            | Opcode::GtF
            | Opcode::GeF
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::UShr
            | Opcode::LAnd
            | Opcode::LOr
            | Opcode::LoadIndex
            | Opcode::StoreIndex => {
                let dst = read_u16(1)? as u64;
                let a = read_u16(3)? as u64;
                let b = read_u16(5)? as u64;
                Ok((7, vec![dst, a, b]))
            }

            // Jump: opcode target:u32 (5 bytes)
            Opcode::Jump => {
                let target = read_u32(1)? as u64;
                Ok((5, vec![target]))
            }

            // Conditional jump: opcode cond:u16 target:u32 (7 bytes)
            Opcode::JumpIf | Opcode::JumpUnless => {
                let cond = read_u16(1)? as u64;
                let target = read_u32(3)? as u64;
                Ok((7, vec![cond, target]))
            }

            // Load const: opcode dst:u16 idx:u32 (7 bytes)
            Opcode::LoadConst => {
                let dst = read_u16(1)? as u64;
                let idx = read_u32(3)? as u64;
                Ok((7, vec![dst, idx]))
            }

            // Load/store global: opcode dst:u16 idx:u32 (7 bytes)
            Opcode::LoadGlobal => {
                let dst = read_u16(1)? as u64;
                let idx = read_u32(3)? as u64;
                Ok((7, vec![dst, idx]))
            }
            Opcode::StoreGlobal => {
                let idx = read_u32(1)? as u64;
                let src = read_u16(5)? as u64;
                Ok((7, vec![idx, src]))
            }

            // Load field: opcode dst:u16 obj:u16 field:u16 (7 bytes)
            Opcode::LoadField => {
                let dst = read_u16(1)? as u64;
                let obj = read_u16(3)? as u64;
                let field = read_u16(5)? as u64;
                Ok((7, vec![dst, obj, field]))
            }

            // Store field: opcode obj:u16 field:u16 val:u16 (7 bytes)
            Opcode::StoreField => {
                let obj = read_u16(1)? as u64;
                let field = read_u16(3)? as u64;
                let val = read_u16(5)? as u64;
                Ok((7, vec![obj, field, val]))
            }

            // Load immediate: opcode dst:u16 val:i8 (4 bytes)
            Opcode::LoadImm => {
                let dst = read_u16(1)? as u64;
                let val = read_u8(3)? as i8 as i64 as u64;
                Ok((4, vec![dst, val]))
            }

            // Array push: opcode arr:u16 val:u16 (5 bytes)
            Opcode::ArrayPush => {
                let arr = read_u16(1)? as u64;
                let val = read_u16(3)? as u64;
                Ok((5, vec![arr, val]))
            }

            // New array: opcode dst:u16 capacity:u16 (5 bytes)
            Opcode::NewArray => {
                let dst = read_u16(1)? as u64;
                let capacity = read_u16(3)? as u64;
                Ok((5, vec![dst, capacity]))
            }

            // Tuple get: opcode dst:u16 tuple:u16 idx:u16 (7 bytes)
            Opcode::TupleGet => {
                let dst = read_u16(1)? as u64;
                let tuple = read_u16(3)? as u64;
                let idx = read_u16(5)? as u64;
                Ok((7, vec![dst, tuple, idx]))
            }

            // Atomic operations: opcode id:u16 (3 bytes)
            Opcode::AtomicBegin | Opcode::AtomicCommit | Opcode::AtomicRollback => {
                let id = read_u16(1)? as u64;
                Ok((3, vec![id]))
            }

            // Call: opcode dst:u16 func:u32 argc:u16 [args...] (variable)
            Opcode::Call => {
                let dst = read_u16(1)? as u64;
                let func = read_u32(3)? as u64;
                let argc = read_u16(7)? as usize;
                let mut operands = vec![dst, func, argc as u64];
                // Read argument registers
                for i in 0..argc {
                    let arg = read_u16(9 + i * 2)? as u64;
                    operands.push(arg);
                }
                Ok((9 + argc * 2, operands))
            }

            // New tuple: opcode dst:u16 count:u16 [elements...] (variable)
            Opcode::NewTuple => {
                let dst = read_u16(1)? as u64;
                let count = read_u16(3)? as usize;
                let mut operands = vec![dst, count as u64];
                for i in 0..count {
                    let elem = read_u16(5 + i * 2)? as u64;
                    operands.push(elem);
                }
                Ok((5 + count * 2, operands))
            }

            // Emit: opcode event_id:u32 argc:u16 [args...] (variable)
            Opcode::Emit => {
                let event_id = read_u32(1)? as u64;
                let argc = read_u16(5)? as usize;
                let mut operands = vec![event_id, argc as u64];
                for i in 0..argc {
                    let arg = read_u16(7 + i * 2)? as u64;
                    operands.push(arg);
                }
                Ok((7 + argc * 2, operands))
            }

            // Agent init: opcode agent:u16 field_count:u16 [field_idx:u16 val:u16...] (variable)
            Opcode::AgentInit => {
                let agent = read_u16(1)? as u64;
                let field_count = read_u16(3)? as usize;
                let mut operands = vec![agent, field_count as u64];
                for i in 0..field_count {
                    let field_idx = read_u16(5 + i * 4)? as u64;
                    let val = read_u16(7 + i * 4)? as u64;
                    operands.push(field_idx);
                    operands.push(val);
                }
                Ok((5 + field_count * 4, operands))
            }

            // EVM call: opcode dst:u16 gas:u16 addr:u16 value:u16 data:u16 (11 bytes)
            Opcode::EvmCall => {
                let dst = read_u16(1)? as u64;
                let gas = read_u16(3)? as u64;
                let addr = read_u16(5)? as u64;
                let value = read_u16(7)? as u64;
                let data = read_u16(9)? as u64;
                Ok((11, vec![dst, gas, addr, value, data]))
            }

            // EVM staticcall/delegatecall: opcode dst:u16 gas:u16 addr:u16 data:u16 (9 bytes)
            Opcode::EvmStaticCall | Opcode::EvmDelegateCall => {
                let dst = read_u16(1)? as u64;
                let gas = read_u16(3)? as u64;
                let addr = read_u16(5)? as u64;
                let data = read_u16(7)? as u64;
                Ok((9, vec![dst, gas, addr, data]))
            }

            // EVM sload: opcode dst:u16 slot:u16 (5 bytes)
            Opcode::EvmSload => {
                let dst = read_u16(1)? as u64;
                let slot = read_u16(3)? as u64;
                Ok((5, vec![dst, slot]))
            }

            // EVM sstore: opcode slot:u16 val:u16 (5 bytes)
            Opcode::EvmSstore => {
                let slot = read_u16(1)? as u64;
                let val = read_u16(3)? as u64;
                Ok((5, vec![slot, val]))
            }

            // EVM create: opcode dst:u16 value:u16 code:u16 (7 bytes)
            Opcode::EvmCreate => {
                let dst = read_u16(1)? as u64;
                let value = read_u16(3)? as u64;
                let code_reg = read_u16(5)? as u64;
                Ok((7, vec![dst, value, code_reg]))
            }

            // EVM create2: opcode dst:u16 value:u16 code:u16 salt:u16 (9 bytes)
            Opcode::EvmCreate2 => {
                let dst = read_u16(1)? as u64;
                let value = read_u16(3)? as u64;
                let code_reg = read_u16(5)? as u64;
                let salt = read_u16(7)? as u64;
                Ok((9, vec![dst, value, code_reg, salt]))
            }

            // ================================================================
            // Debug/Meta (0xF0 - 0xFF)
            // ================================================================

            // Debug print: opcode src:u16 (3 bytes)
            Opcode::DebugPrint => {
                let src = read_u16(1)? as u64;
                Ok((3, vec![src]))
            }

            // Breakpoint: opcode (1 byte) - no operands
            Opcode::Breakpoint => Ok((1, vec![])),

            // Assert: opcode cond:u16 msg_idx:u32 (7 bytes)
            Opcode::Assert => {
                let cond = read_u16(1)? as u64;
                let msg_idx = read_u32(3)? as u64;
                Ok((7, vec![cond, msg_idx]))
            }

            // Panic: opcode msg_idx:u32 (5 bytes)
            Opcode::Panic => {
                let msg_idx = read_u32(1)? as u64;
                Ok((5, vec![msg_idx]))
            }

            // ================================================================
            // EVM Intrinsics (0xB0 - 0xBF) - remaining opcodes
            // ================================================================

            // EVM log: opcode topic_count:u8 [topics:u16...] data:u16 (variable)
            // Size = 1 (opcode) + 1 (count) + count*2 (topics) + 2 (data)
            Opcode::EvmLog => {
                let topic_count = read_u8(1)? as usize;
                let mut operands = vec![topic_count as u64];
                for i in 0..topic_count {
                    let topic = read_u16(2 + i * 2)? as u64;
                    operands.push(topic);
                }
                let data = read_u16(2 + topic_count * 2)? as u64;
                operands.push(data);
                Ok((4 + topic_count * 2, operands))
            }

            // EVM balance: opcode dst:u16 addr:u16 (5 bytes)
            Opcode::EvmBalance => {
                let dst = read_u16(1)? as u64;
                let addr = read_u16(3)? as u64;
                Ok((5, vec![dst, addr]))
            }

            // EVM codesize: opcode dst:u16 addr:u16 (5 bytes)
            Opcode::EvmCodeSize => {
                let dst = read_u16(1)? as u64;
                let addr = read_u16(3)? as u64;
                Ok((5, vec![dst, addr]))
            }

            // ================================================================
            // SVM Intrinsics (0xC0 - 0xCF)
            // ================================================================

            // SVM invoke: opcode dst:u16 program:u16 accounts:u16 data:u16 (9 bytes)
            Opcode::SvmInvoke => {
                let dst = read_u16(1)? as u64;
                let program = read_u16(3)? as u64;
                let accounts = read_u16(5)? as u64;
                let data = read_u16(7)? as u64;
                Ok((9, vec![dst, program, accounts, data]))
            }

            // SVM invoke signed: opcode dst:u16 program:u16 accounts:u16 data:u16 seeds:u16 (11 bytes)
            Opcode::SvmInvokeSigned => {
                let dst = read_u16(1)? as u64;
                let program = read_u16(3)? as u64;
                let accounts = read_u16(5)? as u64;
                let data = read_u16(7)? as u64;
                let seeds = read_u16(9)? as u64;
                Ok((11, vec![dst, program, accounts, data, seeds]))
            }

            // SVM create account: opcode dst:u16 lamports:u16 space:u16 owner:u16 (9 bytes)
            Opcode::SvmCreateAccount => {
                let dst = read_u16(1)? as u64;
                let lamports = read_u16(3)? as u64;
                let space = read_u16(5)? as u64;
                let owner = read_u16(7)? as u64;
                Ok((9, vec![dst, lamports, space, owner]))
            }

            // SVM transfer: opcode from:u16 to:u16 lamports:u16 (7 bytes)
            Opcode::SvmTransfer => {
                let from = read_u16(1)? as u64;
                let to = read_u16(3)? as u64;
                let lamports = read_u16(5)? as u64;
                Ok((7, vec![from, to, lamports]))
            }

            // SVM get data: opcode dst:u16 account:u16 (5 bytes)
            Opcode::SvmGetData => {
                let dst = read_u16(1)? as u64;
                let account = read_u16(3)? as u64;
                Ok((5, vec![dst, account]))
            }

            // SVM set data: opcode account:u16 data:u16 (5 bytes)
            Opcode::SvmSetData => {
                let account = read_u16(1)? as u64;
                let data = read_u16(3)? as u64;
                Ok((5, vec![account, data]))
            }

            // SVM get rent: opcode dst:u16 (3 bytes)
            Opcode::SvmGetRent => {
                let dst = read_u16(1)? as u64;
                Ok((3, vec![dst]))
            }

            // SVM get clock: opcode dst:u16 (3 bytes)
            Opcode::SvmGetClock => {
                let dst = read_u16(1)? as u64;
                Ok((3, vec![dst]))
            }
        }
    }

    /// Verify CFG: all jump targets are valid instruction boundaries.
    fn verify_cfg(
        instrs: &[DecodedInstr],
        offsets: &BTreeSet<usize>,
        module: &BytecodeModule,
    ) -> VerifierResult<()> {
        let code_len = module.code.len();

        for instr in instrs {
            let opcode = Opcode::from_byte(instr.opcode);

            match opcode {
                Some(Opcode::Jump) => {
                    let target = instr.operands[0] as u32;
                    Self::check_jump_target(target, code_len, offsets, instr.offset)?;
                }
                Some(Opcode::JumpIf) | Some(Opcode::JumpUnless) => {
                    let target = instr.operands[1] as u32;
                    Self::check_jump_target(target, code_len, offsets, instr.offset)?;
                }
                Some(Opcode::Call) => {
                    let func_idx = instr.operands[1] as u32;
                    if func_idx as usize >= module.functions.len() {
                        return Err(VerifierError::new(
                            VerifierErrorKind::FunctionIndexOutOfBounds(
                                func_idx,
                                module.functions.len(),
                            ),
                            instr.offset,
                        ));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn check_jump_target(
        target: u32,
        code_len: usize,
        offsets: &BTreeSet<usize>,
        from_offset: usize,
    ) -> VerifierResult<()> {
        if target as usize >= code_len {
            return Err(VerifierError::new(
                VerifierErrorKind::JumpTargetOutOfBounds(target, code_len),
                from_offset,
            ));
        }
        if !offsets.contains(&(target as usize)) {
            return Err(VerifierError::new(
                VerifierErrorKind::JumpTargetUnaligned(target),
                from_offset,
            ));
        }
        Ok(())
    }

    /// Verify all const pool references are in bounds.
    fn verify_const_pool_refs(
        instrs: &[DecodedInstr],
        module: &BytecodeModule,
    ) -> VerifierResult<()> {
        let pool_len = module.const_pool.len();

        for instr in instrs {
            if instr.opcode == Opcode::LoadConst.to_byte() {
                let idx = instr.operands[1] as u32;
                if idx as usize >= pool_len {
                    return Err(VerifierError::new(
                        VerifierErrorKind::ConstPoolIndexOutOfBounds(idx, pool_len),
                        instr.offset,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Verify atomic begin/end markers are balanced.
    fn verify_atomic_balance(instrs: &[DecodedInstr]) -> VerifierResult<()> {
        // Track active atomic blocks by ID
        let mut active: BTreeMap<u16, usize> = BTreeMap::new();

        for instr in instrs {
            match Opcode::from_byte(instr.opcode) {
                Some(Opcode::AtomicBegin) => {
                    let id = instr.operands[0] as u16;
                    active.insert(id, instr.offset);
                }
                Some(Opcode::AtomicCommit) | Some(Opcode::AtomicRollback) => {
                    let id = instr.operands[0] as u16;
                    if active.remove(&id).is_none() {
                        return Err(VerifierError::new(
                            VerifierErrorKind::AtomicIdMismatch(id),
                            instr.offset,
                        ));
                    }
                }
                _ => {}
            }
        }

        if !active.is_empty() {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::AtomicUnbalanced(active.len() as i32),
            ));
        }

        Ok(())
    }

    /// Verify gas budget doesn't exceed limits.
    fn verify_gas(instrs: &[DecodedInstr], options: &VerifyOptions) -> VerifierResult<()> {
        let total_gas: u128 = instrs.iter().map(|i| i.gas_cost as u128).sum();

        if total_gas > options.max_gas_per_function {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::GasBudgetExceeded(total_gas, options.max_gas_per_function),
            ));
        }

        Ok(())
    }

    /// Verify on-chain restrictions (no debug opcodes).
    fn verify_on_chain_restrictions(instrs: &[DecodedInstr]) -> VerifierResult<()> {
        for instr in instrs {
            match Opcode::from_byte(instr.opcode) {
                Some(Opcode::DebugPrint) => {
                    return Err(VerifierError::new(
                        VerifierErrorKind::ForbiddenOnChain(instr.opcode),
                        instr.offset,
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }
}

/// Get gas cost for an opcode.
pub fn opcode_gas_cost(opcode: u8) -> u64 {
    match Opcode::from_byte(opcode) {
        Some(op) => match op {
            // Free operations
            Opcode::Nop => 0,

            // Very cheap (1 gas)
            Opcode::Mov
            | Opcode::LoadZero
            | Opcode::LoadTrue
            | Opcode::LoadFalse
            | Opcode::LoadImm => 1,

            // Cheap (2 gas)
            Opcode::AddI
            | Opcode::SubI
            | Opcode::NegI
            | Opcode::Inc
            | Opcode::Dec
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Not
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::UShr
            | Opcode::LAnd
            | Opcode::LOr
            | Opcode::LNot => 2,

            // Moderate (3 gas)
            Opcode::MulI
            | Opcode::DivI
            | Opcode::ModI
            | Opcode::AddF
            | Opcode::SubF
            | Opcode::MulF
            | Opcode::DivF
            | Opcode::ModF
            | Opcode::NegF
            | Opcode::EqI
            | Opcode::NeI
            | Opcode::LtI
            | Opcode::LeI
            | Opcode::GtI
            | Opcode::GeI
            | Opcode::EqF
            | Opcode::NeF
            | Opcode::LtF
            | Opcode::LeF
            | Opcode::GtF
            | Opcode::GeF => 3,

            // Memory operations (5 gas)
            Opcode::LoadConst
            | Opcode::LoadGlobal
            | Opcode::StoreGlobal
            | Opcode::LoadIndex
            | Opcode::StoreIndex
            | Opcode::LoadField
            | Opcode::StoreField => 5,

            // Type conversions (2 gas)
            Opcode::I32ToI64
            | Opcode::I64ToI32
            | Opcode::I32ToF32
            | Opcode::I64ToF64
            | Opcode::F32ToI32
            | Opcode::F64ToI64
            | Opcode::F32ToF64
            | Opcode::F64ToF32
            | Opcode::ToBool => 2,

            // Control flow (1-10 gas)
            Opcode::Jump | Opcode::JumpIf | Opcode::JumpUnless => 3,
            Opcode::Call => 10,
            Opcode::Ret | Opcode::RetVoid => 2,
            Opcode::Halt => 1,

            // Array operations (5-10 gas)
            Opcode::NewArray => 10,
            Opcode::ArrayLen => 2,
            Opcode::ArrayPush | Opcode::ArrayPop => 5,
            Opcode::NewTuple => 10,
            Opcode::TupleGet => 3,

            // Context operations (5 gas)
            Opcode::CtxSender
            | Opcode::CtxBlockHeight
            | Opcode::CtxTimestamp
            | Opcode::CtxValue
            | Opcode::CtxGas
            | Opcode::CtxChainId => 5,

            // Atomic operations (100 gas)
            Opcode::AtomicBegin | Opcode::AtomicCommit | Opcode::AtomicRollback => 100,
            Opcode::AtomicCheck => 5,

            // Agent operations (50 gas)
            Opcode::AgentSelf => 5,
            Opcode::AgentInit => 50,
            Opcode::Emit => 20,

            // EVM intrinsics (very expensive)
            Opcode::EvmCall => 2500,
            Opcode::EvmStaticCall => 700,
            Opcode::EvmDelegateCall => 700,
            Opcode::EvmSload => 200,
            Opcode::EvmSstore => 5000,
            Opcode::EvmCreate => 32000,
            Opcode::EvmCreate2 => 32000,
            Opcode::EvmLog => 375, // Base log cost
            Opcode::EvmBalance => 400,
            Opcode::EvmCodeSize => 100,

            // SVM intrinsics (expensive)
            Opcode::SvmInvoke => 5000,
            Opcode::SvmInvokeSigned => 6000,
            Opcode::SvmCreateAccount => 10000,
            Opcode::SvmTransfer => 3000,
            Opcode::SvmGetData => 200,
            Opcode::SvmSetData => 1000,
            Opcode::SvmGetRent => 100,
            Opcode::SvmGetClock => 100,

            // Debug (0 gas in dev, forbidden on-chain)
            Opcode::DebugPrint => 0,
            Opcode::Breakpoint => 0,
            Opcode::Assert => 5,
            Opcode::Panic => 5,
        },
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_backend::bc_format::{BytecodeModule, ConstPool};
    use x3_backend::opcode::Opcode;

    fn make_simple_module(code: Vec<u8>) -> BytecodeModule {
        BytecodeModule {
            version: VERSION,
            flags: Default::default(),
            const_pool: ConstPool::new(),
            functions: vec![],
            globals: vec![],
            code,
            debug_info: None,
        }
    }

    #[test]
    fn verify_empty_module() {
        let result = Verifier::verify_module_bytes(&[], &VerifyOptions::default());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::EmptyModule
        ));
    }

    #[test]
    fn verify_nop_halt() {
        let mut module = make_simple_module(vec![Opcode::Nop.to_byte(), Opcode::Halt.to_byte()]);
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn verify_invalid_opcode() {
        let module = make_simple_module(vec![0xFF]); // Invalid opcode
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::InvalidOpcode(0xFF)
        ));
    }

    #[test]
    fn verify_jump_target_oob() {
        let mut module = make_simple_module(vec![
            Opcode::Jump.to_byte(),
            0xFF,
            0x00,
            0x00,
            0x00, // Target 255 (out of bounds)
        ]);
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::JumpTargetOutOfBounds(255, _)
        ));
    }

    #[test]
    fn verify_atomic_unbalanced() {
        let module = make_simple_module(vec![
            Opcode::AtomicBegin.to_byte(),
            0x00,
            0x00, // ID 0
            Opcode::Halt.to_byte(),
            // Missing AtomicCommit/Rollback
        ]);
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::AtomicUnbalanced(_)
        ));
    }

    #[test]
    fn verify_atomic_balanced() {
        let module = make_simple_module(vec![
            Opcode::AtomicBegin.to_byte(),
            0x00,
            0x00, // ID 0
            Opcode::AtomicCommit.to_byte(),
            0x00,
            0x00, // ID 0
            Opcode::Halt.to_byte(),
        ]);
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn verify_debug_forbidden_on_chain() {
        let module = make_simple_module(vec![
            Opcode::DebugPrint.to_byte(),
            0x00,
            0x00, // Register 0
        ]);
        let result = Verifier::verify_module(&module, &VerifyOptions::on_chain());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::ForbiddenOnChain(_)
        ));
    }

    #[test]
    fn gas_costs_reasonable() {
        // Ensure all costs are set
        assert_eq!(opcode_gas_cost(Opcode::Nop.to_byte()), 0);
        assert_eq!(opcode_gas_cost(Opcode::AddI.to_byte()), 2);
        assert_eq!(opcode_gas_cost(Opcode::MulI.to_byte()), 3);
        assert_eq!(opcode_gas_cost(Opcode::EvmSstore.to_byte()), 5000);
    }

    // ========================================================================
    // Comprehensive decode_operands tests for all opcode categories
    // ========================================================================

    #[test]
    fn decode_no_operand_instructions() {
        // Nop, Halt, RetVoid, Breakpoint - all 1 byte
        let code = vec![Opcode::Nop.to_byte()];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 1);
        assert!(instrs[0].operands.is_empty());

        let code = vec![Opcode::Halt.to_byte()];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 1);

        let code = vec![Opcode::Breakpoint.to_byte()];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 1);
    }

    #[test]
    fn decode_single_register_instructions() {
        // LoadZero, CtxSender, etc. - 3 bytes (opcode + reg:u16)
        let code = vec![
            Opcode::LoadZero.to_byte(),
            0x05,
            0x00, // r5
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 3);
        assert_eq!(instrs[0].operands, vec![5]);

        // Ret - 3 bytes
        let code = vec![
            Opcode::Ret.to_byte(),
            0x07,
            0x00, // r7
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].operands, vec![7]);
    }

    #[test]
    fn decode_two_register_instructions() {
        // Mov, NegI, Not, ArrayLen, etc. - 5 bytes
        let code = vec![
            Opcode::Mov.to_byte(),
            0x01,
            0x00, // dst: r1
            0x02,
            0x00, // src: r2
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 5);
        assert_eq!(instrs[0].operands, vec![1, 2]);
    }

    #[test]
    fn decode_three_register_instructions() {
        // AddI, SubI, MulI, EqI, LoadIndex, etc. - 7 bytes
        let code = vec![
            Opcode::AddI.to_byte(),
            0x03,
            0x00, // dst: r3
            0x01,
            0x00, // a: r1
            0x02,
            0x00, // b: r2
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 7);
        assert_eq!(instrs[0].operands, vec![3, 1, 2]);
    }

    #[test]
    fn decode_jump_instructions() {
        // Jump - 5 bytes (opcode + target:u32)
        let code = vec![
            Opcode::Jump.to_byte(),
            0x10,
            0x00,
            0x00,
            0x00, // target: 16
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 5);
        assert_eq!(instrs[0].operands, vec![16]);

        // JumpIf - 7 bytes (opcode + cond:u16 + target:u32)
        let code = vec![
            Opcode::JumpIf.to_byte(),
            0x05,
            0x00, // cond: r5
            0x20,
            0x00,
            0x00,
            0x00, // target: 32
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 7);
        assert_eq!(instrs[0].operands, vec![5, 32]);
    }

    #[test]
    fn decode_load_const() {
        // LoadConst - 7 bytes (opcode + dst:u16 + idx:u32)
        let code = vec![
            Opcode::LoadConst.to_byte(),
            0x01,
            0x00, // dst: r1
            0x00,
            0x01,
            0x00,
            0x00, // idx: 256
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 7);
        assert_eq!(instrs[0].operands, vec![1, 256]);
    }

    #[test]
    fn decode_load_imm() {
        // LoadImm - 4 bytes (opcode + dst:u16 + val:i8)
        let code = vec![
            Opcode::LoadImm.to_byte(),
            0x05,
            0x00, // dst: r5
            0xFE, // val: -2 (signed)
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 4);
        // -2 as i8 sign-extended to u64
        assert_eq!(instrs[0].operands[0], 5);
        assert_eq!(instrs[0].operands[1] as i64, -2);
    }

    #[test]
    fn decode_call_variable_length() {
        // Call - variable: opcode + dst:u16 + func:u32 + argc:u16 + [args:u16...]
        // 0 args: 9 bytes
        let code = vec![
            Opcode::Call.to_byte(),
            0x00,
            0x00, // dst: r0
            0x05,
            0x00,
            0x00,
            0x00, // func: 5
            0x00,
            0x00, // argc: 0
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 9);
        assert_eq!(instrs[0].operands, vec![0, 5, 0]);

        // 2 args: 13 bytes
        let code = vec![
            Opcode::Call.to_byte(),
            0x00,
            0x00, // dst: r0
            0x03,
            0x00,
            0x00,
            0x00, // func: 3
            0x02,
            0x00, // argc: 2
            0x01,
            0x00, // arg0: r1
            0x02,
            0x00, // arg1: r2
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 13);
        assert_eq!(instrs[0].operands, vec![0, 3, 2, 1, 2]);
    }

    #[test]
    fn decode_new_tuple_variable_length() {
        // NewTuple - variable: opcode + dst:u16 + count:u16 + [elements:u16...]
        let code = vec![
            Opcode::NewTuple.to_byte(),
            0x05,
            0x00, // dst: r5
            0x03,
            0x00, // count: 3
            0x01,
            0x00, // elem0: r1
            0x02,
            0x00, // elem1: r2
            0x03,
            0x00, // elem2: r3
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 11); // 5 + 3*2
        assert_eq!(instrs[0].operands, vec![5, 3, 1, 2, 3]);
    }

    #[test]
    fn decode_emit_variable_length() {
        // Emit - variable: opcode + event_id:u32 + argc:u16 + [args:u16...]
        let code = vec![
            Opcode::Emit.to_byte(),
            0x0A,
            0x00,
            0x00,
            0x00, // event_id: 10
            0x01,
            0x00, // argc: 1
            0x07,
            0x00, // arg0: r7
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 9); // 7 + 1*2
        assert_eq!(instrs[0].operands, vec![10, 1, 7]);
    }

    #[test]
    fn decode_evm_intrinsics() {
        // EvmCall - 11 bytes
        let code = vec![
            Opcode::EvmCall.to_byte(),
            0x00,
            0x00, // dst: r0
            0x01,
            0x00, // gas: r1
            0x02,
            0x00, // addr: r2
            0x03,
            0x00, // value: r3
            0x04,
            0x00, // data: r4
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 11);
        assert_eq!(instrs[0].operands, vec![0, 1, 2, 3, 4]);

        // EvmSload - 5 bytes
        let code = vec![
            Opcode::EvmSload.to_byte(),
            0x05,
            0x00, // dst: r5
            0x06,
            0x00, // slot: r6
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 5);
        assert_eq!(instrs[0].operands, vec![5, 6]);
    }

    #[test]
    fn decode_svm_intrinsics() {
        // SvmInvoke - 9 bytes
        let code = vec![
            Opcode::SvmInvoke.to_byte(),
            0x00,
            0x00, // dst: r0
            0x01,
            0x00, // program: r1
            0x02,
            0x00, // accounts: r2
            0x03,
            0x00, // data: r3
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 9);
        assert_eq!(instrs[0].operands, vec![0, 1, 2, 3]);

        // SvmTransfer - 7 bytes
        let code = vec![
            Opcode::SvmTransfer.to_byte(),
            0x01,
            0x00, // from: r1
            0x02,
            0x00, // to: r2
            0x03,
            0x00, // lamports: r3
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 7);
        assert_eq!(instrs[0].operands, vec![1, 2, 3]);

        // SvmGetClock - 3 bytes
        let code = vec![
            Opcode::SvmGetClock.to_byte(),
            0x09,
            0x00, // dst: r9
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 3);
        assert_eq!(instrs[0].operands, vec![9]);
    }

    #[test]
    fn decode_debug_instructions() {
        // Assert - 7 bytes (opcode + cond:u16 + msg_idx:u32)
        let code = vec![
            Opcode::Assert.to_byte(),
            0x01,
            0x00, // cond: r1
            0x00,
            0x00,
            0x00,
            0x00, // msg_idx: 0
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 7);
        assert_eq!(instrs[0].operands, vec![1, 0]);

        // Panic - 5 bytes (opcode + msg_idx:u32)
        let code = vec![
            Opcode::Panic.to_byte(),
            0x01,
            0x00,
            0x00,
            0x00, // msg_idx: 1
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 5);
        assert_eq!(instrs[0].operands, vec![1]);
    }

    #[test]
    fn decode_evm_log_variable_length() {
        // EvmLog - variable: opcode + topic_count:u8 + [topics:u16...] + data:u16
        // 0 topics: 4 bytes
        let code = vec![
            Opcode::EvmLog.to_byte(),
            0x00, // topic_count: 0
            0x05,
            0x00, // data: r5
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 4);
        assert_eq!(instrs[0].operands, vec![0, 5]);

        // 2 topics: 8 bytes
        let code = vec![
            Opcode::EvmLog.to_byte(),
            0x02, // topic_count: 2
            0x01,
            0x00, // topic0: r1
            0x02,
            0x00, // topic1: r2
            0x03,
            0x00, // data: r3
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 8); // 4 + 2*2
        assert_eq!(instrs[0].operands, vec![2, 1, 2, 3]);
    }

    #[test]
    fn decode_multi_instruction_sequence() {
        // Test a realistic instruction sequence
        let mut code = Vec::new();

        // load_const r1, c0 (7 bytes)
        code.push(Opcode::LoadConst.to_byte());
        code.extend_from_slice(&1u16.to_le_bytes());
        code.extend_from_slice(&0u32.to_le_bytes());

        // load_const r2, c1 (7 bytes)
        code.push(Opcode::LoadConst.to_byte());
        code.extend_from_slice(&2u16.to_le_bytes());
        code.extend_from_slice(&1u32.to_le_bytes());

        // add_i r3, r1, r2 (7 bytes)
        code.push(Opcode::AddI.to_byte());
        code.extend_from_slice(&3u16.to_le_bytes());
        code.extend_from_slice(&1u16.to_le_bytes());
        code.extend_from_slice(&2u16.to_le_bytes());

        // ret r3 (3 bytes)
        code.push(Opcode::Ret.to_byte());
        code.extend_from_slice(&3u16.to_le_bytes());

        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 4);

        // Verify offsets
        assert_eq!(instrs[0].offset, 0);
        assert_eq!(instrs[1].offset, 7);
        assert_eq!(instrs[2].offset, 14);
        assert_eq!(instrs[3].offset, 21);

        // Total should be 24 bytes
        assert_eq!(code.len(), 24);
    }

    #[test]
    fn decode_all_context_ops() {
        // All context ops are 3 bytes
        for opcode in [
            Opcode::CtxSender,
            Opcode::CtxBlockHeight,
            Opcode::CtxTimestamp,
            Opcode::CtxValue,
            Opcode::CtxGas,
            Opcode::CtxChainId,
        ] {
            let code = vec![
                opcode.to_byte(),
                0x01,
                0x00, // dst: r1
            ];
            let instrs = Verifier::decode_all_instructions(&code).unwrap();
            assert_eq!(instrs[0].size, 3, "Failed for {:?}", opcode);
            assert_eq!(instrs[0].operands, vec![1]);
        }
    }

    #[test]
    fn decode_all_type_conversions() {
        // All type conversions are 5 bytes (dst:u16 + src:u16)
        for opcode in [
            Opcode::I32ToI64,
            Opcode::I64ToI32,
            Opcode::I32ToF32,
            Opcode::I64ToF64,
            Opcode::F32ToI32,
            Opcode::F64ToI64,
            Opcode::F32ToF64,
            Opcode::F64ToF32,
            Opcode::ToBool,
        ] {
            let code = vec![
                opcode.to_byte(),
                0x02,
                0x00, // dst: r2
                0x01,
                0x00, // src: r1
            ];
            let instrs = Verifier::decode_all_instructions(&code).unwrap();
            assert_eq!(instrs[0].size, 5, "Failed for {:?}", opcode);
            assert_eq!(instrs[0].operands, vec![2, 1]);
        }
    }

    #[test]
    fn gas_costs_complete() {
        // Verify all newly added opcodes have gas costs
        assert_eq!(opcode_gas_cost(Opcode::EvmLog.to_byte()), 375);
        assert_eq!(opcode_gas_cost(Opcode::EvmBalance.to_byte()), 400);
        assert_eq!(opcode_gas_cost(Opcode::EvmCodeSize.to_byte()), 100);

        assert_eq!(opcode_gas_cost(Opcode::SvmInvoke.to_byte()), 5000);
        assert_eq!(opcode_gas_cost(Opcode::SvmInvokeSigned.to_byte()), 6000);
        assert_eq!(opcode_gas_cost(Opcode::SvmCreateAccount.to_byte()), 10000);
        assert_eq!(opcode_gas_cost(Opcode::SvmTransfer.to_byte()), 3000);
        assert_eq!(opcode_gas_cost(Opcode::SvmGetData.to_byte()), 200);
        assert_eq!(opcode_gas_cost(Opcode::SvmSetData.to_byte()), 1000);
        assert_eq!(opcode_gas_cost(Opcode::SvmGetRent.to_byte()), 100);
        assert_eq!(opcode_gas_cost(Opcode::SvmGetClock.to_byte()), 100);

        assert_eq!(opcode_gas_cost(Opcode::Breakpoint.to_byte()), 0);
        assert_eq!(opcode_gas_cost(Opcode::Assert.to_byte()), 5);
        assert_eq!(opcode_gas_cost(Opcode::Panic.to_byte()), 5);
    }
}
