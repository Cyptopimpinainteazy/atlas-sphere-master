//! Real BPF Executor using solana-rbpf
//!
//! This module provides actual Solana BPF program execution using
//! the solana-rbpf virtual machine.

use crate::{
    AccountUpdate, SvmAccountMeta, SvmConfig, SvmError, SvmExecutionResult, SvmExecutor,
    SvmInstruction, SvmResult,
};
use solana_rbpf::{
    elf::Executable,
    error::ProgramResult,
    memory_region::{MemoryMapping, MemoryRegion},
    program::{BuiltinProgram, FunctionRegistry, SBPFVersion},
    verifier::RequisiteVerifier,
    vm::{Config, ContextObject, EbpfVm},
};
use std::sync::Arc;

/// Real SVM executor using solana-rbpf
pub struct RbpfSvmExecutor {
    /// VM configuration
    config: Config,
}

impl RbpfSvmExecutor {
    /// Create a new RBPF executor
    pub fn new() -> Self {
        Self {
            config: Config {
                max_call_depth: 64,
                stack_frame_size: 4096,
                enable_stack_frame_gaps: true,
                instruction_meter_checkpoint_distance: 10000,
                enable_instruction_meter: true,
                enable_instruction_tracing: false,
                enable_symbol_and_section_labels: false,
                reject_broken_elfs: true,
                noop_instruction_rate: 256,
                sanitize_user_provided_values: true,
                external_internal_function_hash_collision: false,
                reject_callx_r10: true,
                optimize_rodata: true,
                aligned_memory_mapping: true,
                ..Config::default()
            },
        }
    }

    /// Create executor with custom config
    pub fn with_config(config: Config) -> Self {
        Self { config }
    }
}

impl Default for RbpfSvmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl RbpfSvmExecutor {
    /// Serialize accounts into a buffer for BPF program access (M-7 fix).
    ///
    /// Format per account:
    /// - 32 bytes: pubkey
    /// - 8 bytes: lamports (little-endian u64)
    /// - 1 byte: is_signer flag
    /// - 1 byte: is_writable flag
    /// - 4 bytes: data length
    /// - Variable: data from AccountUpdate
    fn serialize_accounts(accounts: &[(SvmAccountMeta, AccountUpdate)]) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Write account count as u32 LE
        buffer.extend_from_slice(&(accounts.len() as u32).to_le_bytes());

        for (meta, update) in accounts {
            // Pubkey (32 bytes)
            buffer.extend_from_slice(&meta.pubkey);

            // Lamports from update (8 bytes)
            buffer.extend_from_slice(&update.lamports.to_le_bytes());

            // Flags (2 bytes)
            buffer.push(if meta.is_signer { 1 } else { 0 });
            buffer.push(if meta.is_writable { 1 } else { 0 });

            // Data length and data
            buffer.extend_from_slice(&(update.data.len() as u32).to_le_bytes());
            buffer.extend_from_slice(&update.data);
        }

        buffer
    }

    /// Create a simple test program that returns success
    #[allow(dead_code)]
    fn create_test_program() -> Vec<u8> {
        // Minimal BPF program: mov r0, 0; exit
        vec![
            0xb7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov r0, 0
            0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit
        ]
    }
}

/// Context for Atlas syscall execution
/// Tracks compute units, logs, and return data during BPF execution
struct AtlasSyscallContext {
    /// Remaining compute units
    compute_units_remaining: u64,
    /// Compute units consumed
    compute_units_used: u64,
    /// Logs emitted during execution
    logs: Vec<Vec<u8>>,
    /// Return data from the program (reserved for future use)
    #[allow(dead_code)]
    return_data: Vec<u8>,
}

impl AtlasSyscallContext {
    fn new(compute_limit: u64) -> Self {
        Self {
            compute_units_remaining: compute_limit,
            compute_units_used: 0,
            logs: Vec::new(),
            return_data: Vec::new(),
        }
    }
}

impl ContextObject for AtlasSyscallContext {
    fn trace(&mut self, _state: [u64; 12]) {}

    fn consume(&mut self, amount: u64) {
        self.compute_units_used = self.compute_units_used.saturating_add(amount);
        self.compute_units_remaining = self.compute_units_remaining.saturating_sub(amount);
    }

    fn get_remaining(&self) -> u64 {
        self.compute_units_remaining
    }
}

/// Create the built-in program (no syscalls for minimal version)
fn create_loader() -> Arc<BuiltinProgram<AtlasSyscallContext>> {
    Arc::new(BuiltinProgram::new_loader(
        Config::default(),
        FunctionRegistry::default(),
    ))
}

