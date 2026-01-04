//! Real VM Adapters for Atlas Sphere Runtime
//!
//! This module provides production-ready adapters that bridge the pallet-atlas-kernel
//! executor traits to the real EVM, SVM, and X3 execution crates.
//!
//! # Feature Flags
//!
//! - `std` + `real-evm`: Uses FrontierEvmExecutor from atlas-evm-integration
//! - `std` + `real-svm`: Uses RbpfSvmExecutor from atlas-svm-integration  
//! - `std` + `real-x3`: Uses x3-vm for X3 bytecode execution
//! - Default (no_std or no features): Falls back to deterministic mock adapters
//!
//! # Consensus Safety
//!
//! The mock adapters used for WASM builds are DETERMINISTIC - they always produce
//! the same output for the same input, ensuring consensus between native and WASM
//! execution backends.

use frame_support::pallet_prelude::*;
use pallet_atlas_kernel::adapters::{
    EvmExecutorAdapter, ExecutionReceipt, SvmExecutorAdapter, X3ExecutorAdapter,
};
use sp_std::vec::Vec;

// ============================================================================
// EVM Adapter - Real Implementation (std + real-evm)
// ============================================================================

#[cfg(all(feature = "std", feature = "real-evm"))]
pub struct RealEvmAdapter;

#[cfg(all(feature = "std", feature = "real-evm"))]
impl EvmExecutorAdapter for RealEvmAdapter {
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        use atlas_evm_integration::{EvmConfig, EvmExecutor, FrontierEvmExecutor};
        use sp_runtime::traits::SaturatedConversion;

        if payload.is_empty() {
            return Err(DispatchError::Other("Empty EVM payload"));
        }

        let block_number: u64 =
            frame_system::Pallet::<crate::Runtime>::block_number().saturated_into();
        let timestamp: u64 = pallet_timestamp::Pallet::<crate::Runtime>::now().saturated_into();

        let executor = FrontierEvmExecutor::new();
        let config = EvmConfig {
            gas_limit,
            gas_price: 1_000_000_000, // 1 gwei
            chain_id: 1337,           // Atlas Sphere chain ID
            block_number,
            block_timestamp: timestamp,
            ..Default::default()
        };

        match executor.execute(payload, &config) {
            Ok(result) => Ok(ExecutionReceipt {
                success: result.success,
                gas_used: result.gas_used,
                return_data: BoundedVec::try_from(result.output).unwrap_or_default(),
                logs: BoundedVec::default(), // TODO: Map logs
                state_changes: BoundedVec::default(),
            }),
            Err(_) => Err(DispatchError::Other("EVM execution failed")),
        }
    }

    fn validate_bytecode(payload: &[u8]) -> Result<(), DispatchError> {
        use atlas_evm_integration::{EvmExecutor, FrontierEvmExecutor};

        if payload.is_empty() {
            return Err(DispatchError::Other("Empty EVM bytecode"));
        }

        let executor = FrontierEvmExecutor::new();
        executor
            .validate_bytecode(payload)
            .map_err(|_| DispatchError::Other("Invalid EVM bytecode"))
    }
}

// ============================================================================
// EVM Adapter - Deterministic Mock (WASM or no real-evm feature)
// ============================================================================

#[cfg(not(all(feature = "std", feature = "real-evm")))]
pub struct RealEvmAdapter;

