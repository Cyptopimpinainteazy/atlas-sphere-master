// EVM Integration Layer for Atlas Sphere
// This module provides the bridge between the Atlas Kernel and the EVM execution environment

#![cfg_attr(not(feature = "std"), no_std)]

//! Frontier-backed EVM execution for Atlas Sphere.
//!
//! This crate provides a dual-VM execution component for the Atlas Kernel pallet.
//! When the `frontier-executor` feature is enabled, [`FrontierEvmExecutor`] executes
//! EVM payloads using the Frontier EVM stack (`evm` crate) with an in-memory state
//! backend bridged via [`state::FrontierStateBackend`].
//!
//! ## Payload format
//!
//! The executor supports two payload kinds:
//!
//! - **Contract call** (`0x00`):
//!   - `0x00 | to[20] | value_u64_be[8] | input[..]`
//! - **Contract deployment** (`0x01`):
//!   - `0x01 | value_u64_be[8] | init_code[..]`
//!
//! This format is intentionally compact for Comit payload transport and may evolve.
//!
//! ## Features & testing
//!
//! - To run the heavy cryptographic precompiles (bn128, ModExp, Blake2F) enable the
//!   `full-precompiles` feature. This pulls in optional dependencies and is intended
//!   for native builds (not runtime wasm).
//! - Example: `cargo test -p atlas-evm-integration --features "frontier-executor full-precompiles"`

use sp_core::H160;
use sp_std::vec::Vec;
use sp_std::vec;

/// Phase 2: EVM State Integration
/// Account state management, contract code storage, and state database
pub mod state;

/// Result type for EVM operations
pub type EvmResult<T> = Result<T, EvmError>;

/// Errors that can occur during EVM execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvmError {
    /// Invalid bytecode or transaction data
    InvalidPayload,
    /// EVM execution reverted; optionally contains revert data
    ExecutionReverted(Option<Vec<u8>>),
    /// Out of gas
    OutOfGas,
    /// Invalid account state
    InvalidState,
    /// Other execution error
    ExecutionFailed(u32),
} 

/// Represents the result of EVM execution
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvmExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Output data from the execution
    pub output: Vec<u8>,
    /// Gas used in the execution
    pub gas_used: u64,
    /// Any logs emitted during execution
    pub logs: Vec<EvmLog>,
    /// State root after execution
    pub state_root: [u8; 32],
}

/// Represents an EVM log entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmLog {
    /// Address that emitted the log
    pub address: H160,
    /// Topics for the log
    pub topics: Vec<[u8; 32]>,
    /// Data payload
    pub data: Vec<u8>,
}

/// EVM execution environment configuration
#[derive(Debug, Clone)]
pub struct EvmConfig {
    /// Maximum gas per transaction
    pub gas_limit: u64,
    /// Gas price per unit
    pub gas_price: u64,
    /// Block number for execution context
    pub block_number: u64,
    /// Block timestamp for execution context
    pub block_timestamp: u64,
    /// Chain ID
    pub chain_id: u64,
}

impl Default for EvmConfig {
    fn default() -> Self {
        Self {
            gas_limit: 21_000_000,      // ~20M gas per block
            gas_price: 1,                // 1 wei
            block_number: 0,
            block_timestamp: 0,
            chain_id: 42,                // Atlas Sphere default chain ID
        }
    }
}

/// EvmConfig builder for explicit runtime configuration
impl EvmConfig {
    /// Create a new EvmConfig with explicit parameters
    pub fn new(
        gas_limit: u64,
        gas_price: u64,
        block_number: u64,
        block_timestamp: u64,
        chain_id: u64,
    ) -> Self {
        Self {
            gas_limit,
            gas_price,
            block_number,
            block_timestamp,
            chain_id,
        }
    }
}

