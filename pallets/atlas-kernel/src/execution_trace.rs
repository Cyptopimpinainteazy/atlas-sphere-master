//! Execution Trace Module
//!
//! Provides step-by-step execution tracing for both EVM and SVM transactions.
//! Enables debugging and verification of transaction execution paths.

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Codec, Decode, Encode};
use sp_core::H256;
use sp_std::vec::Vec;

/// Represents a single step in the execution trace
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ExecutionStep {
    /// Sequential step counter
    pub step_index: u64,
    /// Program counter (instruction pointer)
    pub pc: u64,
    /// Current operation code
    pub opcode: u8,
    /// Operand stack state (for EVM: stack items; for SVM: stack frames)
    pub stack: Vec<Vec<u8>>,
    /// Memory state at this step (first 256 bytes typically)
    pub memory: Vec<u8>,
    /// Current storage read (slot → value)
    pub storage_access: Option<(H256, H256)>,
    /// Gas/compute units consumed so far
    pub gas_used: u64,
    /// Whether this step caused a state change
    pub state_modified: bool,
}

/// Complete execution trace for a single transaction
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ExecutionTrace {
    /// Transaction ID or hash
    pub tx_id: H256,
    /// VM type: 0 = EVM, 1 = SVM
    pub vm_type: u8,
    /// Total steps executed
    pub step_count: u64,
    /// All execution steps
    pub steps: Vec<ExecutionStep>,
    /// Entry point (function selector for EVM, program ID for SVM)
    pub entry_point: Vec<u8>,
    /// Return value from execution
    pub return_value: Vec<u8>,
    /// Total gas/compute consumed
    pub total_gas_used: u64,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error_msg: Vec<u8>,
}

/// Detailed trace frame for call/return sequences
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TraceFrame {
    /// Frame ID
    pub frame_id: u32,
    /// Parent frame ID (None for root)
    pub parent_frame: Option<u32>,
    /// Function/program being executed
    pub function: Vec<u8>,
    /// Entry parameters
    pub params: Vec<Vec<u8>>,
    /// Return value
    pub return_value: Vec<u8>,
    /// Gas allocated to this frame
    pub gas_allocated: u64,
    /// Gas actually consumed
    pub gas_consumed: u64,
    /// Depth in call stack
    pub depth: u32,
}

/// Execution trace with structured call frames
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CallTraceFrame {
    /// Unique frame ID
    pub frame_id: u32,
    /// Call type: 0 = call, 1 = delegatecall, 2 = staticcall, 3 = internal
    pub call_type: u8,
    /// Target contract/program
    pub target: Vec<u8>,
    /// Input data
    pub input: Vec<u8>,
    /// Output data
    pub output: Vec<u8>,
    /// Execution result
    pub success: bool,
    /// Error if failed
    pub error: Vec<u8>,
    /// Gas limit for this call
    pub gas_limit: u64,
    /// Gas used
    pub gas_used: u64,
    /// Value transferred (if applicable)
    pub value: Vec<u8>,
    /// Child frames
    pub children: Vec<u32>,
}

/// Storage access record for transaction
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StorageAccess {
    /// Read or write: 0 = read, 1 = write
    pub access_type: u8,
    /// Storage slot key
    pub key: H256,
    /// Value read/written
    pub value: H256,
    /// Step index where access occurred
    pub step_index: u64,
}

/// Memory trace for detecting memory expansion
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct MemoryTrace {
    /// Byte offset accessed
    pub offset: u32,
    /// Access size
    pub size: u32,
    /// Value if write
    pub value: Option<Vec<u8>>,
    /// Step index
    pub step_index: u64,
}

/// Aggregate execution statistics
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
pub struct ExecutionStats {
    /// Total steps executed
    pub step_count: u64,
    /// Total gas/compute used
    pub gas_used: u64,
    /// Storage reads
    pub storage_reads: u32,
    /// Storage writes
    pub storage_writes: u32,
    /// Memory expansions
    pub memory_expansions: u32,
    /// External calls made
    pub external_calls: u32,
    /// Error count
    pub error_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_step_encoding() {
        let step = ExecutionStep {
            step_index: 1,
            pc: 0,
            opcode: 0x01, // ADD
            stack: vec![vec![1], vec![2]],
            memory: vec![],
            storage_access: None,
            gas_used: 3,
            state_modified: false,
        };

        let encoded = step.encode();
        let decoded: ExecutionStep = ExecutionStep::decode(&mut &encoded[..]).unwrap();
        assert_eq!(step, decoded);
    }

    #[test]
    fn test_execution_trace_complete() {
        let trace = ExecutionTrace {
            tx_id: H256::zero(),
            vm_type: 0, // EVM
            step_count: 100,
            steps: vec![],
            entry_point: vec![0x60, 0x60],
            return_value: vec![0],
            total_gas_used: 21000,
            success: true,
            error_msg: vec![],
        };

        assert_eq!(trace.vm_type, 0);
        assert_eq!(trace.total_gas_used, 21000);
        assert!(trace.success);
    }

    #[test]
    fn test_call_trace_frame_structure() {
        let frame = CallTraceFrame {
            frame_id: 0,
            call_type: 0, // call
            target: vec![0xaa; 20],
            input: vec![0x60, 0x60],
            output: vec![0],
            success: true,
            error: vec![],
            gas_limit: 100000,
            gas_used: 50000,
            value: vec![0],
            children: vec![1, 2],
        };

        assert_eq!(frame.frame_id, 0);
        assert_eq!(frame.children.len(), 2);
    }

    #[test]
    fn test_storage_access_tracking() {
        let access = StorageAccess {
            access_type: 1, // write
            key: H256::from_low_u64_be(0x01),
            value: H256::from_low_u64_be(42),
            step_index: 10,
        };

        assert_eq!(access.access_type, 1);
        assert_eq!(access.step_index, 10);
    }

    #[test]
    fn test_execution_stats_defaults() {
        let stats = ExecutionStats::default();
        assert_eq!(stats.step_count, 0);
        assert_eq!(stats.gas_used, 0);
        assert_eq!(stats.storage_reads, 0);
    }
}