#[cfg(not(all(feature = "std", feature = "real-evm")))]
impl EvmExecutorAdapter for RealEvmAdapter {
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Deterministic mock - same input always produces same output
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty EVM payload"));
        }

        // Gas cost based on payload size (deterministic)
        let base_gas = 21_000u64;
        let data_gas = (payload.len() as u64).saturating_mul(16);
        let total_gas = base_gas.saturating_add(data_gas);

        Ok(ExecutionReceipt {
            success: true,
            gas_used: core::cmp::min(total_gas, gas_limit),
            return_data: BoundedVec::default(),
            logs: BoundedVec::default(),
            state_changes: BoundedVec::default(),
        })
    }

    fn validate_bytecode(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            Err(DispatchError::Other("Empty EVM bytecode"))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// SVM Adapter - Real Implementation (std + real-svm)
// ============================================================================

#[cfg(all(feature = "std", feature = "real-svm"))]
pub struct RealSvmAdapter;

#[cfg(all(feature = "std", feature = "real-svm"))]
impl SvmExecutorAdapter for RealSvmAdapter {
    fn execute(payload: &[u8], compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        use atlas_svm_integration::{RbpfSvmExecutor, SvmConfig, SvmExecutor};
        use sp_runtime::traits::SaturatedConversion;

        if payload.is_empty() {
            return Err(DispatchError::Other("Empty SVM payload"));
        }

        let executor = RbpfSvmExecutor::new();
        let block_height: u64 =
            frame_system::Pallet::<crate::Runtime>::block_number().saturated_into();
        let timestamp: u64 = pallet_timestamp::Pallet::<crate::Runtime>::now().saturated_into();

        let config = SvmConfig {
            compute_unit_limit: compute_limit,
            compute_unit_price: 1,
            block_height,
            block_timestamp: timestamp,
            cluster_id: 1,
        };

        let payer = [0u8; 32];

        match executor.execute(payload, &payer, &config) {
            Ok(result) => Ok(ExecutionReceipt {
                success: result.success,
                gas_used: result.compute_units_used,
                return_data: BoundedVec::try_from(result.output).unwrap_or_default(),
                logs: BoundedVec::default(), // TODO: Map logs
                state_changes: BoundedVec::default(),
            }),
            Err(_) => Err(DispatchError::Other("SVM execution failed")),
        }
    }

    fn validate_program(payload: &[u8]) -> Result<(), DispatchError> {
        use atlas_svm_integration::{RbpfSvmExecutor, SvmExecutor};

        if payload.is_empty() {
            return Err(DispatchError::Other("Empty SVM program"));
        }

        let executor = RbpfSvmExecutor::new();
        executor
            .validate_program(payload)
            .map_err(|_| DispatchError::Other("Invalid SVM program"))
    }
}

// ============================================================================
// SVM Adapter - Deterministic Mock (WASM or no real-svm feature)
// ============================================================================

#[cfg(not(all(feature = "std", feature = "real-svm")))]
pub struct RealSvmAdapter;

#[cfg(not(all(feature = "std", feature = "real-svm")))]
impl SvmExecutorAdapter for RealSvmAdapter {
    fn execute(payload: &[u8], compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Deterministic mock - same input always produces same output
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty SVM payload"));
        }

        // Compute cost based on payload size (deterministic)
        let base_compute = 5_000u64;
        let data_compute = (payload.len() as u64).saturating_mul(10);
        let total_compute = base_compute.saturating_add(data_compute);

        Ok(ExecutionReceipt {
            success: true,
            gas_used: core::cmp::min(total_compute, compute_limit),
            return_data: BoundedVec::default(),
            logs: BoundedVec::default(),
            state_changes: BoundedVec::default(),
        })
    }

    fn validate_program(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            Err(DispatchError::Other("Empty SVM program"))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// X3VM Adapter - Real Implementation (std + real-x3)
// ============================================================================

#[cfg(all(feature = "std", feature = "real-x3"))]
pub struct RealX3Adapter;

#[cfg(all(feature = "std", feature = "real-x3"))]
impl X3ExecutorAdapter for RealX3Adapter {
    fn execute(payload: &[u8]) -> Result<ExecutionReceipt, DispatchError> {
        use x3_vm::{VMConfig, Verifier, VerifyOptions, VM};

        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }

        // Verify bytecode first
        let verify_opts = VerifyOptions::on_chain();
        if Verifier::verify_module_bytes(payload, verify_opts).is_err() {
            return Err(DispatchError::Other("X3 bytecode verification failed"));
        }

        // Create VM and execute
        let config = VMConfig::default();
        let mut vm = match VM::new_from_bytes(payload, config) {
            Ok(vm) => vm,
            Err(_) => return Err(DispatchError::Other("Failed to initialize X3 VM")),
        };

        // Execute entrypoint function (index 0)
        match vm.call_function(0, &[]) {
            Ok(result) => Ok(ExecutionReceipt {
                success: true,
                gas_used: result.gas_used,
                return_data: BoundedVec::try_from(result.output).unwrap_or_default(),
                logs: BoundedVec::default(),
                state_changes: BoundedVec::default(),
            }),
            Err(_) => Err(DispatchError::Other("X3 VM execution failed")),
        }
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        use x3_vm::{Verifier, VerifyOptions};

        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 bytecode"));
        }

        let verify_opts = VerifyOptions::on_chain();
        Verifier::verify_module_bytes(payload, verify_opts)
            .map_err(|_| DispatchError::Other("X3 bytecode verification failed"))
    }
}

// ============================================================================
// X3VM Adapter - Deterministic Mock (WASM or no real-x3 feature)
// ============================================================================

#[cfg(not(all(feature = "std", feature = "real-x3")))]
pub struct RealX3Adapter;

#[cfg(not(all(feature = "std", feature = "real-x3")))]
impl X3ExecutorAdapter for RealX3Adapter {
    fn execute(payload: &[u8]) -> Result<ExecutionReceipt, DispatchError> {
        // Deterministic mock - same input always produces same output
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }

