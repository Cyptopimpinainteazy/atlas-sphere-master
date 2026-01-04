//! SVM (Solana Virtual Machine) Integration for Atlas Sphere
//!
//! This crate provides integration points for executing SVM transactions
//! as part of dual-VM operations on Atlas Sphere.
//!
//! rBPF executor
//! ----------------
//! When compiled with the `rbpf-executor` Cargo feature this crate exposes
//! `RbpfSvmExecutor` which performs verification and execution of Solana BPF
//! programs. The executor uses `solana-rbpf` for verification and (optionally)
//! JIT/interpreter execution. A simple in-memory account backend is provided
//! for test and runtime usage. Syscall handlers (logging, CPI, sysvars)
//! are planned and documented in the code where they will be registered into
//! the rBPF VM at initialization.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec;
use sp_std::vec::Vec;

/// Result type for SVM operations
pub type SvmResult<T> = Result<T, SvmError>;

/// Errors that can occur during SVM execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvmError {
    /// Invalid program or transaction data
    InvalidPayload,
    /// Program execution failed
    ExecutionFailed,
    /// Account not found or invalid
    InvalidAccount,
    /// Signature verification failed
    InvalidSignature,
    /// Other execution error
    ExecutionError(u32),
}

/// Represents the result of SVM program execution
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SvmExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Output data from the execution
    pub output: Vec<u8>,
    /// Compute units used in the execution
    pub compute_units_used: u64,
    /// Account changes during execution
    pub account_updates: Vec<AccountUpdate>,
    /// State root after execution
    pub state_root: [u8; 32],
    /// Logs emitted by the program via `sol_log_`
    pub logs: Vec<Vec<u8>>,
}

/// Represents an update to an account during SVM execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdate {
    /// Account public key (32 bytes)
    pub pubkey: [u8; 32],
    /// New account data
    pub data: Vec<u8>,
    /// New lamport balance
    pub lamports: u64,
    /// Is account executable
    pub executable: bool,
}

/// SVM execution environment configuration
#[derive(Debug, Clone)]
pub struct SvmConfig {
    /// Maximum compute units per transaction
    pub compute_unit_limit: u64,
    /// Compute unit price (microlamports)
    pub compute_unit_price: u64,
    /// Block height for execution context
    pub block_height: u64,
    /// Block timestamp for execution context
    pub block_timestamp: u64,
    /// Cluster identifier
    pub cluster_id: u8,
}

impl Default for SvmConfig {
    fn default() -> Self {
        Self {
            compute_unit_limit: 200_000, // Standard compute limit
            compute_unit_price: 1,       // 1 microlamport per compute unit
            block_height: 0,
            block_timestamp: 0,
            cluster_id: 1, // Solana mainnet-beta cluster ID
        }
    }
}

/// SvmConfig builder for explicit runtime configuration
impl SvmConfig {
    /// Create a new SvmConfig with explicit parameters
    pub fn new(
        compute_unit_limit: u64,
        compute_unit_price: u64,
        block_height: u64,
        block_timestamp: u64,
        cluster_id: u8,
    ) -> Self {
        Self {
            compute_unit_limit,
            compute_unit_price,
            block_height,
            block_timestamp,
            cluster_id,
        }
    }
}

/// Trait for SVM execution adapters
pub trait SvmExecutor {
    /// Execute SVM program
    fn execute(
        &self,
        payload: &[u8],
        payer: &[u8; 32],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult>;

    /// Validate SVM program without executing
    fn validate_program(&self, payload: &[u8]) -> SvmResult<()>;
}

/// Mock SVM executor for testing (always succeeds)
pub struct MockSvmExecutor;

impl SvmExecutor for MockSvmExecutor {
    fn execute(
        &self,
        payload: &[u8],
        _payer: &[u8; 32],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        if payload.is_empty() {
            return Err(SvmError::InvalidPayload);
        }

        Ok(SvmExecutionResult {
            success: true,
            output: vec![0x01], // Success indicator
            compute_units_used: config.compute_unit_limit / 2,
            account_updates: vec![],
            state_root: [0u8; 32],
            logs: vec![],
        })
    }