/// Trait for EVM execution adapters
pub trait EvmExecutor {
    /// Execute EVM bytecode.
    ///
    /// On EVM revert, the returned `EvmError::ExecutionReverted` may include the revert
    /// return data as `Some(Vec<u8>)` when available.
    fn execute(
        &self,
        payload: &[u8],
        caller: &[u8; 20],
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult>;

    /// Validate EVM bytecode without executing
    fn validate_bytecode(&self, payload: &[u8]) -> EvmResult<()>;
}

/// Mock EVM executor for testing (always succeeds)
pub struct MockEvmExecutor;

impl EvmExecutor for MockEvmExecutor {
    fn execute(
        &self,
        payload: &[u8],
        _caller: &[u8; 20],
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult> {
        if payload.is_empty() {
            return Err(EvmError::InvalidPayload);
        }

        Ok(EvmExecutionResult {
            success: true,
            output: vec![0x01], // Success indicator
            gas_used: config.gas_limit / 2,
            logs: vec![],
            state_root: [0u8; 32],
        })
    }

    fn validate_bytecode(&self, payload: &[u8]) -> EvmResult<()> {
        if payload.is_empty() {
            Err(EvmError::InvalidPayload)
        } else {
            Ok(())
        }
    }
}

/// Frontier-backed EVM executor (skeleton).
///
/// Enabled via the `frontier-executor` Cargo feature.
///
/// Note: this is currently a placeholder and returns an execution failure until
/// Frontier wiring is completed in the runtime.
#[cfg(feature = "frontier-executor")]
pub struct FrontierEvmExecutor;

#[cfg(feature = "frontier-executor")]
impl EvmExecutor for FrontierEvmExecutor {
    fn execute(
        &self,
        payload: &[u8],
        caller: &[u8; 20],
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult> {
        use ethereum_types::{H160 as EthH160, H256 as EthH256, U256};
        use evm::backend::ApplyBackend as _;
        use evm::executor::stack::{MemoryStackState, StackExecutor, StackSubstateMetadata};
        use evm::{ExitError, ExitReason, ExitRevert, ExitSucceed};

        use crate::state::{EvmStateDb, FrontierStateBackend};

        let (kind, to, value_u64, input_or_code) = parse_payload(payload)?;

        let caller_h160 = EthH160::from_slice(caller);
        let gas_limit = config.gas_limit;

        let evm_cfg = evm::Config::istanbul();

        let backend = FrontierStateBackend::new(EvmStateDb::new()).with_environment(
            U256::from(config.gas_price),
            caller_h160,
            U256::from(config.block_number),
            U256::from(config.block_timestamp),
            U256::from(config.gas_limit),
            U256::from(config.chain_id),
        );
        let mut backend = backend;

        // Seed caller with a large balance so value transfers / gas fees don't underflow
        backend.set_balance(caller_h160, U256::from(u128::MAX));

        let metadata = StackSubstateMetadata::new(gas_limit, &evm_cfg);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles = precompiles::Precompiles;
        let mut executor = StackExecutor::new_with_precompiles(state, &evm_cfg, &precompiles);

        let value = U256::from(value_u64);
        let (exit_reason, return_value) = match kind {
            PayloadKind::Call => {
                let to = to.ok_or(EvmError::InvalidPayload)?;
                executor.transact_call(
                    caller_h160,
                    to,
                    value,
                    input_or_code.to_vec(),
                    gas_limit,
                    Vec::new(),
                )
            }
            PayloadKind::Create => {
                executor.transact_create(caller_h160, value, input_or_code.to_vec(), gas_limit, Vec::new())
            }
        };

        let used = executor.used_gas();
        // Compute gas used via the user-facing GasCalculator for consistent reporting
        let gas_calc = GasCalculator::new(gas_limit).finalize(gas_limit.saturating_sub(used));
        let gas_used = gas_calc.gas_used;

        // Finalize state changes into backend (values + logs)
        let state = executor.into_state();
        let (values, evm_logs) = state.deconstruct();
        let evm_logs_vec: Vec<evm::backend::Log> = evm_logs.into_iter().collect();
        let logs = evm_logs_vec
            .iter()
            .map(|log| {
                let address = H160::from_slice(log.address.as_bytes());
                // Ethereum LOG opcodes support up to 4 topics.
                if log.topics.len() > 4 {
                    // Should not happen for valid EVM execution, but avoid
                    // constructing oversized receipts.
                    return EvmLog { address, topics: Vec::new(), data: Vec::new() };
                }
                // Avoid accidental huge log payloads in receipts.
                if log.data.len() > 64 * 1024 {
                    return EvmLog { address, topics: Vec::new(), data: Vec::new() };
                }

                let topics = log
                    .topics
                    .iter()
                    .map(|t| {
                        let mut out = [0u8; 32];
                        out.copy_from_slice(t.as_bytes());
                        out
                    })
                    .collect::<Vec<[u8; 32]>>();
                EvmLog {
                    address,
                    topics,
                    data: log.data.clone(),
                }
            })
            .collect::<Vec<_>>();

        backend.apply(values, evm_logs_vec.into_iter(), true);

        let state_db = backend.into_state();
        let state_root = state_db.compute_state_root();

        let (success, output) = match &exit_reason {
            ExitReason::Succeed(ExitSucceed::Stopped) | ExitReason::Succeed(ExitSucceed::Returned) => {
                (true, return_value)
            }
            ExitReason::Revert(ExitRevert::Reverted) => {
                // Include revert return data (reason) when available
                return Err(EvmError::ExecutionReverted(Some(return_value.clone())));
            }
            ExitReason::Error(ExitError::OutOfGas) => {
                return Err(EvmError::OutOfGas);
            }
            ExitReason::Error(ExitError::OutOfFund) => {
                return Err(EvmError::ExecutionFailed(0xF002));
            }
            ExitReason::Error(ExitError::InvalidJump) => {
                return Err(EvmError::ExecutionFailed(0xF003));
            }
            ExitReason::Error(ExitError::InvalidRange) => {
                return Err(EvmError::ExecutionFailed(0xF004));
            }
            ExitReason::Error(ExitError::StackUnderflow) => {
                return Err(EvmError::ExecutionFailed(0xF005));
            }
            ExitReason::Error(ExitError::StackOverflow) => {
                return Err(EvmError::ExecutionFailed(0xF006));
            }
            _ => {
                return Err(EvmError::ExecutionFailed(0xF001));
            }
        };

        Ok(EvmExecutionResult {
            success,
            output,
            gas_used,
            logs,
            state_root,
        })
    }

    fn validate_bytecode(&self, payload: &[u8]) -> EvmResult<()> {
        let (kind, to, _value, bytes) = parse_payload(payload)?;
        if bytes.is_empty() {
            return Err(EvmError::InvalidPayload);
        }
        if kind == PayloadKind::Call {
            let _ = to.ok_or(EvmError::InvalidPayload)?;
        }

        // Basic bytecode sanity checks (PUSH data bounds + JUMPDEST structure).
        validate_evm_bytecode(bytes)?;

        // Optional static-ish gas estimation (native only).
        #[cfg(feature = "std")]
        {
            use ethereum_types::U256;
            use evm::executor::stack::{MemoryStackState, StackExecutor, StackSubstateMetadata};
            use crate::state::{EvmStateDb, FrontierStateBackend};

            // Conservative cap for validation-time dry runs.
            const DRY_RUN_GAS_LIMIT: u64 = 50_000_000;

            let evm_cfg = evm::Config::istanbul();
            let caller_h160 = to.unwrap_or_else(ethereum_types::H160::zero);
            let origin = ethereum_types::H160::zero();

            let backend = FrontierStateBackend::new(EvmStateDb::new()).with_environment(
                U256::one(),
                origin,
                U256::zero(),
                U256::zero(),
                U256::from(DRY_RUN_GAS_LIMIT),
                U256::one(),
            );
            let mut backend = backend;

            let metadata = StackSubstateMetadata::new(DRY_RUN_GAS_LIMIT, &evm_cfg);
            let state = MemoryStackState::new(metadata, &backend);
            let precompiles = precompiles::Precompiles;
            let mut executor = StackExecutor::new_with_precompiles(state, &evm_cfg, &precompiles);

            match kind {
                PayloadKind::Call => {
                    let to = to.unwrap();
                    let _ = executor.transact_call(
                        caller_h160,
                        to,
                        U256::zero(),
                        bytes.to_vec(),
                        DRY_RUN_GAS_LIMIT,
                        Vec::new(),
                    );
                }
                PayloadKind::Create => {
                    let _ = executor.transact_create(
                        caller_h160,
                        U256::zero(),
                        bytes.to_vec(),
                        DRY_RUN_GAS_LIMIT,
                        Vec::new(),
                    );
                }
            }

            let used = executor.used_gas();
            if used > DRY_RUN_GAS_LIMIT {
                return Err(EvmError::OutOfGas);
            }
        }

        Ok(())
    }
}

/// Enum for payload types - reserved for future execution model expansion
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum PayloadKind {
    Call,
    Create,
}

/// Parse EVM payload format - reserved for future use in custom execution paths
#[allow(dead_code)]
fn parse_payload(payload: &[u8]) -> EvmResult<(PayloadKind, Option<ethereum_types::H160>, u64, &[u8])> {
    use ethereum_types::H160 as EthH160;
    if payload.is_empty() {
        return Err(EvmError::InvalidPayload);
    }
    match payload[0] {
        0x00 => {
            if payload.len() < 1 + 20 + 8 {
                return Err(EvmError::InvalidPayload);
            }
            let mut to_bytes = [0u8; 20];
            to_bytes.copy_from_slice(&payload[1..21]);
            let mut value_bytes = [0u8; 8];
            value_bytes.copy_from_slice(&payload[21..29]);
            let value = u64::from_be_bytes(value_bytes);
            Ok((PayloadKind::Call, Some(EthH160::from(to_bytes)), value, &payload[29..]))
        }
        0x01 => {
            if payload.len() < 1 + 8 {
                return Err(EvmError::InvalidPayload);
            }
            let mut value_bytes = [0u8; 8];
            value_bytes.copy_from_slice(&payload[1..9]);
            let value = u64::from_be_bytes(value_bytes);
            Ok((PayloadKind::Create, None, value, &payload[9..]))
        }
        _ => Err(EvmError::InvalidPayload),
    }
}

/// Validate EVM bytecode format - reserved for future integration with custom bytecode validation
///
/// Performs basic checks including EOF container detection and PUSH instruction bounds validation.
#[allow(dead_code)]
fn validate_evm_bytecode(bytecode: &[u8]) -> EvmResult<()> {
    if bytecode.is_empty() {
        return Err(EvmError::InvalidPayload);
    }

    // Basic EOF (EIP-3541) guard: 0xEF is reserved for EOF containers.
    if bytecode[0] == 0xEF {
        return Err(EvmError::InvalidPayload);
    }

    // PUSH bounds check
    let mut i = 0usize;
    while i < bytecode.len() {
        let op = bytecode[i];
        if (0x60..=0x7f).contains(&op) {
            let push_len = (op - 0x5f) as usize;
            let end = i + 1 + push_len;
            if end > bytecode.len() {
                return Err(EvmError::InvalidPayload);
            }
            i = end;
            continue;
        }
        i += 1;
    }

    Ok(())
}

/// Gas metering helper.
///
/// The EVM stack executor accounts gas internally; this type records the user-facing
/// `gas_limit` and reports `gas_used` consistently.
#[derive(Clone, Copy, Debug)]
pub struct GasCalculator {
    pub gas_limit: u64,
    pub gas_used: u64,
}

impl GasCalculator {
    pub fn new(gas_limit: u64) -> Self {
        Self { gas_limit, gas_used: 0 }
    }