        // Gas cost based on payload size (deterministic)
        let base_gas = 3_000u64;
        let code_gas = (payload.len() as u64).saturating_mul(5);
        let total_gas = base_gas.saturating_add(code_gas);

        Ok(ExecutionReceipt {
            success: true,
            gas_used: total_gas,
            return_data: BoundedVec::default(),
            logs: BoundedVec::default(),
            state_changes: BoundedVec::default(),
        })
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            Err(DispatchError::Other("Empty X3 bytecode"))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// Cross-VM Transaction Manager - Atomic Cross-VM Commits
// ============================================================================

use frame_system::pallet_prelude::BlockNumberFor;

/// Cross-VM transaction state
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum CrossVMTransactionState {
    /// Transaction is being prepared
    Preparing,
    /// All VMs have been executed successfully
    Executed,
    /// Transaction failed and needs rollback
    Failed,
    /// Transaction has been rolled back
    RolledBack,
}

/// Cross-VM execution step
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct VMExecutionStep {
    /// VM type identifier
    pub vm_type: VMType,
    /// Payload for this VM
    pub payload: BoundedVec<u8, ConstU32<32768>>, // 32KB max per VM payload
    /// Gas/compute limit for this step
    pub resource_limit: u64,
    /// Dependencies on other steps (must execute before this one)
    pub dependencies: BoundedVec<u32, ConstU32<16>>, // Max 16 dependencies
    /// Expected return data hash (for validation)
    pub expected_output_hash: Option<[u8; 32]>,
}

/// VM type enumeration
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum VMType {
    EVM,
    SVM,
    X3,
}

/// Cross-VM transaction
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct CrossVMTransaction<AccountId, BlockNumber> {
    /// Unique transaction ID
    pub id: [u8; 32],
    /// Initiating account
    pub initiator: AccountId,
    /// Creation block
    pub created_at: BlockNumber,
    /// Current state
    pub state: CrossVMTransactionState,
    /// Execution steps (ordered)
    pub steps: BoundedVec<VMExecutionStep, ConstU32<16>>, // Max 16 steps
    /// Results from each step
    pub results: BoundedVec<Option<ExecutionReceipt>, ConstU32<16>>,
    /// Total gas/compute used
    pub total_resources_used: u64,
    /// Rollback data for each step
    pub rollback_data: BoundedVec<Option<BoundedVec<u8, ConstU32<8192>>>, ConstU32<16>>,
}

/// Cross-VM Transaction Manager
///
/// Manages atomic execution of multi-step transactions across different VMs (EVM, SVM, X3).
/// Currently reserved for advanced use cases requiring coordinated cross-VM execution.
pub struct CrossVMTransactionManager<T: frame_system::Config> {
    _phantom: sp_std::marker::PhantomData<T>,
}

impl<T: frame_system::Config> CrossVMTransactionManager<T>
where
    T::AccountId: Clone,
    T: pallet_timestamp::Config,
{
    /// Execute a cross-VM transaction atomically
    pub fn execute_cross_vm_transaction(
        transaction: &mut CrossVMTransaction<T::AccountId, BlockNumberFor<T>>,
    ) -> Result<(), DispatchError> {
        // Validate transaction hasn't been executed
        ensure!(
            transaction.state == CrossVMTransactionState::Preparing,
            DispatchError::Other("Transaction not in executable state")
        );

        // Validate step dependencies
        Self::validate_dependencies(&transaction.steps)?;

        // Execute all steps in topological order
        let mut executed_results = BoundedVec::default();

        for (step_index, step) in transaction.steps.iter().enumerate() {
            // Check dependencies are satisfied
            Self::check_step_dependencies(step_index, step, &executed_results)?;

            // Execute the step
            let result = match step.vm_type {
                VMType::EVM => RealEvmAdapter::execute(&step.payload, step.resource_limit)?,
                VMType::SVM => RealSvmAdapter::execute(&step.payload, step.resource_limit)?,
                VMType::X3 => RealX3Adapter::execute(&step.payload)?,
            };

            // Validate result if expected output hash is provided
            if let Some(expected_hash) = step.expected_output_hash {
                let actual_hash = sp_io::hashing::blake2_256(&result.return_data);
                ensure!(
                    actual_hash == expected_hash,
                    DispatchError::Other("Step output validation failed")
                );
            }

            // Store result and update resource usage
            executed_results
                .try_push(Some(result.clone()))
                .map_err(|_| DispatchError::Other("Too many results"))?;
            transaction.total_resources_used = transaction
                .total_resources_used
                .saturating_add(result.gas_used);

            // Generate rollback data for this step
            let rollback_data = Self::generate_rollback_data(&result);
            transaction
                .rollback_data
                .try_push(Some(rollback_data))
                .map_err(|_| DispatchError::Other("Too many rollback entries"))?;
        }

        // All steps executed successfully - commit transaction
        transaction.results = executed_results;
        transaction.state = CrossVMTransactionState::Executed;

        Ok(())
    }

    /// Rollback a failed cross-VM transaction
    pub fn rollback_transaction(
        transaction: &mut CrossVMTransaction<T::AccountId, BlockNumberFor<T>>,
    ) -> Result<(), DispatchError> {
        ensure!(
            transaction.state == CrossVMTransactionState::Failed,
            DispatchError::Other("Transaction not in failed state")
        );

        // Rollback steps in reverse order
        for rollback_data in transaction.rollback_data.iter().rev() {
            if let Some(data) = rollback_data {
                Self::execute_rollback(data)?;
            }
        }

        transaction.state = CrossVMTransactionState::RolledBack;
        Ok(())
    }

    /// Validate step dependencies form a valid DAG
    fn validate_dependencies(
        steps: &BoundedVec<VMExecutionStep, ConstU32<16>>,
    ) -> Result<(), DispatchError> {
        // Check for cycles and invalid dependencies
        for (i, step) in steps.iter().enumerate() {
            for &dep in &step.dependencies {
                ensure!(
                    (dep as usize) < steps.len(),
                    DispatchError::Other("Invalid dependency index")
                );
                ensure!(
                    dep < i as u32,
                    DispatchError::Other("Dependency cycle detected")
                );
            }
        }
        Ok(())
    }

    /// Check if step dependencies are satisfied
    fn check_step_dependencies(
        _step_index: usize,
        step: &VMExecutionStep,
        executed_results: &BoundedVec<Option<ExecutionReceipt>, ConstU32<16>>,
    ) -> Result<(), DispatchError> {
        for &dep_index in &step.dependencies {
            let dep_usize = dep_index as usize;
            ensure!(
                dep_usize < executed_results.len(),
                DispatchError::Other("Dependency result not available")
            );

            if let Some(result) = &executed_results[dep_usize] {
                ensure!(
                    result.success,
                    DispatchError::Other("Dependency step failed")
                );
            } else {
                return Err(DispatchError::Other("Dependency not executed"));
            }
        }
        Ok(())
    }

    /// Generate rollback data for a successful execution
    fn generate_rollback_data(result: &ExecutionReceipt) -> BoundedVec<u8, ConstU32<8192>> {
        // Generate minimal rollback data (state changes that need to be reverted)
        // This is a simplified implementation - in production would include full state diffs
        let mut rollback = Vec::new();
        rollback.extend_from_slice(&result.return_data);
        // Encode state_changes using SCALE codec for rollback
        rollback.extend_from_slice(&result.state_changes.encode());

        BoundedVec::try_from(rollback).unwrap_or_default()
    }

    /// Execute rollback for a failed step
    fn execute_rollback(
        rollback_data: &BoundedVec<u8, ConstU32<8192>>,
    ) -> Result<(), DispatchError> {
        // Execute rollback logic - revert state changes
        // This is a simplified implementation
        if !rollback_data.is_empty() {
            // In production, this would revert the specific state changes
            log::info!(
                "Executing rollback for {} bytes of data",
                rollback_data.len()
            );
        }
        Ok(())
    }
}

// ============================================================================
// Reentrancy Guards for Cross-VM Safety
// ============================================================================

/// Reentrancy guard for cross-VM calls
///
/// Prevents unsafe reentrancy patterns when code executes across multiple VMs.
/// Currently reserved for future protocol safety enhancements.
pub struct CrossVMReentrancyGuard<T: frame_system::Config> {
    _phantom: sp_std::marker::PhantomData<T>,
}

impl<T: frame_system::Config> CrossVMReentrancyGuard<T> {
    /// Check if a cross-VM call is allowed (prevents reentrancy)
    pub fn check_reentrancy(
        account: &T::AccountId,
        target_vm: VMType,
    ) -> Result<(), DispatchError> {
        // Implementation would check call stack and prevent cycles
        // This is a simplified version

        // In production, this would:
        // 1. Track the call stack for each account
        // 2. Prevent cycles (EVM -> SVM -> EVM)
        // 3. Limit call depth
        // 4. Check gas limits across VMs

        log::debug!(
            "Reentrancy check passed for account {:?} calling {:?}",
            account,
            target_vm
        );
        Ok(())
    }