    fn validate_program(&self, payload: &[u8]) -> SvmResult<()> {
        if payload.is_empty() {
            Err(SvmError::InvalidPayload)
        } else {
            Ok(())
        }
    }
}

// Solana rBPF-backed SVM executor.
//
// Enabled via the `rbpf-executor` Cargo feature.
#[cfg(feature = "rbpf-executor")]
mod rbpf {
    use super::*;
    use parity_scale_codec::{Decode, Encode};
    use sp_core::hashing::blake2_256;
    use sp_std::collections::btree_map::BTreeMap;

    use _solana_rbpf::aligned_memory::AlignedMemory;
    use _solana_rbpf::ebpf;
    use _solana_rbpf::elf::Executable;
    use _solana_rbpf::memory_region::{MemoryMapping, MemoryRegion};
    use _solana_rbpf::program::{BuiltinProgram, FunctionRegistry, SBPFVersion};
    use _solana_rbpf::verifier::RequisiteVerifier;
    use solana_rbpf as _solana_rbpf;
    use solana_rbpf::vm::{ContextObject, EbpfVm, TestContextObject};

    use core::convert::TryInto;
    use sp_std::sync::Arc;
    use sp_std::vec::Vec;

    /// SCALE encoded SVM payload (program bytecode, accounts, instruction data)
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
    pub struct SvmPayload {
        /// Raw BPF program bytes (ELF or raw BPF). For now, we accept bytes directly
        pub program: Vec<u8>,
        /// Accounts passed to the instruction
        pub accounts: Vec<SvmAccountInput>,
        /// Instruction data buffer
        pub data: Vec<u8>,
    }

    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
    pub struct SvmAccountInput {
        pub pubkey: [u8; 32],
        pub is_signer: bool,
        pub is_writable: bool,
        pub lamports: u64,
        pub data: Vec<u8>,
        pub owner: [u8; 32],
        pub executable: bool,
    }

    /// In-memory account state for SVM execution
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SvmAccount {
        pub pubkey: [u8; 32],
        pub lamports: u64,
        pub data: Vec<u8>,
        pub owner: [u8; 32],
        pub executable: bool,
        pub rent_epoch: u64,
    }

    impl From<&SvmAccountInput> for SvmAccount {
        fn from(input: &SvmAccountInput) -> Self {
            Self {
                pubkey: input.pubkey,
                lamports: input.lamports,
                data: input.data.clone(),
                owner: input.owner,
                executable: input.executable,
                rent_epoch: 0,
            }
        }
    }

    /// Simple in-memory state backend.
    pub struct StateBackend {
        accounts: BTreeMap<[u8; 32], SvmAccount>,
    }

    impl StateBackend {
        pub fn new() -> Self {
            Self {
                accounts: BTreeMap::new(),
            }
        }

        pub fn load_account(&self, pubkey: &[u8; 32]) -> Option<SvmAccount> {
            self.accounts.get(pubkey).cloned()
        }

        pub fn store_account(&mut self, account: SvmAccount) {
            self.accounts.insert(account.pubkey, account);
        }

        pub fn iter_accounts(&self) -> impl Iterator<Item = &SvmAccount> {
            self.accounts.values()
        }
    }

    fn hash_account(account: &SvmAccount) -> [u8; 32] {
        let mut data = Vec::new();
        data.extend_from_slice(&account.pubkey);
        data.extend_from_slice(&account.lamports.to_le_bytes());
        data.extend_from_slice(&account.data);
        data.extend_from_slice(&account.owner);
        data.push(account.executable as u8);
        blake2_256(&data)
    }

    fn compute_state_root(state: &StateBackend) -> [u8; 32] {
        // Concatenate account hashes in pubkey order and then hash the result
        let mut concat = Vec::new();
        for acc in state.iter_accounts() {
            concat.extend_from_slice(&hash_account(acc));
        }
        blake2_256(&concat)
    }

    /// rBPF-backed executor implementation
    pub struct RbpfSvmExecutor {
        // potential configuration or caches can go here
    }

    impl RbpfSvmExecutor {
        pub fn new() -> Self {
            Self {}
        }

        /// Minimal helper: attempt to perform rBPF verification using solana_rbpf
        fn verify_bpf(program: &[u8]) -> Result<(), ()> {
            // Try to construct an Executable; this performs validation/relocations
            let loader = Arc::new(BuiltinProgram::new_mock());
            let function_registry = FunctionRegistry::default();
            let sbpf_version = SBPFVersion::V2;
            let exe_res = Executable::<TestContextObject>::from_text_bytes(
                program,
                loader,
                sbpf_version,
                function_registry,
            );
            match exe_res {
                Ok(exe) => exe.verify::<RequisiteVerifier>().map_err(|_| ()),
                Err(_) => Err(()),
            }
        }
    }

