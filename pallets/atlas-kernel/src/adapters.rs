//! VM Adapters for Atlas Kernel

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;

/// Execution result from a VM adapter
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ExecutionReceipt {
    /// Whether execution succeeded
    pub success: bool,
    /// Gas used during execution
    pub gas_used: u64,
    /// Return data from execution
    pub return_data: BoundedVec<u8, ConstU32<4096>>,
    /// Logs emitted during execution
    pub logs: BoundedVec<ExecutionLog, ConstU32<32>>,
    /// State changes from execution
    pub state_changes: BoundedVec<StateChange, ConstU32<32>>,
}

/// Log entry emitted during VM execution
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ExecutionLog {
    /// Address that emitted the log
    pub address: BoundedVec<u8, ConstU32<32>>,
    /// Log topics
    pub topics: BoundedVec<H256, ConstU32<4>>,
    /// Log data
    pub data: BoundedVec<u8, ConstU32<1024>>,
}

/// State change resulting from VM execution
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StateChange {
    /// Account/storage key that changed
    pub address: BoundedVec<u8, ConstU32<32>>,
    /// Storage slot key
    pub key: H256,
    /// New value at the storage slot
    pub value: H256,
}

/// VM Adapter trait for pluggable EVM execution
pub trait EvmExecutorAdapter {
    /// Execute a transaction payload on EVM
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError>;
    /// Validate a transaction payload without executing it
    fn validate_bytecode(payload: &[u8]) -> Result<(), DispatchError>;
}

/// VM Adapter trait for pluggable SVM execution
pub trait SvmExecutorAdapter {
    /// Execute a program on SVM
    fn execute(payload: &[u8], compute_limit: u64) -> Result<ExecutionReceipt, DispatchError>;
    /// Validate a program without executing it
    fn validate_program(payload: &[u8]) -> Result<(), DispatchError>;
}

/// Mock EVM Adapter for testing
pub struct MockEvmAdapter;

impl EvmExecutorAdapter for MockEvmAdapter {
    fn execute(_payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Ok(ExecutionReceipt {
            success: true,
            gas_used: 21000,
            return_data: BoundedVec::default(),
            logs: BoundedVec::default(),
            state_changes: BoundedVec::default(),
        })
    }

    fn validate_bytecode(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// Failing Mock EVM Adapter for testing error cases
pub struct FailingMockEvmAdapter;

impl EvmExecutorAdapter for FailingMockEvmAdapter {
    fn execute(_payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Err(DispatchError::Other("Mock EVM execution failed"))
    }

    fn validate_bytecode(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// Mock SVM Adapter for testing
pub struct MockSvmAdapter;

impl SvmExecutorAdapter for MockSvmAdapter {
    fn execute(_payload: &[u8], _compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Ok(ExecutionReceipt {
            success: true,
            gas_used: 5000,
            return_data: BoundedVec::default(),
            logs: BoundedVec::default(),
            state_changes: BoundedVec::default(),
        })
    }

    fn validate_program(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// Failing Mock SVM Adapter for testing error cases
pub struct FailingMockSvmAdapter;

impl SvmExecutorAdapter for FailingMockSvmAdapter {
    fn execute(_payload: &[u8], _compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Err(DispatchError::Other("Mock SVM execution failed"))
    }

    fn validate_program(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// X3 VM Executor Adapter trait
pub trait X3ExecutorAdapter {
    /// Execute on X3 VM
    fn execute(payload: &[u8]) -> Result<ExecutionReceipt, DispatchError>;
    /// Validate X3 payload
    fn validate(payload: &[u8]) -> Result<(), DispatchError>;
}

/// Mock X3 Adapter for testing
pub struct MockX3Adapter;

impl X3ExecutorAdapter for MockX3Adapter {
    fn execute(_payload: &[u8]) -> Result<ExecutionReceipt, DispatchError> {
        Ok(ExecutionReceipt {
            success: true,
            gas_used: 3000,
            return_data: BoundedVec::default(),
            logs: BoundedVec::default(),
            state_changes: BoundedVec::default(),
        })
    }

    fn validate(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// Failing Mock X3 Adapter for testing error cases
pub struct FailingMockX3Adapter;

impl X3ExecutorAdapter for FailingMockX3Adapter {
    fn execute(_payload: &[u8]) -> Result<ExecutionReceipt, DispatchError> {
        Err(DispatchError::Other("Mock X3 execution failed"))
    }

    fn validate(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// Production adapters module (only available in std builds)
#[cfg(feature = "std")]
pub mod real_adapters {
    /// Real Frontier EVM Adapter (uses pallet-evm)
    pub struct FrontierEvmAdapter;

    /// Real RBPF SVM Adapter (uses Solana VM)
    pub struct RbpfSvmAdapter;

    /// Real X3 Adapter
    pub struct X3VmAdapter;
}