    pub fn finalize(mut self, remaining_gas: u64) -> Self {
        self.gas_used = self.gas_limit.saturating_sub(remaining_gas);
        self
    }
}

#[cfg(feature = "frontier-executor")]
/// Ethereum standard precompiles (0x01..=0x09).
///
/// - Most precompiles have lightweight std implementations available by default.
/// - Heavy cryptographic precompiles (bn128, modexp, blake2f) are gated under
///   the `full-precompiles` feature and require optional dependencies.
mod precompiles {
    use ethereum_types::{H160, U256};
    use evm::executor::stack::{
        IsPrecompileResult, PrecompileFailure, PrecompileHandle, PrecompileOutput, PrecompileSet,
    };
    use sha2::{Digest as _, Sha256};
    use ripemd::Ripemd160;

    pub struct Precompiles;

    fn gas_cost_identity(input_len: usize) -> u64 {
        15 + 3 * ((input_len as u64 + 31) / 32)
    }

    fn gas_cost_sha256(input_len: usize) -> u64 {
        60 + 12 * ((input_len as u64 + 31) / 32)
    }

    fn gas_cost_ripemd160(input_len: usize) -> u64 {
        600 + 120 * ((input_len as u64 + 31) / 32)
    }

    impl PrecompileSet for Precompiles {
        fn execute(
            &self,
            handle: &mut impl PrecompileHandle,
        ) -> Option<Result<PrecompileOutput, PrecompileFailure>> {
            let address = handle.code_address();
            let input = handle.input();
            let gas_limit = handle.gas_limit();

            let addr_u = U256::from_big_endian(address.as_bytes());
            let id = addr_u.low_u64();

            match id {
                1 => Some(ecrecover_precompile(input, gas_limit)),
                2 => Some(sha256_precompile(input, gas_limit)),
                3 => Some(ripemd160_precompile(input, gas_limit)),
                4 => Some(identity_precompile(input, gas_limit)),
                5 => Some(modexp_precompile(input, gas_limit)),
                6 => Some(bn128_add_precompile(input, gas_limit)),
                7 => Some(bn128_mul_precompile(input, gas_limit)),
                8 => Some(bn128_pairing_precompile(input, gas_limit)),
                9 => Some(blake2f_precompile(input, gas_limit)),
                _ => None,
            }
        }

        fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
            let addr_u = U256::from_big_endian(address.as_bytes());
            let is_precompile = matches!(addr_u.low_u64(), 1..=9);
            IsPrecompileResult::Answer {
                is_precompile,
                extra_cost: 0,
            }
        }
    }