    impl super::SvmExecutor for RbpfSvmExecutor {
        fn execute(
            &self,
            payload: &[u8],
            _payer: &[u8; 32],
            config: &super::SvmConfig,
        ) -> super::SvmResult<super::SvmExecutionResult> {
            // 1. Decode payload
            let p = SvmPayload::decode(&mut &payload[..])
                .map_err(|_| super::SvmError::InvalidPayload)?;
            if p.program.is_empty() {
                return Err(super::SvmError::InvalidPayload);
            }

            // 2. Verify program bytes using rBPF verifier indirectly
            if let Err(_) = Self::verify_bpf(&p.program) {
                return Err(super::SvmError::InvalidPayload);
            }

            // 3. Prepare state backend from accounts
            let mut backend = StateBackend::new();
            for acc_in in &p.accounts {
                backend.store_account(SvmAccount::from(acc_in));
            }

            // 4. Initialize rBPF VM
            let loader = Arc::new(BuiltinProgram::new_mock());
            let function_registry = FunctionRegistry::default();
            let sbpf_version = SBPFVersion::V2;

            let executable = Executable::<TestContextObject>::from_text_bytes(
                p.program.as_slice(),
                loader.clone(),
                sbpf_version.clone(),
                function_registry,
            )
            .map_err(|_| super::SvmError::InvalidPayload)?;
            executable
                .verify::<RequisiteVerifier>()
                .map_err(|_| super::SvmError::InvalidPayload)?;

            let stack_size = executable.get_config().stack_size();
            let mut stack = AlignedMemory::<{ ebpf::HOST_ALIGN }>::zero_filled(stack_size);
            let stack_len = stack.len();
            let mut heap = AlignedMemory::<{ ebpf::HOST_ALIGN }>::with_capacity(0);

            // Build input memory: pack account headers + data
            let mut input_mem: Vec<u8> = Vec::new();
            let mut account_offsets: BTreeMap<[u8; 32], (usize, usize)> = BTreeMap::new();
            for acc_in in &p.accounts {
                let offset = input_mem.len();
                // Header: lamports (8), owner (32), exec (1), rent_epoch (8), data_len (4)
                input_mem.extend_from_slice(&acc_in.lamports.to_le_bytes());
                input_mem.extend_from_slice(&acc_in.owner);
                input_mem.push(acc_in.executable as u8);
                input_mem.extend_from_slice(&0u64.to_le_bytes()); // rent_epoch
                let data_len = acc_in.data.len() as u32;
                input_mem.extend_from_slice(&data_len.to_le_bytes());
                input_mem.extend_from_slice(&acc_in.data);
                let end = input_mem.len();
                account_offsets.insert(acc_in.pubkey, (offset, end - offset));
            }

            let mut regions: Vec<MemoryRegion> = vec![executable.get_ro_region()];
            regions.push(MemoryRegion::new_writable(
                stack.as_slice_mut(),
                ebpf::MM_STACK_START,
            ));
            regions.push(MemoryRegion::new_writable(
                heap.as_slice_mut(),
                ebpf::MM_HEAP_START,
            ));
            regions.push(MemoryRegion::new_writable(
                input_mem.as_mut_slice(),
                ebpf::MM_INPUT_START,
            ));

            let memory_mapping =
                MemoryMapping::new(regions, executable.get_config(), &sbpf_version)
                    .map_err(|_| super::SvmError::ExecutionFailed)?;

            let mut context = TestContextObject::new(config.compute_unit_limit as u64);

            let mut vm = EbpfVm::new(
                loader.clone(),
                &sbpf_version,
                &mut context,
                memory_mapping,
                stack_len,
            );

            // Execute program (interpreted = false -> interpreter-only)
            let (instruction_count, program_result) = vm.execute_program(&executable, false);

            let mut compute_units_used = instruction_count;
            if compute_units_used > config.compute_unit_limit {
                return Err(super::SvmError::ExecutionError(0x1001)); // OUT_OF_COMPUTE
            }

            if program_result.is_err() {
                // If the compute budget was zero or exhausted, prefer OUT_OF_COMPUTE
                if config.compute_unit_limit == 0 || context.get_remaining() == 0 {
                    return Err(super::SvmError::ExecutionError(0x1001)); // OUT_OF_COMPUTE
                }
                return Err(super::SvmError::ExecutionFailed);
            }

            // Simulate detection of syscalls triggered by instruction data prefix markers.
            // This provides a deterministic and testable syscall surface until full
            // host-function registration is wired.
            let mut logs: Vec<Vec<u8>> = Vec::new();
            if p.data.starts_with(b"LOG:") {
                let msg = p.data[4..].to_vec();
                // cost 10 units for logging
                compute_units_used = compute_units_used.saturating_add(10);
                logs.push(msg);
            }

            // MEMCPY:<src_pubkey_hex>:<dst_pubkey_hex>:<dst_offset>:<len>
            if p.data.starts_with(b"MEMCPY:") {
                let s = core::str::from_utf8(&p.data[7..]).unwrap_or("");
                let parts: Vec<&str> = s.split(':').collect();
                if parts.len() == 4 {
                    if let (Ok(dst_off), Ok(len)) =
                        (parts[2].parse::<usize>(), parts[3].parse::<usize>())
                    {
                        // find source and destination accounts by hex pubkey
                        let src_hex = parts[0];
                        let dst_hex = parts[1];
                        if let (Ok(src_pub), Ok(dst_pub)) =
                            (hex::decode(src_hex), hex::decode(dst_hex))
                        {
                            if src_pub.len() == 32 && dst_pub.len() == 32 {
                                let mut src = [0u8; 32];
                                src.copy_from_slice(&src_pub);
                                let mut dst = [0u8; 32];
                                dst.copy_from_slice(&dst_pub);
                                if let (Some(mut src_acc), Some(mut dst_acc)) =
                                    (backend.load_account(&src), backend.load_account(&dst))
                                {
                                    // clamp length
                                    let copy_len = core::cmp::min(len, src_acc.data.len());
                                    let dst_off = core::cmp::min(dst_off, dst_acc.data.len());
                                    for i in 0..copy_len {
                                        dst_acc.data[dst_off + i] = src_acc.data[i];
                                    }
                                    backend.store_account(dst_acc.clone());
                                    // record update
                                    // cost 50 units
                                    compute_units_used = compute_units_used.saturating_add(50);
                                }
                            }
                        }
                    }
                }
            }

            // CPI:<base64_payload> -- nested SCALE-encoded payload
            if p.data.starts_with(b"CPI:") {
                let b64 = &p.data[4..];
                if let Ok(bytes) = base64::decode(b64) {
                    // recursively execute nested payload with reduced compute limit
                    let nested_cfg = super::SvmConfig::new(
                        (config.compute_unit_limit / 4),
                        config.compute_unit_price,
                        config.block_height,
                        config.block_timestamp,
                        config.cluster_id,
                    );
                    let nested_res = self.execute(&bytes, _payer, &nested_cfg);
                    match nested_res {
                        Ok(nr) => {
                            // merge account updates
                            compute_units_used =
                                compute_units_used.saturating_add(nr.compute_units_used);
                            for u in nr.account_updates {
                                // apply to backend
                                backend.store_account(super::rbpf::SvmAccount {
                                    pubkey: u.pubkey,
                                    lamports: u.lamports,
                                    data: u.data.clone(),
                                    owner: [0u8; 32],
                                    executable: u.executable,
                                    rent_epoch: 0,
                                });
                            }
                            if !nr.logs.is_empty() {
                                logs.extend(nr.logs.clone());
                            }
                        }
                        Err(_) => return Err(super::SvmError::ExecutionFailed),
                    }
                }
            }

            // Read back account data and detect updates
            let mut account_updates: Vec<super::AccountUpdate> = Vec::new();
            for acc_in in &p.accounts {
                if let Some((offset, _len)) = account_offsets.get(&acc_in.pubkey) {
                    let base = *offset;
                    let lamports = u64::from_le_bytes(
                        input_mem[base..base + 8].try_into().unwrap_or([0u8; 8]),
                    );
                    let owner_slice = &input_mem[base + 8..base + 40];
                    let mut owner = [0u8; 32];
                    owner.copy_from_slice(owner_slice);
                    let exec_flag = input_mem[base + 40] != 0;
                    let data_len = u32::from_le_bytes(
                        input_mem[base + 41..base + 45]
                            .try_into()
                            .unwrap_or([0u8; 4]),
                    ) as usize;
                    let data_start = base + 45;
                    let data_end = data_start + data_len;
                    let data = input_mem[data_start..data_end].to_vec();

                    if let Some(stored) = backend.load_account(&acc_in.pubkey) {
                        if data != stored.data
                            || lamports != stored.lamports
                            || exec_flag != stored.executable
                            || owner != stored.owner
                        {
                            account_updates.push(super::AccountUpdate {
                                pubkey: acc_in.pubkey,
                                data: stored.data.clone(),
                                lamports: stored.lamports,
                                executable: stored.executable,
                            });
                        }
                    }
                }
            }

            let state_root = compute_state_root(&backend);

            Ok(super::SvmExecutionResult {
                success: true,
                output: vec![],
                compute_units_used,
                account_updates,
                state_root,
                logs,
            })
        }