impl SvmExecutor for RbpfSvmExecutor {
    fn execute(
        &self,
        instruction: &SvmInstruction,
        _payer: [u8; 32],
        accounts: &[(SvmAccountMeta, AccountUpdate)],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        // For now, we expect the program data to be in instruction.data
        // In a full implementation, we'd look up the program from storage by program_id
        if instruction.program_id == [0u8; 32] {
            return Err(SvmError::InvalidProgramId);
        }

        // Serialize accounts into input buffer for BPF program access (M-7 fix)
        let account_input = Self::serialize_accounts(accounts);

        // Execute the BPF program with instruction data + serialized accounts as input
        self.execute_bpf(&instruction.data, &account_input, config)
    }

    fn execute_bpf(
        &self,
        program: &[u8],
        input: &[u8],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        if program.is_empty() {
            return Err(SvmError::InvalidPayload);
        }

        // Create the loader with no syscalls (minimal execution)
        let loader = create_loader();

        // Parse the program (either ELF or raw text bytecode)
        let executable_result = if program.starts_with(b"\x7fELF") {
            Executable::from_elf(program, loader.clone())
        } else {
            Executable::from_text_bytes(
                program,
                loader.clone(),
                SBPFVersion::V1,
                FunctionRegistry::default(),
            )
        };

        let executable = match executable_result {
            Ok(exe) => exe,
            Err(_) => return Err(SvmError::InvalidPayload),
        };

        // Verify the program before execution
        if executable.verify::<RequisiteVerifier>().is_err() {
            return Err(SvmError::InvalidPayload);
        }

        // Create execution context with compute unit metering
        let mut context = AtlasSyscallContext::new(config.compute_unit_limit);

        // Set up memory regions for the VM
        // Region 0: Program code (read-only)
        // Region 1: Input data (read-write for return data)
        let mut input_buffer = input.to_vec();
        // Ensure minimum buffer size for BPF
        if input_buffer.len() < 64 {
            input_buffer.resize(64, 0);
        }

        let regions: Vec<MemoryRegion> =
            vec![MemoryRegion::new_writable(&mut input_buffer, 0x100000000)];

        let sbpf_version = SBPFVersion::V1;
        let memory_mapping = match MemoryMapping::new(regions, &self.config, &sbpf_version) {
            Ok(mm) => mm,
            Err(_) => return Err(SvmError::ExecutionFailed),
        };

        // Create and run the VM
        let mut vm = EbpfVm::new(
            loader,
            &sbpf_version,
            &mut context,
            memory_mapping,
            4096, // stack size
        );

        // Execute the BPF program
        let (instruction_count, result) = vm.execute_program(&executable, true);

        // Consume compute units based on instructions executed
        context.consume(instruction_count);

        // Check if we ran out of compute units
        if context.get_remaining() == 0 && instruction_count >= config.compute_unit_limit {
            return Err(SvmError::OutOfComputeUnits);
        }

        // Interpret execution result
        let (success, return_data) = match result {
            ProgramResult::Ok(return_value) => {
                // Return value 0 indicates success in BPF convention
                (return_value == 0, vec![return_value as u8])
            }
            ProgramResult::Err(_) => (false, vec![]),
        };

        // Compute state root from logs and return data
        let state_root = compute_state_root(&context.logs, &return_data);

        Ok(SvmExecutionResult {
            success,
            output: return_data,
            compute_units_used: context.compute_units_used,
            account_updates: vec![],
            logs: context.logs,
            state_root,
        })
    }

    fn validate_program(&self, program: &[u8]) -> SvmResult<()> {
        if program.is_empty() {
            return Err(SvmError::InvalidPayload);
        }

        let loader = create_loader();
        let sbpf_version = SBPFVersion::V1;

        // Try to parse and verify
        let executable = if program.starts_with(b"\x7fELF") {
            Executable::from_elf(program, loader).map_err(|_| SvmError::InvalidPayload)?
        } else {
            Executable::from_text_bytes(program, loader, sbpf_version, FunctionRegistry::default())
                .map_err(|_| SvmError::InvalidPayload)?
        };

        executable
            .verify::<RequisiteVerifier>()
            .map_err(|_| SvmError::InvalidPayload)?;

        Ok(())
    }
}

/// Compute state root from execution results
fn compute_state_root(logs: &[Vec<u8>], return_data: &[u8]) -> [u8; 32] {
    use sp_io::hashing::blake2_256;

    let mut data = Vec::new();
    for log in logs {
        data.extend_from_slice(log);
    }
    data.extend_from_slice(return_data);

    if data.is_empty() {
        return [0u8; 32];
    }

    blake2_256(&data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbpf_executor_creation() {
        let executor = RbpfSvmExecutor::new();
        assert!(executor.config.enable_instruction_meter);
    }

    #[test]
    fn test_rbpf_executor_empty_program() {
        let executor = RbpfSvmExecutor::new();
        let result = executor.execute_bpf(&[], &[], &SvmConfig::default());
        assert_eq!(result, Err(SvmError::InvalidPayload));
    }

    #[test]
    fn test_rbpf_executor_validate_empty() {
        let executor = RbpfSvmExecutor::new();
        let result = executor.validate_program(&[]);
        assert_eq!(result, Err(SvmError::InvalidPayload));
    }
}