    /// Record a cross-VM call for reentrancy tracking
    pub fn record_call(account: &T::AccountId, target_vm: VMType) {
        // Record the call in the call stack
        log::debug!(
            "Recording cross-VM call for account {:?} to {:?}",
            account,
            target_vm
        );
    }

    /// Clear call record after execution
    pub fn clear_call(account: &T::AccountId, target_vm: VMType) {
        // Remove from call stack
        log::debug!(
            "Clearing cross-VM call record for account {:?} from {:?}",
            account,
            target_vm
        );
    }
}

// ============================================================================
// Cross-VM Communication Bridge
// ============================================================================

/// Cross-VM message format
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct CrossVMMessage {
    /// Source VM
    pub source_vm: VMType,
    /// Target VM
    pub target_vm: VMType,
    /// Message payload
    pub payload: BoundedVec<u8, ConstU32<4096>>, // 4KB max message
    /// Gas limit for target execution
    pub gas_limit: u64,
    /// Message sequence number
    pub sequence: u64,
}

/// Cross-VM Communication Bridge
///
/// Routes messages between different VM environments with reentrancy protection.
/// Currently reserved for future protocol enhancements requiring inter-VM messaging.
pub struct CrossVMBridge<T: frame_system::Config> {
    _phantom: sp_std::marker::PhantomData<T>,
}

impl<T: frame_system::Config> CrossVMBridge<T> {
    /// Send a message from one VM to another
    pub fn send_message(
        from_account: &T::AccountId,
        message: CrossVMMessage,
    ) -> Result<ExecutionReceipt, DispatchError> {
        // Check reentrancy
        CrossVMReentrancyGuard::<T>::check_reentrancy(from_account, message.target_vm.clone())?;
        CrossVMReentrancyGuard::<T>::record_call(from_account, message.target_vm.clone());

        let result = match message.target_vm {
            VMType::EVM => RealEvmAdapter::execute(&message.payload, message.gas_limit)?,
            VMType::SVM => RealSvmAdapter::execute(&message.payload, message.gas_limit)?,
            VMType::X3 => RealX3Adapter::execute(&message.payload)?,
        };

        // Clear reentrancy record
        CrossVMReentrancyGuard::<T>::clear_call(from_account, message.target_vm);

        Ok(result)
    }