        fn validate_program(&self, payload: &[u8]) -> super::SvmResult<()> {
            let p = SvmPayload::decode(&mut &payload[..])
                .map_err(|_| super::SvmError::InvalidPayload)?;
            if p.program.is_empty() {
                return Err(super::SvmError::InvalidPayload);
            }

            match Self::verify_bpf(&p.program) {
                Ok(()) => Ok(()),
                Err(_) => Err(super::SvmError::InvalidPayload),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use parity_scale_codec::Encode;

        #[test]
        fn test_validate_rejects_invalid_program() {
            let payload = SvmPayload {
                program: vec![0x00, 0x01],
                accounts: vec![],
                data: vec![],
            };
            let exec = RbpfSvmExecutor::new();
            let res = exec.validate_program(&payload.encode());
            // Most random bytes are rejected by the rBPF verifier
            assert!(res.is_err());
        }

        #[test]
        fn test_execute_minimal_program() {
            // minimal program that just exits
            let prog = vec![0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let payload = SvmPayload {
                program: prog.clone(),
                accounts: vec![],
                data: vec![],
            };
            let exec = RbpfSvmExecutor::new();
            let cfg = super::super::SvmConfig::default();
            let res = exec.execute(&payload.encode(), &[0u8; 32], &cfg);
            assert!(res.is_ok());
            let r = res.unwrap();
            assert_eq!(r.compute_units_used, 1);
            assert!(r.success);
        }

        #[test]
        fn test_sol_log_syscall_simulation() {
            let prog = vec![0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let data = [b"LOG:", b"hello world"].concat();
            let payload = SvmPayload {
                program: prog,
                accounts: vec![],
                data,
            };
            let exec = RbpfSvmExecutor::new();
            let cfg = super::super::SvmConfig::default();
            let res = exec.execute(&payload.encode(), &[0u8; 32], &cfg).unwrap();
            assert_eq!(res.logs.len(), 1);
            assert_eq!(res.logs[0], b"hello world".to_vec());
        }

        #[test]
        fn test_memcpy_syscall_simulation() {
            // prepare two accounts
            let mut acc_a = SvmAccountInput {
                pubkey: [2u8; 32],
                is_signer: false,
                is_writable: true,
                lamports: 0,
                data: vec![1, 2, 3, 4, 5],
                owner: [0u8; 32],
                executable: false,
            };
            let mut acc_b = SvmAccountInput {
                pubkey: [3u8; 32],
                is_signer: false,
                is_writable: true,
                lamports: 0,
                data: vec![0, 0, 0, 0, 0],
                owner: [0u8; 32],
                executable: false,
            };
            let src_hex = hex::encode(acc_a.pubkey);
            let dst_hex = hex::encode(acc_b.pubkey);
            let data = format!("MEMCPY:{}:{}:1:2", src_hex, dst_hex).into_bytes();
            let prog = vec![0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let payload = SvmPayload {
                program: prog,
                accounts: vec![acc_a.clone(), acc_b.clone()],
                data,
            };
            let exec = RbpfSvmExecutor::new();
            let cfg = super::super::SvmConfig::default();
            let res = exec.execute(&payload.encode(), &[0u8; 32], &cfg).unwrap();
            // dst should now have bytes copied from src at offset 1 length 2
            let updated = res
                .account_updates
                .into_iter()
                .find(|u| u.pubkey == acc_b.pubkey)
                .unwrap();
            assert_eq!(updated.data[1], 1);
            assert_eq!(updated.data[2], 2);
        }

        #[test]
        fn test_cpi_simulation() {
            // nested payload: program that logs 'nested'
            let nested_prog = vec![0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let nested_payload = SvmPayload {
                program: nested_prog,
                accounts: vec![],
                data: [b"LOG:", b"nested"].concat(),
            };
            let nested_b64 = base64::encode(&nested_payload.encode());
            let top_prog = vec![0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let data = [b"CPI:", nested_b64.as_bytes()].concat();
            let payload = SvmPayload {
                program: top_prog,
                accounts: vec![],
                data,
            };
            let exec = RbpfSvmExecutor::new();
            let cfg = super::super::SvmConfig::default();
            let res = exec.execute(&payload.encode(), &[0u8; 32], &cfg).unwrap();
            assert!(res.logs.iter().any(|l| l == b"nested"));
        }

        // Load a test program packaged as hex under test-data and validate it.
        #[test]
        fn test_validate_accepts_valid_program() {
            let hex_str = include_str!("../test-data/return_success.hex").trim();
            let program_bytes = hex::decode(hex_str).expect("hex decode");
            let payload = SvmPayload {
                program: program_bytes.to_vec(),
                accounts: vec![],
                data: vec![],
            };
            let exec = RbpfSvmExecutor::new();
            assert!(exec.validate_program(&payload.encode()).is_ok());
        }

        #[test]
        fn test_validate_accepts_write_program() {
            let hex_str = include_str!("../test-data/write_account.hex").trim();
            let program_bytes = hex::decode(hex_str).expect("hex decode");
            let payload = SvmPayload {
                program: program_bytes.to_vec(),
                accounts: vec![],
                data: vec![],
            };
            let exec = RbpfSvmExecutor::new();
            assert!(exec.validate_program(&payload.encode()).is_ok());
        }

        #[test]
        fn test_state_root_deterministic() {
            let mut backend = StateBackend::new();
            let acc = SvmAccount {
                pubkey: [1u8; 32],
                lamports: 100,
                data: vec![1, 2, 3],
                owner: [0u8; 32],
                executable: false,
                rent_epoch: 0,
            };
            backend.store_account(acc.clone());
            let root1 = compute_state_root(&backend);
            let root2 = compute_state_root(&backend);
            assert_eq!(root1, root2);
        }

        #[test]
        fn test_compute_unit_limit_enforced() {
            // minimal exit program (1 instruction)
            let prog = vec![0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let payload = SvmPayload {
                program: prog.clone(),
                accounts: vec![],
                data: vec![],
            };
            let exec = RbpfSvmExecutor::new();
            // Set a tiny compute limit to force an out-of-compute error (limit 0)
            let cfg = super::super::SvmConfig::new(0, 1, 0, 0, 1);
            let res = exec.execute(&payload.encode(), &[0u8; 32], &cfg);
            assert_eq!(res, Err(super::super::SvmError::ExecutionError(0x1001)));
        }
    }
}

#[cfg(feature = "rbpf-executor")]
pub use rbpf::RbpfSvmExecutor;
/// Prepare root computation for SVM execution
pub fn compute_svm_prepare_root(
    comit_id: &[u8; 32],
    payload: &[u8],
    result: &SvmExecutionResult,
) -> [u8; 32] {
    use sp_core::hashing::blake2_256;

    let mut preimage = Vec::new();
    preimage.extend_from_slice(comit_id);
    preimage.extend_from_slice(payload);
    preimage.extend_from_slice(&result.state_root);

    blake2_256(&preimage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SvmConfig::default();
        assert_eq!(config.compute_unit_limit, 200_000);
        assert_eq!(config.cluster_id, 1); // Mainnet cluster
    }

    #[test]
    fn test_mock_executor_success() {
        let executor = MockSvmExecutor;
        let result = executor.execute(&[0x01, 0x02], &[0u8; 32], &SvmConfig::default());
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_mock_executor_empty_payload() {
        let executor = MockSvmExecutor;
        let result = executor.execute(&[], &[0u8; 32], &SvmConfig::default());
        assert_eq!(result, Err(SvmError::InvalidPayload));
    }
}
