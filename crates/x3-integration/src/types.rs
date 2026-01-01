//! Type definitions for X3 integration with Atlas Kernel

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;

/// Execution receipt from X3 VM
#[derive(Clone, Debug, Default, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub struct X3ExecutionReceipt {
    /// Whether execution succeeded
    pub success: bool,
    /// Gas used during execution
    pub gas_used: u64,
    /// Return data from execution
    pub return_data: Vec<u8>,
    /// Logs emitted during execution
    pub logs: Vec<X3ExecutionLog>,
    /// State changes produced
    pub state_changes: Vec<X3StateChange>,
    /// Function that was called
    pub function_index: u32,
    /// Number of instructions executed
    pub instructions_executed: u64,
}

/// Log entry from X3 execution
#[derive(Clone, Debug, Default, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub struct X3ExecutionLog {
    /// Log topic (event identifier)
    pub topic: H256,
    /// Log data
    pub data: Vec<u8>,
}

/// State change from X3 execution
#[derive(Clone, Debug, Default, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub struct X3StateChange {
    /// Storage key (32 bytes)
    pub key: H256,
    /// Old value (if any)
    pub old_value: Option<Vec<u8>>,
    /// New value
    pub new_value: Vec<u8>,
}

/// X3 module metadata for on-chain storage
#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub struct X3ModuleInfo {
    /// Module hash (blake2_256 of bytecode)
    pub code_hash: H256,
    /// Number of functions
    pub function_count: u32,
    /// Estimated max gas for each function
    pub gas_estimates: Vec<u64>,
    /// Whether module has unbounded loops
    pub has_unbounded_loops: bool,
    /// Module version
    pub version: u32,
    /// Author/deployer account (32 bytes)
    pub author: Vec<u8>,
    /// Deployment timestamp
    pub deployed_at: u64,
}

/// Arguments for X3 function call
#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq)]
pub enum X3Value {
    /// 64-bit signed integer
    I64(i64),
    /// 64-bit floating point (as bits for determinism)
    F64Bits(u64),
    /// Boolean
    Bool(bool),
    /// Byte array
    Bytes(Vec<u8>),
    /// Address (20 or 32 bytes)
    Address(Vec<u8>),
    /// Unit/void
    Unit,
}

impl X3Value {
    /// Create from i64
    pub fn from_i64(v: i64) -> Self {
        Self::I64(v)
    }

    /// Create from f64
    pub fn from_f64(v: f64) -> Self {
        Self::F64Bits(v.to_bits())
    }

    /// Create from bool
    pub fn from_bool(v: bool) -> Self {
        Self::Bool(v)
    }

    /// Create from bytes
    pub fn from_bytes(v: Vec<u8>) -> Self {
        Self::Bytes(v)
    }

    /// Get as i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64Bits(bits) => Some(f64::from_bits(*bits)),
            _ => None,
        }
    }

    /// Get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            Self::I64(v) => Some(*v != 0),
            _ => None,
        }
    }
}

/// Gas cost configuration for X3 operations
#[derive(Clone, Debug, Encode, Decode, TypeInfo)]
pub struct X3GasConfig {
    /// Base cost per instruction
    pub instruction_base: u64,
    /// Cost per byte of memory access
    pub memory_per_byte: u64,
    /// Cost for hostcall invocation
    pub hostcall_base: u64,
    /// Cost per byte of storage read
    pub storage_read_per_byte: u64,
    /// Cost per byte of storage write
    pub storage_write_per_byte: u64,
    /// Cost for arithmetic operations
    pub arithmetic_cost: u64,
    /// Cost for comparison operations
    pub comparison_cost: u64,
    /// Cost for control flow (jump/branch)
    pub control_flow_cost: u64,
    /// Cost for function call
    pub call_cost: u64,
}

impl Default for X3GasConfig {
    fn default() -> Self {
        Self {
            instruction_base: 1,
            memory_per_byte: 3,
            hostcall_base: 100,
            storage_read_per_byte: 50,
            storage_write_per_byte: 200,
            arithmetic_cost: 1,
            comparison_cost: 1,
            control_flow_cost: 2,
            call_cost: 10,
        }
    }
}