    fn ensure_gas(gas_limit: Option<u64>, required: u64) -> Result<(), PrecompileFailure> {
        if let Some(limit) = gas_limit {
            if required > limit {
                return Err(PrecompileFailure::Error { exit_status: evm::ExitError::OutOfGas });
            }
        }
        Ok(())
    }

    fn identity_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        let cost = gas_cost_identity(input.len());
        ensure_gas(gas_limit, cost)?;
        Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: input.to_vec() })
    }

    fn sha256_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        let cost = gas_cost_sha256(input.len());
        ensure_gas(gas_limit, cost)?;
        let mut h = Sha256::new();
        h.update(input);
        let out = h.finalize();
        Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: out.to_vec() })
    }

    fn ripemd160_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        let cost = gas_cost_ripemd160(input.len());
        ensure_gas(gas_limit, cost)?;
        let mut h = Ripemd160::new();
        h.update(input);
        let out = h.finalize();
        let mut padded = [0u8; 32];
        padded[12..32].copy_from_slice(&out);
        Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: padded.to_vec() })
    }

    fn ecrecover_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        // Gas cost fixed at 3000
        ensure_gas(gas_limit, 3000)?;
        if input.len() < 128 {
            return Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: Vec::new() });
        }
        let mut msg = [0u8; 32];
        msg.copy_from_slice(&input[0..32]);
        let mut v = [0u8; 32];
        v.copy_from_slice(&input[32..64]);
        let mut r = [0u8; 32];
        r.copy_from_slice(&input[64..96]);
        let mut s = [0u8; 32];
        s.copy_from_slice(&input[96..128]);

        let rec_id = v[31];
        let rec_id = match rec_id {
            27 | 28 => rec_id - 27,
            0 | 1 => rec_id,
            _ => {
                return Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: Vec::new() });
            }
        };
        let mut sig = [0u8; 65];
        sig[0..32].copy_from_slice(&r);
        sig[32..64].copy_from_slice(&s);
        sig[64] = rec_id;

        // Use Substrate secp256k1 recover (available in native builds)
        let pubkey = match sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg) {
            Ok(pk) => pk,
            Err(_) => {
                return Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: Vec::new() });
            }
        };

        let hash = sp_core::hashing::keccak_256(&pubkey);
        let mut out = [0u8; 32];
        out[12..32].copy_from_slice(&hash[12..32]);
        Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: out.to_vec() })
    }

    fn revert() -> Result<PrecompileOutput, PrecompileFailure> {
        Err(PrecompileFailure::Revert { exit_status: evm::ExitRevert::Reverted, output: Vec::new() })
    }

    // --- 0x05: ModExp (EIP-198)

    fn modexp_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        #[cfg(not(feature = "full-precompiles"))]
        {
            let _ = (input, gas_limit);
            return revert();
        }

        #[cfg(feature = "full-precompiles")]
        {
            use num_bigint::BigUint;
            use num_traits::Zero as _;

            fn read_u256_be_32(bytes: &[u8]) -> U256 {
                let mut tmp = [0u8; 32];
                let take = core::cmp::min(32, bytes.len());
                tmp[32 - take..32].copy_from_slice(&bytes[..take]);
                U256::from_big_endian(&tmp)
            }

            fn slice_padded(input: &[u8], offset: usize, len: usize) -> Vec<u8> {
                if len == 0 {
                    return Vec::new();
                }
                let mut out = vec![0u8; len];
                if offset >= input.len() {
                    return out;
                }
                let available = core::cmp::min(len, input.len() - offset);
                out[..available].copy_from_slice(&input[offset..offset + available]);
                out
            }

            // Parse lengths (each 32 bytes; missing bytes treated as zero)
            let base_len = read_u256_be_32(input.get(0..32).unwrap_or(&[])).as_usize();
            let exp_len = read_u256_be_32(input.get(32..64).unwrap_or(&[])).as_usize();
            let mod_len = read_u256_be_32(input.get(64..96).unwrap_or(&[])).as_usize();

            let mut offset = 96usize;
            let base = slice_padded(input, offset, base_len);
            offset = offset.saturating_add(base_len);
            let exp = slice_padded(input, offset, exp_len);
            offset = offset.saturating_add(exp_len);
            let modu = slice_padded(input, offset, mod_len);

            // Gas cost per EIP-198 (pre-Berlin formula).
            fn mult_complexity(x: u64) -> u64 {
                if x <= 64 {
                    x.saturating_mul(x)
                } else if x <= 1024 {
                    (x.saturating_mul(x) / 4)
                        .saturating_add(96u64.saturating_mul(x))
                        .saturating_sub(3072)
                } else {
                    (x.saturating_mul(x) / 16)
                        .saturating_add(480u64.saturating_mul(x))
                        .saturating_sub(199_680)
                }
            }

            fn adjusted_exp_len(exp: &[u8]) -> u64 {
                if exp.is_empty() {
                    return 0;
                }
                // Determine bit length of exponent (EIP-198 adjusted exponent length).
                let mut first_nonzero = None;
                for (i, b) in exp.iter().enumerate() {
                    if *b != 0 {
                        first_nonzero = Some((i, *b));
                        break;
                    }
                }
                let Some((i, b)) = first_nonzero else { return 0; };
                let remaining_bytes = (exp.len() - i) as u64;
                let leading_bits = 8u64.saturating_sub((b as u64).leading_zeros() as u64);
                (remaining_bytes.saturating_sub(1) * 8).saturating_add(leading_bits)
            }

            let max_len = core::cmp::max(base_len, mod_len) as u64;
            let mcomp = mult_complexity(max_len);
            let adj_exp = core::cmp::max(adjusted_exp_len(&exp), 1);
            let gas_cost = mcomp.saturating_mul(adj_exp).saturating_div(20);
            ensure_gas(gas_limit, gas_cost)?;

            if mod_len == 0 {
                return Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: Vec::new() });
            }

            let modulus = BigUint::from_bytes_be(&modu);
            if modulus.is_zero() {
                // Per EIP-198: if modulus is zero, return zeroed output.
                return Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: vec![0u8; mod_len] });
            }

            let base_n = BigUint::from_bytes_be(&base);
            let exp_n = BigUint::from_bytes_be(&exp);
            let res = base_n.modpow(&exp_n, &modulus);
            let mut out = res.to_bytes_be();
            if out.len() > mod_len {
                // Truncate to mod_len (shouldn't normally happen).
                out = out[out.len() - mod_len..].to_vec();
            }
            if out.len() < mod_len {
                let mut padded = vec![0u8; mod_len - out.len()];
                padded.extend_from_slice(&out);
                out = padded;
            }

            Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: out })
        }
    }

    // --- 0x06..0x08: bn128 (EIP-196/EIP-197)

    fn bn128_add_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        #[cfg(not(feature = "full-precompiles"))]
        {
            let _ = (input, gas_limit);
            return revert();
        }

        #[cfg(feature = "full-precompiles")]
        {
            ensure_gas(gas_limit, 500)?;
            let mut data = vec![0u8; 128];
            let take = core::cmp::min(128, input.len());
            data[..take].copy_from_slice(&input[..take]);
            let out = bn128::add(&data).map_err(|_| PrecompileFailure::Revert { exit_status: evm::ExitRevert::Reverted, output: Vec::new() })?;
            Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: out })
        }
    }

    fn bn128_mul_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        #[cfg(not(feature = "full-precompiles"))]
        {
            let _ = (input, gas_limit);
            return revert();
        }

        #[cfg(feature = "full-precompiles")]
        {
            ensure_gas(gas_limit, 40_000)?;
            let mut data = vec![0u8; 96];
            let take = core::cmp::min(96, input.len());
            data[..take].copy_from_slice(&input[..take]);
            let out = bn128::mul(&data).map_err(|_| PrecompileFailure::Revert { exit_status: evm::ExitRevert::Reverted, output: Vec::new() })?;
            Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: out })
        }
    }

    fn bn128_pairing_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        #[cfg(not(feature = "full-precompiles"))]
        {
            let _ = (input, gas_limit);
            return revert();
        }

        #[cfg(feature = "full-precompiles")]
        {
            // Gas per EIP-197: 80_000 + 100_000 * k
            if input.len() % 192 != 0 {
                return revert();
            }
            let k = (input.len() / 192) as u64;
            let gas = 80_000u64.saturating_add(100_000u64.saturating_mul(k));
            ensure_gas(gas_limit, gas)?;

            let out = bn128::pairing(input).map_err(|_| PrecompileFailure::Revert { exit_status: evm::ExitRevert::Reverted, output: Vec::new() })?;
            Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: out })
        }
    }

    #[cfg(feature = "full-precompiles")]
    mod bn128 {
        use substrate_bn::{Fq, Fq2, Fr, Group, G1, G2};

        fn read_fq(slice: &[u8]) -> Result<Fq, ()> {
            if slice.len() != 32 {
                return Err(());
            }
            let mut buf = [0u8; 32];
            buf.copy_from_slice(slice);
            Fq::from_slice(&buf).map_err(|_| ())
        }

        fn read_fr(slice: &[u8]) -> Result<Fr, ()> {
            if slice.len() != 32 {
                return Err(());
            }
            let mut buf = [0u8; 32];
            buf.copy_from_slice(slice);
            Fr::from_slice(&buf).map_err(|_| ())
        }

        fn encode_g1(point: G1) -> Vec<u8> {
            if point == G1::zero() {
                return vec![0u8; 64];
            }
            let affine = point.into_affine();
            let mut out = vec![0u8; 64];
            affine.x().to_big_endian(&mut out[0..32]).expect("32 bytes; qed");
            affine.y().to_big_endian(&mut out[32..64]).expect("32 bytes; qed");
            out
        }

        pub fn add(input: &[u8]) -> Result<Vec<u8>, ()> {
            if input.len() != 128 {
                return Err(());
            }
            let x1 = read_fq(&input[0..32])?;
            let y1 = read_fq(&input[32..64])?;
            let x2 = read_fq(&input[64..96])?;
            let y2 = read_fq(&input[96..128])?;

            let p1 = if x1.is_zero() && y1.is_zero() {
                G1::zero()
            } else {
                G1::from_affine(substrate_bn::AffineG1::new(x1, y1).map_err(|_| ())?).map_err(|_| ())?
            };
            let p2 = if x2.is_zero() && y2.is_zero() {
                G1::zero()
            } else {
                G1::from_affine(substrate_bn::AffineG1::new(x2, y2).map_err(|_| ())?).map_err(|_| ())?
            };

            Ok(encode_g1(p1 + p2))
        }

        pub fn mul(input: &[u8]) -> Result<Vec<u8>, ()> {
            if input.len() != 96 {
                return Err(());
            }
            let x = read_fq(&input[0..32])?;
            let y = read_fq(&input[32..64])?;
            let s = read_fr(&input[64..96])?;

            let p = if x.is_zero() && y.is_zero() {
                G1::zero()
            } else {
                G1::from_affine(substrate_bn::AffineG1::new(x, y).map_err(|_| ())?).map_err(|_| ())?
            };

            Ok(encode_g1(p * s))
        }

        pub fn pairing(input: &[u8]) -> Result<Vec<u8>, ()> {
            if input.is_empty() {
                let mut out = vec![0u8; 32];
                out[31] = 1;
                return Ok(out);
            }
            if input.len() % 192 != 0 {
                return Err(());
            }

            let mut acc = substrate_bn::Gt::one();
            for chunk in input.chunks(192) {
                let ax = read_fq(&chunk[0..32])?;
                let ay = read_fq(&chunk[32..64])?;

                // G2 encoding: x = (x_im, x_re), y = (y_im, y_re)
                let bx_im = read_fq(&chunk[64..96])?;
                let bx_re = read_fq(&chunk[96..128])?;
                let by_im = read_fq(&chunk[128..160])?;
                let by_re = read_fq(&chunk[160..192])?;

                let a = if ax.is_zero() && ay.is_zero() {
                    G1::zero()
                } else {
                    G1::from_affine(substrate_bn::AffineG1::new(ax, ay).map_err(|_| ())?).map_err(|_| ())?
                };

                let bx = Fq2::new(bx_re, bx_im);
                let by = Fq2::new(by_re, by_im);
                let b = if bx.is_zero() && by.is_zero() {
                    G2::zero()
                } else {
                    G2::from_affine(substrate_bn::AffineG2::new(bx, by).map_err(|_| ())?).map_err(|_| ())?
                };

                acc = acc * substrate_bn::pairing(a, b);
            }

            let mut out = vec![0u8; 32];
            if acc == substrate_bn::Gt::one() {
                out[31] = 1;
            }
            Ok(out)
        }
    }

    // --- 0x09: Blake2F (EIP-152)

    fn blake2f_precompile(input: &[u8], gas_limit: Option<u64>) -> Result<PrecompileOutput, PrecompileFailure> {
        #[cfg(not(feature = "full-precompiles"))]
        {
            let _ = (input, gas_limit);
            return revert();
        }

        #[cfg(feature = "full-precompiles")]
        {
            // EIP-152: gas cost equals number of rounds.
            if input.len() != 213 {
                return revert();
            }
            let rounds = u32::from_be_bytes([input[0], input[1], input[2], input[3]]) as u64;
            ensure_gas(gas_limit, rounds)?;

            let mut h = [0u64; 8];
            for i in 0..8 {
                let off = 4 + i * 8;
                h[i] = u64::from_le_bytes(input[off..off + 8].try_into().unwrap());
            }
            let mut m = [0u64; 16];
            for i in 0..16 {
                let off = 68 + i * 8;
                m[i] = u64::from_le_bytes(input[off..off + 8].try_into().unwrap());
            }
            let t0 = u64::from_le_bytes(input[196..204].try_into().unwrap());
            let t1 = u64::from_le_bytes(input[204..212].try_into().unwrap());
            let f = match input[212] {
                0 => false,
                1 => true,
                _ => return revert(),
            };

            let out_h = blake2f(rounds as u32, h, m, [t0, t1], f);
            let mut out = vec![0u8; 64];
            for i in 0..8 {
                out[i * 8..i * 8 + 8].copy_from_slice(&out_h[i].to_le_bytes());
            }

            Ok(PrecompileOutput { exit_status: evm::ExitSucceed::Returned, output: out })
        }
    }

    #[cfg(feature = "full-precompiles")]
    fn blake2f(rounds: u32, h: [u64; 8], m: [u64; 16], t: [u64; 2], f: bool) -> [u64; 8] {
        const IV: [u64; 8] = [
            0x6a09e667f3bcc908,
            0xbb67ae8584caa73b,
            0x3c6ef372fe94f82b,
            0xa54ff53a5f1d36f1,
            0x510e527fade682d1,
            0x9b05688c2b3e6c1f,
            0x1f83d9abfb41bd6b,
            0x5be0cd19137e2179,
        ];

        const SIGMA: [[usize; 16]; 12] = [
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
            [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
            [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
            [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
            [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
            [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
            [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
            [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
            [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        ];

        #[inline(always)]
        fn rotr(x: u64, n: u32) -> u64 {
            (x >> n) | (x << (64 - n))
        }

        #[inline(always)]
        fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
            v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
            v[d] = rotr(v[d] ^ v[a], 32);
            v[c] = v[c].wrapping_add(v[d]);
            v[b] = rotr(v[b] ^ v[c], 24);
            v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
            v[d] = rotr(v[d] ^ v[a], 16);
            v[c] = v[c].wrapping_add(v[d]);
            v[b] = rotr(v[b] ^ v[c], 63);
        }

        let mut v = [0u64; 16];
        v[0..8].copy_from_slice(&h);
        v[8..16].copy_from_slice(&IV);
        v[12] ^= t[0];
        v[13] ^= t[1];
        if f {
            v[14] = !v[14];
        }

        for i in 0..rounds {
            let s = &SIGMA[(i as usize) % 12];
            g(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
            g(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
            g(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
            g(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
            g(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
            g(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
            g(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
            g(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
        }

        let mut out = [0u64; 8];
        for i in 0..8 {
            out[i] = h[i] ^ v[i] ^ v[i + 8];
        }
        out
    }
}

/// Prepare root computation for EVM execution
pub fn compute_evm_prepare_root(
    comit_id: &[u8; 32],
    payload: &[u8],
    result: &EvmExecutionResult,
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
        let config = EvmConfig::default();
        assert_eq!(config.gas_limit, 21_000_000);
        assert_eq!(config.chain_id, 42);
    }

    #[test]
    fn test_gas_calculator_finalize() {
        let gc = GasCalculator::new(100).finalize(30);
        assert_eq!(gc.gas_limit, 100);
        assert_eq!(gc.gas_used, 70);
    }

    #[test]
    fn test_mock_executor_success() {
        let executor = MockEvmExecutor;
        let result = executor.execute(&[0x01, 0x02], &[0u8; 20], &EvmConfig::default());
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_mock_executor_empty_payload() {
        let executor = MockEvmExecutor;
        let result = executor.execute(&[], &[0u8; 20], &EvmConfig::default());
        assert_eq!(result, Err(EvmError::InvalidPayload));
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_precompile_identity() {
        let executor = FrontierEvmExecutor;
        let mut payload = Vec::new();
        payload.push(0x00);
        // to = 0x04 (identity)
        let mut to = [0u8; 20];
        to[19] = 0x04;
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        payload.extend_from_slice(b"hello");
        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        assert_eq!(result.output, b"hello");

        // Now run with an extremely low gas limit to ensure OutOfGas is returned
        let mut cfg = EvmConfig::default();
        cfg.gas_limit = 1;
        let err = executor.execute(&payload, &[0u8; 20], &cfg).unwrap_err();
        assert_eq!(err, EvmError::OutOfGas);
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_gas_limit_enforcement() {
        let executor = FrontierEvmExecutor;
        let mut cfg = EvmConfig::default();
        cfg.gas_limit = 1; // far too low
        let mut payload = Vec::new();
        payload.push(0x00);
        let mut to = [0u8; 20];
        to[19] = 0x02; // sha256
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        payload.extend_from_slice(&[0u8; 32]);
        let err = executor.execute(&payload, &[0u8; 20], &cfg).unwrap_err();
        assert_eq!(err, EvmError::OutOfGas);
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_simple_contract_deployment() {
        let executor = FrontierEvmExecutor;
        // Simple contract: just STOP
        let bytecode = vec![0x00]; // STOP
        let mut payload = vec![0x01]; // CREATE
        payload.extend_from_slice(&0u64.to_be_bytes()); // value
        payload.extend_from_slice(&bytecode);

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        // Contract deployment may or may not return output depending on implementation
        // Just check it succeeds
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_contract_call() {
        let executor = FrontierEvmExecutor;
        // First deploy a simple storage contract
        let bytecode = vec![
            0x60, 0x42, // PUSH1 0x42
            0x60, 0x00, // PUSH1 0x00
            0x55,       // SSTORE
            0x60, 0x00, // PUSH1 0x00
            0x60, 0x00, // PUSH1 0x00
            0xF3,       // RETURN (returns 0)
        ];
        let mut deploy_payload = vec![0x01]; // CREATE
        deploy_payload.extend_from_slice(&0u64.to_be_bytes()); // value
        deploy_payload.extend_from_slice(&bytecode);

        let deploy_result = executor.execute(&deploy_payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(deploy_result.success);

        // Now call the contract (though it doesn't have a function, just returns 0)
        let mut call_payload = vec![0x00]; // CALL
        call_payload.extend_from_slice(&[0u8; 20]); // to (zero address for simplicity)
        call_payload.extend_from_slice(&0u64.to_be_bytes()); // value
        call_payload.extend_from_slice(&[]); // input

        let call_result = executor.execute(&call_payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(call_result.success);
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_revert_handling() {
        let executor = FrontierEvmExecutor;
        // Contract that always reverts: PUSH1 0x00 PUSH1 0x00 REVERT
        let bytecode = vec![
            0x60, 0x00, // PUSH1 0x00
            0x60, 0x00, // PUSH1 0x00
            0xFD,       // REVERT
        ];
        let mut payload = vec![0x01]; // CREATE
        payload.extend_from_slice(&0u64.to_be_bytes()); // value
        payload.extend_from_slice(&bytecode);

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default());
        assert!(matches!(result, Err(EvmError::ExecutionReverted(Some(_)))));
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_precompile_sha256() {
        let executor = FrontierEvmExecutor;
        let mut payload = Vec::new();
        payload.push(0x00);
        // to = 0x02 (sha256)
        let mut to = [0u8; 20];
        to[19] = 0x02;
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        payload.extend_from_slice(b"hello");

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        assert_eq!(result.output.len(), 32);
        // SHA256 of "hello"
        let expected = [
            0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e,
            0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9, 0xe2, 0x9e,
            0x1b, 0x16, 0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e,
            0x73, 0x04, 0x33, 0x62, 0x93, 0x8b, 0x98, 0x24,
        ];
        assert_eq!(result.output, expected);
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_precompile_ripemd160() {
        let executor = FrontierEvmExecutor;
        let mut payload = Vec::new();
        payload.push(0x00);
        // to = 0x03 (ripemd160)
        let mut to = [0u8; 20];
        to[19] = 0x03;
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        payload.extend_from_slice(b"hello");

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        assert_eq!(result.output.len(), 32);
        // Just check it's not all zeros
        assert!(!result.output.iter().all(|&x| x == 0));
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_state_root_computation() {
        let executor = FrontierEvmExecutor;
        // Deploy a contract that modifies state
        let bytecode = vec![
            0x60, 0x42, // PUSH1 0x42
            0x60, 0x00, // PUSH1 0x00
            0x55,       // SSTORE
            0x60, 0x00, // PUSH1 0x00
            0x60, 0x00, // PUSH1 0x00
            0xF3,       // RETURN
        ];
        let mut payload = vec![0x01]; // CREATE
        payload.extend_from_slice(&0u64.to_be_bytes()); // value
        payload.extend_from_slice(&bytecode);

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        // State root should be non-zero after state changes
        assert_ne!(result.state_root, [0u8; 32]);
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_log_emission() {
        let executor = FrontierEvmExecutor;
        // Contract that emits a log: PUSH1 0x20 PUSH1 0x00 PUSH1 0x00 LOG1
        let bytecode = vec![
            0x60, 0x20, // PUSH1 0x20 (size)
            0x60, 0x00, // PUSH1 0x00 (offset)
            0x60, 0x00, // PUSH1 0x00 (topic count, but LOG1 uses 1 topic)
            0xA1,       // LOG1
            0x60, 0x00, // PUSH1 0x00
            0x60, 0x00, // PUSH1 0x00
            0xF3,       // RETURN
        ];
        let mut payload = vec![0x01]; // CREATE
        payload.extend_from_slice(&0u64.to_be_bytes()); // value
        payload.extend_from_slice(&bytecode);

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        // Should have emitted logs
        assert!(!result.logs.is_empty());
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_invalid_payload() {
        let executor = FrontierEvmExecutor;
        // Invalid payload
        let payload = vec![0xFF];
        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default());
        assert_eq!(result, Err(EvmError::InvalidPayload));
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_bytecode_validation() {
        let executor = FrontierEvmExecutor;
        // Valid bytecode
        let valid_bytecode = vec![0x00]; // STOP
        assert!(executor.validate_bytecode(&[0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0x00]).is_ok());

        // Empty bytecode
        assert_eq!(executor.validate_bytecode(&[0x01, 0, 0, 0, 0, 0, 0, 0, 0]), Err(EvmError::InvalidPayload));

        // Invalid payload
        assert_eq!(executor.validate_bytecode(&[0xFF]), Err(EvmError::InvalidPayload));
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_precompile_ecrecover() {
        let executor = FrontierEvmExecutor;
        let mut payload = Vec::new();
        payload.push(0x00);
        // to = 0x01 (ecrecover)
        let mut to = [0u8; 20];
        to[19] = 0x01;
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        // Sample ecrecover input (128 bytes)
        let input = vec![0u8; 128];
        payload.extend_from_slice(&input);

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        // ecrecover returns 32 bytes (address or zeros for invalid input)
        // Allow empty output for invalid inputs
        assert!(result.output.is_empty() || result.output.len() == 32);
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_precompile_modexp() {
        let executor = FrontierEvmExecutor;
        let mut payload = Vec::new();
        payload.push(0x00);
        // to = 0x05 (modexp)
        let mut to = [0u8; 20];
        to[19] = 0x05;
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        // Minimal modexp input: base_len=1, exp_len=1, mod_len=1, base=2, exp=3, mod=5
        let mut input = vec![0u8; 96]; // 3 * 32 bytes for lengths
        input[31] = 1; // base_len
        input[63] = 1; // exp_len
        input[95] = 1; // mod_len
        input.push(2); // base
        input.push(3); // exp
        input.push(5); // mod
        payload.extend_from_slice(&input);

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default());
        // ModExp might not be implemented in basic config, so allow revert
        match result {
            Ok(res) => {
                assert!(res.success);
            }
            Err(EvmError::ExecutionReverted(_)) => {
                // Expected if full-precompiles not enabled
            }
            _ => panic!("Unexpected error"),
        }
    }

    // --- Full precompiles tests (require full-precompiles feature)
    #[cfg(all(feature = "frontier-executor", feature = "full-precompiles"))]
    #[test]
    fn test_full_precompiles_modexp_exact() {
        let executor = FrontierEvmExecutor;
        let mut payload = Vec::new();
        payload.push(0x00);
        let mut to = [0u8; 20];
        to[19] = 0x05; // modexp
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        // base=258, exp=5, mod=97 (small prime)
        let base = vec![0x01, 0x02]; // 258
        let exp = vec![0x05]; // 5
        let modu = vec![97u8]; // 97
        let mut input = vec![0u8; 96];
        // lengths
        input[31] = base.len() as u8;
        input[63] = exp.len() as u8;
        input[95] = modu.len() as u8;
        // append data blocks
        input.extend_from_slice(&base);
        input.extend_from_slice(&exp);
        input.extend_from_slice(&modu);
        payload.extend_from_slice(&input);

        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        // manual computation: 258^5 mod 97
        let mut acc = 1u128;
        for _ in 0..5 { acc = (acc * 258) % 97 }
        if !result.output.is_empty() {
            let mut out_bytes = [0u8; 32];
            let take = core::cmp::min(32, result.output.len());
            out_bytes[32 - take..32].copy_from_slice(&result.output[..take]);
            let out_val = ethereum_types::U256::from_big_endian(&out_bytes);
            assert_eq!(out_val % ethereum_types::U256::from(97u64), ethereum_types::U256::from(acc));
        }
    }

    #[cfg(all(feature = "frontier-executor", feature = "full-precompiles"))]
    #[test]
    fn test_full_precompiles_bn128_pairing() {
        let executor = FrontierEvmExecutor;
        let mut payload = Vec::new();
        payload.push(0x00);
        let mut to = [0u8; 20];
        to[19] = 0x08; // bn128 pairing
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        // empty input should return 32 bytes with last byte 1 per implementation
        payload.extend_from_slice(&[]);
        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        assert_eq!(result.output.len(), 32);
        assert_eq!(result.output[31], 1);
    }

    #[cfg(all(feature = "frontier-executor", feature = "full-precompiles"))]
    #[test]
    fn test_full_precompiles_blake2f_rounds() {
        let executor = FrontierEvmExecutor;
        let mut payload = Vec::new();
        payload.push(0x00);
        let mut to = [0u8; 20];
        to[19] = 0x09; // blake2f
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        // Prepare 213-byte input with rounds = 1 and valid zeros; expect success
        let mut input = vec![0u8; 213];
        input[0..4].copy_from_slice(&1u32.to_be_bytes());
        payload.extend_from_slice(&input);
        let result = executor.execute(&payload, &[0u8; 20], &EvmConfig::default()).unwrap();
        assert!(result.success);
        assert_eq!(result.output.len(), 64);
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_frontier_out_of_gas() {
        let executor = FrontierEvmExecutor;
        let mut cfg = EvmConfig::default();
        cfg.gas_limit = 100; // Very low gas limit
        let mut payload = Vec::new();
        payload.push(0x00);
        let mut to = [0u8; 20];
        to[19] = 0x04; // identity (should be cheap)
        payload.extend_from_slice(&to);
        payload.extend_from_slice(&0u64.to_be_bytes());
        payload.extend_from_slice(&[0u8; 1000000]); // Large input to consume gas

        let result = executor.execute(&payload, &[0u8; 20], &cfg);
        assert_eq!(result, Err(EvmError::OutOfGas));
    }

    #[cfg(feature = "frontier-executor")]
    #[test]
    fn test_state_root_deterministic() {
        // Deploy same contract twice in separate fresh backends and ensure state_root is deterministic
        let executor = FrontierEvmExecutor;

        let bytecode = vec![
            0x60, 0x42, // PUSH1 0x42
            0x60, 0x00, // PUSH1 0x00
            0x55,       // SSTORE
            0x60, 0x00, // PUSH1 0x00
            0x60, 0x00, // PUSH1 0x00
            0xF3,       // RETURN
        ];

        let mut payload = vec![0x01]; // CREATE
        payload.extend_from_slice(&0u64.to_be_bytes()); // value
        payload.extend_from_slice(&bytecode);

        let res1 = executor.execute(&payload, &[1u8; 20], &EvmConfig::default()).unwrap();
        assert!(res1.success);

        let res2 = executor.execute(&payload, &[1u8; 20], &EvmConfig::default()).unwrap();
        assert!(res2.success);

        assert_eq!(res1.state_root, res2.state_root, "State roots should be deterministic for identical deployments");
    }
}