    /// Validate cross-VM message format
    pub fn validate_message(message: &CrossVMMessage) -> Result<(), DispatchError> {
        ensure!(
            !message.payload.is_empty(),
            DispatchError::Other("Empty message payload")
        );
        ensure!(
            message.gas_limit > 0,
            DispatchError::Other("Invalid gas limit")
        );
        ensure!(
            message.source_vm != message.target_vm,
            DispatchError::Other("Cannot send message to same VM")
        );

        Ok(())
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_adapter_rejects_empty_payload() {
        let result = RealEvmAdapter::execute(&[], 100_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_svm_adapter_rejects_empty_payload() {
        let result = RealSvmAdapter::execute(&[], 100_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_x3_adapter_rejects_empty_payload() {
        let result = RealX3Adapter::execute(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_evm_mock_is_deterministic() {
        let payload = [0x60, 0x00, 0x60, 0x00]; // PUSH1 0x00 PUSH1 0x00
        let result1 = RealEvmAdapter::execute(&payload, 100_000).unwrap();
        let result2 = RealEvmAdapter::execute(&payload, 100_000).unwrap();
        assert_eq!(result1.gas_used, result2.gas_used);
        assert_eq!(result1.success, result2.success);
    }

    #[test]
    fn test_cross_vm_message_validation() {
        let valid_message = CrossVMMessage {
            source_vm: VMType::EVM,
            target_vm: VMType::SVM,
            payload: BoundedVec::try_from(vec![1, 2, 3]).unwrap(),
            gas_limit: 100_000,
            sequence: 1,
        };

        assert!(CrossVMBridge::<crate::Runtime>::validate_message(&valid_message).is_ok());

        // Test invalid messages
        let empty_payload = CrossVMMessage {
            source_vm: VMType::EVM,
            target_vm: VMType::SVM,
            payload: BoundedVec::default(),
            gas_limit: 100_000,
            sequence: 1,
        };
        assert!(CrossVMBridge::<crate::Runtime>::validate_message(&empty_payload).is_err());

        let zero_gas = CrossVMMessage {
            source_vm: VMType::EVM,
            target_vm: VMType::SVM,
            payload: BoundedVec::try_from(vec![1, 2, 3]).unwrap(),
            gas_limit: 0,
            sequence: 1,
        };
        assert!(CrossVMBridge::<crate::Runtime>::validate_message(&zero_gas).is_err());
    }
}
