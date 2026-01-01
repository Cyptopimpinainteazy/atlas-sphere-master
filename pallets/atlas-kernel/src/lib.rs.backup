#![cfg_attr(not(feature = "std"), no_std)]

//! # Atlas Kernel Pallet
//!
//! The core orchestration layer for Atlas Sphere's dual-VM execution architecture.
//! Enables atomic cross-VM transactions (Comits) that execute on both EVM and SVM.
//!
//! ## Security Design Decisions
//!
//! ### H-1: prepare_root Verification (Input Commitment Design)
//!
//! The `prepare_root` field is a cryptographic commitment to the **input parameters** of a Comit,
//! NOT the execution outputs. This is intentional:
//!
//! - **Rationale**: Clients must compute `prepare_root` before submission. If it committed to
//!   outputs, clients couldn't know the hash until after execution (circular dependency).
//! - **Security**: The prepare_root ensures the submitted Comit matches what the client intended.
//!   It prevents parameter tampering but does NOT guarantee execution results.
//! - **Enhancement**: For high-value transactions requiring output verification, consider adding
//!   an optional `expected_output_hash` field in future versions.
//!
//! ### H-5: VM Adapter Production Status
//!
//! The pallet uses pluggable VM adapters (`T::EvmAdapter`, `T::SvmAdapter`) configured at runtime:
//!
//! - **Test Runtime**: Uses `MockEvmAdapter` and `MockSvmAdapter` for deterministic testing
//! - **Production Runtime**: Should use `FrontierEvmAdapter` and `RbpfSvmAdapter`
//!
//! **IMPORTANT**: Before mainnet deployment, verify runtime configuration uses real adapters.
//! The `adapters.rs` module includes `FrontierEvmAdapter` which wraps pallet-evm, but runtime
//! must be properly configured to use it instead of mocks.

pub use pallet::*;

/// Phase 1: Full Consensus Implementation
/// Authority set management, pending changes scheduling, and enactment mechanism
pub mod authority;

/// VM Execution Adapters
/// Provides EvmExecutorAdapter and SvmExecutorAdapter traits for runtime configuration.
///
/// **H-5 Note**: For production, configure runtime with `FrontierEvmAdapter` and `RbpfSvmAdapter`
/// instead of mock adapters. Mock adapters are for testing only.
pub mod adapters;
pub use adapters::{
    EvmExecutorAdapter, FailingMockEvmAdapter, FailingMockSvmAdapter, FailingMockX3Adapter,
    MockEvmAdapter, MockSvmAdapter, MockX3Adapter, SvmExecutorAdapter, X3ExecutorAdapter,
};

// Re-export real adapters for std builds (native runtime)
#[cfg(feature = "std")]
pub use adapters::real_adapters::{FrontierEvmAdapter, RbpfSvmAdapter, X3VmAdapter};

/// Benchmarking support for weight generation.
/// Enable with `--features runtime-benchmarks`.
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

/// Auto-generated weight information for extrinsics.
/// Regenerate using frame-benchmarking CLI.
pub mod weights;

/// Runtime storage migrations.
pub mod migrations;
pub use weights::WeightInfo;

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::{AtLeast32BitUnsigned, CheckedAdd, SaturatedConversion};
use frame_support::sp_runtime::DispatchError;
use frame_support::traits::{Currency, UnixTime};
use frame_system::pallet_prelude::*;
use parity_scale_codec::Codec;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

/// Represents a Comit transaction submitted to the Atlas Kernel.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(AccountId, Balance))]
pub struct Comit<AccountId, Balance> {
    /// Globally unique Comit identifier.
    pub comit_id: H256,
    /// Origin account that submitted the Comit.
    pub origin: AccountId,
    /// Payload destined for the EVM execution environment.
    pub evm_payload: Vec<u8>,
    /// Payload destined for the SVM execution environment.
    pub svm_payload: Vec<u8>,
    /// Sequential nonce scoped to the origin account.
    pub nonce: u64,
    /// Fee charged for processing the Comit.
    pub fee: Balance,
    /// Dual-VM prepare phase commitment root.
    pub prepare_root: H256,
}

/// Version 2 Comit supporting triple-VM execution (EVM + SVM + X3VM).
///
/// This is intentionally a separate type from `Comit` to avoid breaking
/// downstream code that relies on the original dual-VM shape.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(AccountId, Balance))]
pub struct ComitV2<AccountId, Balance> {
    /// Globally unique Comit identifier.
    pub comit_id: H256,
    /// Origin account that submitted the Comit.
    pub origin: AccountId,
    /// Payload destined for the EVM execution environment.
    pub evm_payload: Vec<u8>,
    /// Payload destined for the SVM execution environment.
    pub svm_payload: Vec<u8>,
    /// Payload destined for the X3VM execution environment.
    pub x3_payload: Vec<u8>,
    /// Sequential nonce scoped to the origin account.
    pub nonce: u64,
    /// Fee charged for processing the Comit.
    pub fee: Balance,
    /// Multi-VM prepare phase commitment root.
    pub prepare_root: H256,
}

/// Execution receipt returned by VM runtimes after transaction execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ExecutionReceipt {
    /// Whether the execution was successful.
    pub success: bool,
    /// Gas used during execution.
    pub gas_used: u64,
    /// Return data from the execution.
    pub return_data: Vec<u8>,
    /// Logs emitted during execution.
    pub logs: Vec<ExecutionLog>,
    /// State changes resulting from execution.
    pub state_changes: Vec<StateChange>,
}

/// Log entry emitted during VM execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ExecutionLog {
    /// Address (EVM H160 or SVM 32-byte key) that emitted the log.
    pub address: Vec<u8>,
    /// Topics for the log entry.
    pub topics: Vec<H256>,
    /// Log data.
    pub data: Vec<u8>,
}

/// State change resulting from VM execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StateChange {
    /// Account/contract address affected (EVM H160 or SVM 32-byte key).
    pub address: Vec<u8>,
    /// Storage slot key.
    pub key: H256,
    /// New value at the storage slot.
    pub value: H256,
}

/// Unified state representation for the Atlas Sphere.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
pub struct SphereState {
    /// State root hash representing the entire sphere state.
    pub state_root: H256,
    /// Block number when this state was computed.
    pub block_number: u32,
    /// Timestamp of state computation.
    pub timestamp: u64,
}

/// Dual-VM transaction types that can be executed.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum VmTransaction {
    /// EVM transaction payload.
    Evm(Vec<u8>),
    /// SVM transaction payload.
    Svm(Vec<u8>),
}

/// Reasons describing why a Comit failed verification or execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
/// Granular error codes for comit execution failures with diagnostic context.
/// Each variant includes an error code and optional diagnostic message (max 256 bytes).
pub enum ComitFailureReason {
    /// The provided EVM payload exceeds runtime defined limits.
    /// Error Code: 0x01
    EvmPayloadTooLarge {
        code: u32,
        actual_size: u32,
        max_size: u32,
    },
    /// The provided SVM payload exceeds runtime defined limits.
    /// Error Code: 0x02
    SvmPayloadTooLarge {
        code: u32,
        actual_size: u32,
        max_size: u32,
    },
    /// The provided X3 payload exceeds runtime defined limits.
    /// Error Code: 0x07
    X3PayloadTooLarge {
        code: u32,
        actual_size: u32,
        max_size: u32,
    },
    /// Combined payloads exceed the cumulative limit.
    /// Error Code: 0x03
    CombinedPayloadTooLarge {
        code: u32,
        evm_size: u32,
        svm_size: u32,
        max_combined: u32,
    },
    /// Both payloads were empty, leaving nothing to execute.
    /// Error Code: 0x04
    EmptyPayloads { code: u32 },
    /// The supplied nonce was not the one expected by the pallet.
    /// Error Code: 0x05
    InvalidNonce {
        code: u32,
        expected: u64,
        provided: u64,
    },
    /// Prepare-root verification failed or receipts mismatched.
    /// Error Code: 0x06
    Verification {
        code: u32,
        reason: [u8; 32], // Hash of verification failure reason
    },
    /// EVM execution failed with error code.
    /// Error Code: 0x10
    EvmExecutionFailed {
        code: u32,
        evm_error: u32,
        gas_used: u64,
    },
    /// SVM execution failed with error code.
    /// Error Code: 0x11
    SvmExecutionFailed {
        code: u32,
        svm_error: u32,
        compute_units_used: u64,
    },
    /// X3 execution failed with error code.
    /// Error Code: 0x12
    X3ExecutionFailed {
        code: u32,
        x3_error: u32,
        gas_used: u64,
    },
}

type ComitOf<T> = Comit<<T as frame_system::Config>::AccountId, <T as Config>::Balance>;
type ComitV2Of<T> = ComitV2<<T as frame_system::Config>::AccountId, <T as Config>::Balance>;

/// Dual-VM Dispatcher trait for coordinating execution across EVM and SVM runtimes.
/// This trait defines the interface for executing transactions on both virtual machines
/// and merging their execution results into a unified Sphere State Tree.
pub trait DualVmDispatcher {
    /// AccountId type for authorization checks
    type AccountId;
    /// Balance type for fee accounting
    type Balance;

    /// Execute a transaction on the EVM runtime.
    /// Returns an execution receipt with the results of the transaction.
    fn execute_evm_tx(&self, tx: Vec<u8>) -> Result<ExecutionReceipt, DispatchError>;

    /// Execute a transaction on the SVM runtime.
    /// Returns an execution receipt with the results of the transaction.
    fn execute_svm_tx(&self, tx: Vec<u8>) -> Result<ExecutionReceipt, DispatchError>;

    /// Execute a dual-VM transaction and merge the results.
    /// This is the primary entry point for Comit execution.
    fn execute_dual_tx(
        &self,
        evm_tx: Option<Vec<u8>>,
        svm_tx: Option<Vec<u8>>,
    ) -> Result<SphereState, DispatchError>;

    /// Merge execution receipts from both VMs into a unified state.
    fn merge_receipts(
        &self,
        evm_receipt: Option<&ExecutionReceipt>,
        svm_receipt: Option<&ExecutionReceipt>,
    ) -> SphereState;

    /// Check if an account is authorized to execute a specific cross-VM operation.
    /// This enables granular access control beyond simple origin validation.
    /// Returns Ok(()) if authorized, Err(DispatchError) if not.
    fn auth_check(&self, caller: &Self::AccountId, operation: &[u8]) -> Result<(), DispatchError>;

    /// Calculate execution fees for a comit based on gas/compute usage.
    /// Takes the gas used (EVM) and compute units (SVM) and returns the total fee.
    /// This enables accurate fee accounting across heterogeneous runtimes.
    fn fee_accounting(
        &self,
        evm_gas_used: u64,
        svm_compute_units: u64,
        base_fee: Self::Balance,
    ) -> Result<Self::Balance, DispatchError>;

    /// Update the canonical ledger with state changes from a successful comit.
    /// This persists cross-VM state into the canonical view, enabling future queries.
    /// Returns Ok(()) on success or Err with diagnostics on failure.
    fn canonical_ledger_update(
        &self,
        comit_id: H256,
        state_changes: &[StateChange],
    ) -> Result<(), DispatchError>;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        /// Aggregated runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency trait for fee deduction and balance management.
        type Currency: frame_support::traits::ReservableCurrency<Self::AccountId>;

        /// Balance type used within the canonical ledger (same as Currency::Balance).
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaxEncodedLen
            + CheckedAdd
            + From<<Self::Currency as frame_support::traits::Currency<Self::AccountId>>::Balance>
            + Into<<Self::Currency as frame_support::traits::Currency<Self::AccountId>>::Balance>;

        /// Identifier type for registered assets.
        type AssetId: Parameter + Member + Ord + Default + Copy + MaxEncodedLen;

        /// Identifier type used to map substrate accounts to Atlas IDs.
        type AtlasId: Parameter + Member + Default + Copy + MaxEncodedLen;

        /// Maximum number of unique assets tracked per account in the canonical ledger.
        #[pallet::constant]
        type MaxAssetsPerAccount: Get<u32>;

        /// Maximum length allowed for asset symbols.
        #[pallet::constant]
        type MaxAssetSymbolLength: Get<u32>;

        /// Maximum length allowed for EVM payloads.
        #[pallet::constant]
        type MaxEvmPayloadLength: Get<u32>;

        /// Maximum length allowed for SVM payloads.
        #[pallet::constant]
        type MaxSvmPayloadLength: Get<u32>;

        /// Maximum length allowed for X3 payloads.
        #[pallet::constant]
        type MaxX3PayloadLength: Get<u32>;

        /// Maximum combined length of both EVM and SVM payloads.
        #[pallet::constant]
        type MaxCombinedPayloadLength: Get<u32>;

        /// Maximum combined length of EVM + SVM + X3 payloads (v2 Comits).
        #[pallet::constant]
        type MaxCombinedPayloadLengthV2: Get<u32>;

        /// Maximum number of authorities allowed in the authority set.
        #[pallet::constant]
        type MaxAuthorities: Get<u32>;

        /// Minimum number of authorities required in the authority set.
        #[pallet::constant]
        type MinAuthorities: Get<u32>;

        /// Default gas limit for EVM execution.
        #[pallet::constant]
        type DefaultEvmGasLimit: Get<u64>;

        /// Default compute unit limit for SVM execution.
        #[pallet::constant]
        type DefaultSvmComputeLimit: Get<u64>;

        /// Default gas limit for X3VM execution.
        #[pallet::constant]
        type DefaultX3GasLimit: Get<u64>;

        /// Weight information provider for extrinsics.
        type WeightInfo: WeightInfo;

        /// EVM execution adapter (runtime-configurable)
        /// Implement EvmExecutorAdapter trait for real Frontier integration
        type EvmAdapter: EvmExecutorAdapter;

        /// SVM execution adapter (runtime-configurable)
        /// Implement SvmExecutorAdapter trait for real solana-rbpf integration
        type SvmAdapter: SvmExecutorAdapter;

        /// X3 VM execution adapter (runtime-configurable)
        /// Implement X3ExecutorAdapter trait for X3 bytecode execution
        type X3Adapter: X3ExecutorAdapter;

        /// Origin that can execute privileged governance functions.
        /// Typically EnsureRoot or a council-based origin.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    type AssetSymbolOf<T> = BoundedVec<u8, <T as Config>::MaxAssetSymbolLength>;
    type AssetMetadataOf<T> = AssetMetadata<AssetSymbolOf<T>>;

    /// Canonical ledger mapping (account, asset_id) -> balance.
    /// Uses a double-storage map for efficient access without requiring nested collections.
    #[pallet::storage]
    pub type CanonicalLedger<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        T::AssetId,
        T::Balance,
        ValueQuery,
    >;

    /// Maps accounts to their Atlas identifiers.
    #[pallet::storage]
    pub type AccountRegistry<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::AtlasId>;

    /// Registry of known assets and their metadata.
    #[pallet::storage]
    pub type AssetRegistry<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, AssetMetadataOf<T>>;

    /// Nonce tracker for Comit submissions by account.
    #[pallet::storage]
    pub type Nonces<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// Accounts authorized to submit Comits.
    ///
    /// Security: If AuthorizedAccounts is empty, all submissions are rejected (secure by default).
    /// Accounts must be explicitly authorized via `authorize_account` extrinsic.
    /// In development mode with `dev-bypass` feature enabled, authorization checks are bypassed.
    #[pallet::storage]
    pub type AuthorizedAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (), ValueQuery>;

    /// Current authority set (consensus validators).
    /// Authorities are responsible for block production and finalization.
    #[pallet::storage]
    pub type Authorities<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxAuthorities>, ValueQuery>;

    /// Pending authority changes to be enacted at the next session.
    /// Changes are scheduled via governance and enacted at session boundaries.
    #[pallet::storage]
    pub type PendingAuthorities<T: Config> =
        StorageValue<_, Option<BoundedVec<T::AccountId, T::MaxAuthorities>>, ValueQuery>;

    /// Tracks submitted comit_ids to prevent duplicate submissions.
    /// Value is the block number when the comit was submitted.
    #[pallet::storage]
    pub type SubmittedComits<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>, OptionQuery>;

    /// Rate limiting: tracks Comit submissions per account per block.
    /// Key: (AccountId, BlockNumber), Value: submission count.
    /// Used to prevent DoS via excessive submissions from a single account.
    #[pallet::storage]
    pub type SubmissionsPerBlock<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        BlockNumberFor<T>,
        u32,
        ValueQuery,
    >;

    /// Counter for decode failures in state change processing.
    /// Useful for monitoring and debugging data format issues.
    #[pallet::storage]
    pub type DecodeFailureCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A Comit has been accepted for processing immediately after basic validation.
        ComitSubmitted {
            comit_id: H256,
            origin: T::AccountId,
            nonce: u64,
            fee: T::Balance,
        },
        /// Comit execution has started on both VMs.
        ComitExecutionStarted { comit_id: H256, timestamp: u64 },
        /// Comit execution has completed (may have failed).
        ComitExecutionCompleted {
            comit_id: H256,
            success: bool,
            gas_used: u64,
        },
        /// A Comit was finalized and applied to the canonical ledger.
        ComitFinalized { comit_id: H256 },
        /// Comit submission failed during verification or execution.
        ComitFailed {
            comit_id: H256,
            reason: ComitFailureReason,
        },
        /// An asset was registered with associated metadata.
        AssetRegistered {
            asset_id: T::AssetId,
            symbol: Vec<u8>,
            decimals: u8,
        },
        /// An account was authorized to submit Comits.
        AccountAuthorized { account: T::AccountId },
        /// An account was deauthorized from submitting Comits.
        AccountDeauthorized { account: T::AccountId },
        /// Canonical ledger was updated with state changes from comit execution.
        CanonicalLedgerUpdated {
            comit_id: H256,
            changes_applied: u32,
        },
        /// An authority was added to the current authority set.
        AuthorityAdded { authority: T::AccountId },
        /// An authority was removed from the current authority set.
        AuthorityRemoved { authority: T::AccountId },
        /// Pending authority changes were scheduled.
        AuthorityChangesScheduled { new_authorities: Vec<T::AccountId> },
        /// Pending authority changes were enacted.
        AuthorityChangesEnacted { new_authorities: Vec<T::AccountId> },
        /// Fee was deducted from an account for Comit execution.
        FeeDeducted {
            account: T::AccountId,
            amount: T::Balance,
            comit_id: H256,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset is already present within the registry.
        AssetAlreadyRegistered,
        /// Attempted to modify the ledger with an unknown asset identifier.
        UnknownAsset,
        /// Provided payloads exceeded configured length constraints.
        PayloadTooLarge,
        /// Both payloads were empty, yielding an invalid Comit.
        EmptyPayloads,
        /// Supplied nonce does not match the expected account nonce.
        InvalidNonce,
        /// Nonce increment would overflow.
        NonceOverflow,
        /// Placeholder error signalling dual-VM verification failure.
        ComitVerificationFailed,
        /// Asset symbol exceeds permitted length.
        SymbolTooLong,
        /// Asset decimals exceed maximum allowed value (0-30).
        InvalidDecimals,
        /// Asset symbol contains invalid characters; must be uppercase ASCII, digits, dash, or underscore.
        InvalidSymbolCharset,
        /// Caller is not authorized to perform this operation.
        Unauthorized,
        /// Insufficient balance to cover the transaction fee.
        InsufficientBalance,
        /// Declared fee does not match the expected fee calculated from execution costs.
        IncorrectFee,
        /// Authority already exists in the authority set.
        AuthorityAlreadyExists,
        /// Authority not found in the authority set.
        AuthorityNotFound,
        /// Would violate minimum authorities constraint.
        BelowMinimumAuthorities,
        /// Would exceed maximum authorities constraint.
        ExceedsMaximumAuthorities,
        /// No pending authority changes to enact.
        NoPendingChanges,
        /// Authority set cannot be empty.
        EmptyAuthoritySet,
        /// EVM execution failed during Comit processing.
        EvmExecutionFailed,
        /// SVM execution failed during Comit processing.
        SvmExecutionFailed,
        /// X3VM execution failed during Comit processing.
        X3ExecutionFailed,
        /// Asset symbol cannot be empty.
        EmptySymbol,
        /// Asset symbol cannot start with dash or underscore.
        InvalidSymbolFormat,
        /// Too many state changes in execution receipts.
        TooManyStateChanges,
        /// Arithmetic overflow in fee calculation.
        FeeOverflow,
        /// Comit ID has already been submitted.
        DuplicateComitId,
        /// Rate limit exceeded: too many Comit submissions per block.
        RateLimitExceeded,
    }

    use frame_support::traits::StorageVersion;

    pub(crate) const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a Comit transaction describing dual-VM execution intents.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit())]
        pub fn submit_comit(
            origin: OriginFor<T>,
            comit_id: H256,
            evm_payload: Vec<u8>,
            svm_payload: Vec<u8>,
            nonce: u64,
            fee: T::Balance,
            prepare_root: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Check for duplicate comit_id (M-4: Comit ID uniqueness)
            ensure!(
                !SubmittedComits::<T>::contains_key(comit_id),
                Error::<T>::DuplicateComitId
            );

            // Rate limiting check (L-6): Prevent DoS via excessive submissions
            const MAX_SUBMISSIONS_PER_BLOCK: u32 = 10;
            let current_block = frame_system::Pallet::<T>::block_number();
            let current_count = SubmissionsPerBlock::<T>::get(&who, current_block);
            ensure!(
                current_count < MAX_SUBMISSIONS_PER_BLOCK,
                Error::<T>::RateLimitExceeded
            );

            // Early authorization check: verify caller is authorized for dual-VM operations
            let operation_context = Self::encode_submit_comit_context(&who, comit_id);
            Self::auth_check(&who, &operation_context)?;

            // First layer checks on payload sizes and emptiness.
            Self::verify_payloads(&comit_id, &evm_payload, &svm_payload)?;

            // Atomic nonce check and increment using try_mutate (C-3)
            // This ensures the nonce is atomically verified and incremented in a single storage operation
            Nonces::<T>::try_mutate(&who, |current_nonce| -> DispatchResult {
                if nonce != *current_nonce {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::InvalidNonce {
                            code: 0x05,
                            expected: *current_nonce,
                            provided: nonce,
                        },
                    ));
                }
                *current_nonce = current_nonce
                    .checked_add(1)
                    .ok_or(Error::<T>::NonceOverflow)?;
                Ok(())
            })?;

            let comit = Comit::<T::AccountId, T::Balance> {
                comit_id,
                origin: who.clone(),
                evm_payload: evm_payload.clone(),
                svm_payload: svm_payload.clone(),
                nonce,
                fee,
                prepare_root,
            };

            // Prepare execution: collect receipts before verifying prepare_root
            let evm_tx = if !evm_payload.is_empty() {
                Some(evm_payload.clone())
            } else {
                None
            };
            let svm_tx = if !svm_payload.is_empty() {
                Some(svm_payload.clone())
            } else {
                None
            };

            // Capture timestamp at execution start (M-6: Fix stale timestamp issue)
            // This ensures consistent timing even in long-running block production
            let execution_start_timestamp =
                <pallet_timestamp::Pallet<T> as UnixTime>::now().as_secs();

            // Execute via configured VM adapters (real or mock based on runtime config)
            // Gas limits: Use runtime-configurable constants (M-3)
            let evm_gas_limit = T::DefaultEvmGasLimit::get();
            let svm_compute_limit = T::DefaultSvmComputeLimit::get();

            let evm_receipt = if let Some(ref tx) = evm_tx {
                // Execute EVM payload via configured adapter
                match T::EvmAdapter::execute(tx, evm_gas_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        // EVM execution failed - return with detailed error
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::EvmExecutionFailed {
                                code: 0x10,
                                evm_error: 1,
                                gas_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            let svm_receipt = if let Some(ref tx) = svm_tx {
                // Execute SVM payload via configured adapter
                match T::SvmAdapter::execute(tx, svm_compute_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        // SVM execution failed - must rollback any EVM changes for atomicity
                        // Note: In current Substrate architecture, returning error rolls back all storage
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::SvmExecutionFailed {
                                code: 0x11,
                                svm_error: 1,
                                compute_units_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            // Check for execution failures
            if let Some(ref receipt) = evm_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::EvmExecutionFailed {
                            code: 0x10,
                            evm_error: 1, // Placeholder for actual EVM error
                            gas_used: receipt.gas_used,
                        },
                    ));
                }
            }

            if let Some(ref receipt) = svm_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::SvmExecutionFailed {
                            code: 0x11,
                            svm_error: 1,          // Placeholder for actual SVM error
                            compute_units_used: 0, // Would come from SVM receipt in real impl
                        },
                    ));
                }
            }

            // Fee deduction: Compute required fee before execution
            let evm_gas_used = evm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let svm_compute_units = svm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let base_fee = T::Balance::default();
            let required_fee =
                Self::calculate_execution_fee(evm_gas_used, svm_compute_units, base_fee)?;

            // Check if declared fee matches required fee
            ensure!(fee >= required_fee, Error::<T>::IncorrectFee);

            // Check sufficient balance
            let free_balance = T::Currency::free_balance(&who);
            ensure!(
                free_balance >= required_fee.into(),
                Error::<T>::InsufficientBalance
            );

            // Deduct the fee
            let imbalance = T::Currency::withdraw(
                &who,
                required_fee.into(),
                frame_support::traits::WithdrawReasons::FEE,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;
            drop(imbalance); // Burn the fee or handle as needed

            // Emit fee deduction event for indexer tracking
            Self::deposit_event(Event::FeeDeducted {
                account: who.clone(),
                amount: required_fee,
                comit_id,
            });

            // Verify dual-VM prepare_root against actual receipts
            if let Err(reason) = Self::verify_dual_vm_with_receipts(
                &comit,
                evm_receipt.as_ref(),
                svm_receipt.as_ref(),
            ) {
                return Err(Self::fail_with_reason(comit_id, reason));
            }

            // Record comit_id as submitted (M-4: prevents duplicate submissions)
            SubmittedComits::<T>::insert(comit_id, current_block);

            // Update rate limit counter for this block (L-6)
            SubmissionsPerBlock::<T>::mutate(&who, current_block, |count| {
                *count = count.saturating_add(1);
            });

            // Record a default Atlas identifier if none exists yet.
            AccountRegistry::<T>::mutate(&who, |maybe_id| {
                if maybe_id.is_none() {
                    *maybe_id = Some(T::AtlasId::default());
                }
            });

            // Emit success events in order: Submitted -> ExecutionStarted -> ExecutionCompleted -> Finalized
            Self::deposit_event(Event::ComitSubmitted {
                comit_id,
                origin: who.clone(),
                nonce,
                fee,
            });

            // Use timestamp captured at execution start (M-6: consistent timing)
            Self::deposit_event(Event::ComitExecutionStarted {
                comit_id,
                timestamp: execution_start_timestamp,
            });

            // Calculate total gas used from both receipts
            let total_gas_used = evm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0)
                + svm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);

            Self::deposit_event(Event::ComitExecutionCompleted {
                comit_id,
                success: true,
                gas_used: total_gas_used,
            });

            // Apply state changes from receipts to CanonicalLedger
            let changes_applied = Self::apply_canonical_ledger_update(
                comit_id,
                evm_receipt.as_ref(),
                svm_receipt.as_ref(),
            )?;

            // Emit event for ledger updates
            if changes_applied > 0 {
                Self::deposit_event(Event::CanonicalLedgerUpdated {
                    comit_id,
                    changes_applied,
                });
            }

            Self::deposit_event(Event::ComitFinalized { comit_id });
            Ok(())
        }

        /// Submit a v2 Comit transaction describing triple-VM execution intents (EVM + SVM + X3VM).
        ///
        /// Atomicity model: if any VM execution fails (error or `success=false`), this extrinsic
        /// returns `Err` and all Substrate storage writes (including CanonicalLedger updates)
        /// are rolled back. Runtime VM adapters MUST be transactional to guarantee rollback
        /// for VM state as well.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit_v2())]
        pub fn submit_comit_v2(
            origin: OriginFor<T>,
            comit_id: H256,
            evm_payload: Vec<u8>,
            svm_payload: Vec<u8>,
            x3_payload: Vec<u8>,
            nonce: u64,
            fee: T::Balance,
            prepare_root: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                !SubmittedComits::<T>::contains_key(comit_id),
                Error::<T>::DuplicateComitId
            );

            const MAX_SUBMISSIONS_PER_BLOCK: u32 = 10;
            let current_block = frame_system::Pallet::<T>::block_number();
            let current_count = SubmissionsPerBlock::<T>::get(&who, current_block);
            ensure!(
                current_count < MAX_SUBMISSIONS_PER_BLOCK,
                Error::<T>::RateLimitExceeded
            );

            let operation_context = Self::encode_submit_comit_v2_context(&who, comit_id);
            Self::auth_check(&who, &operation_context)?;

            Self::verify_payloads_v2(&comit_id, &evm_payload, &svm_payload, &x3_payload)?;

            Nonces::<T>::try_mutate(&who, |current_nonce| -> DispatchResult {
                if nonce != *current_nonce {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::InvalidNonce {
                            code: 0x05,
                            expected: *current_nonce,
                            provided: nonce,
                        },
                    ));
                }
                *current_nonce = current_nonce
                    .checked_add(1)
                    .ok_or(Error::<T>::NonceOverflow)?;
                Ok(())
            })?;

            let comit = ComitV2::<T::AccountId, T::Balance> {
                comit_id,
                origin: who.clone(),
                evm_payload: evm_payload.clone(),
                svm_payload: svm_payload.clone(),
                x3_payload: x3_payload.clone(),
                nonce,
                fee,
                prepare_root,
            };

            let evm_tx = (!evm_payload.is_empty()).then(|| evm_payload.clone());
            let svm_tx = (!svm_payload.is_empty()).then(|| svm_payload.clone());
            let x3_tx = (!x3_payload.is_empty()).then(|| x3_payload.clone());

            let execution_start_timestamp =
                <pallet_timestamp::Pallet<T> as UnixTime>::now().as_secs();

            let evm_gas_limit = T::DefaultEvmGasLimit::get();
            let svm_compute_limit = T::DefaultSvmComputeLimit::get();
            let x3_gas_limit = T::DefaultX3GasLimit::get();

            let evm_receipt = if let Some(ref tx) = evm_tx {
                match T::EvmAdapter::execute(tx, evm_gas_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::EvmExecutionFailed {
                                code: 0x10,
                                evm_error: 1,
                                gas_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            let svm_receipt = if let Some(ref tx) = svm_tx {
                match T::SvmAdapter::execute(tx, svm_compute_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::SvmExecutionFailed {
                                code: 0x11,
                                svm_error: 1,
                                compute_units_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            let x3_receipt = if let Some(ref tx) = x3_tx {
                match T::X3Adapter::execute(tx, x3_gas_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::X3ExecutionFailed {
                                code: 0x12,
                                x3_error: 1,
                                gas_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            if let Some(ref receipt) = evm_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::EvmExecutionFailed {
                            code: 0x10,
                            evm_error: 1,
                            gas_used: receipt.gas_used,
                        },
                    ));
                }
            }

            if let Some(ref receipt) = svm_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::SvmExecutionFailed {
                            code: 0x11,
                            svm_error: 1,
                            compute_units_used: receipt.gas_used,
                        },
                    ));
                }
            }

            if let Some(ref receipt) = x3_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::X3ExecutionFailed {
                            code: 0x12,
                            x3_error: 1,
                            gas_used: receipt.gas_used,
                        },
                    ));
                }
            }

            let evm_gas_used = evm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let svm_compute_units = svm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let x3_gas_used = x3_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let base_fee = T::Balance::default();
            let required_fee = Self::calculate_execution_fee_v2(
                evm_gas_used,
                svm_compute_units,
                x3_gas_used,
                base_fee,
            )?;

            ensure!(fee >= required_fee, Error::<T>::IncorrectFee);

            let free_balance = T::Currency::free_balance(&who);
            ensure!(
                free_balance >= required_fee.into(),
                Error::<T>::InsufficientBalance
            );

            let imbalance = T::Currency::withdraw(
                &who,
                required_fee.into(),
                frame_support::traits::WithdrawReasons::FEE,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;
            drop(imbalance);

            Self::deposit_event(Event::FeeDeducted {
                account: who.clone(),
                amount: required_fee,
                comit_id,
            });

            if let Err(reason) = Self::verify_triple_vm_with_receipts(
                &comit,
                evm_receipt.as_ref(),
                svm_receipt.as_ref(),
                x3_receipt.as_ref(),
            ) {
                return Err(Self::fail_with_reason(comit_id, reason));
            }

            SubmittedComits::<T>::insert(comit_id, current_block);

            SubmissionsPerBlock::<T>::mutate(&who, current_block, |count| {
                *count = count.saturating_add(1);
            });

            AccountRegistry::<T>::mutate(&who, |maybe_id| {
                if maybe_id.is_none() {
                    *maybe_id = Some(T::AtlasId::default());
                }
            });

            Self::deposit_event(Event::ComitSubmitted {
                comit_id,
                origin: who.clone(),
                nonce,
                fee,
            });

            Self::deposit_event(Event::ComitExecutionStarted {
                comit_id,
                timestamp: execution_start_timestamp,
            });

            let total_gas_used = evm_gas_used
                .saturating_add(svm_compute_units)
                .saturating_add(x3_gas_used);

            Self::deposit_event(Event::ComitExecutionCompleted {
                comit_id,
                success: true,
                gas_used: total_gas_used,
            });

            let changes_applied = Self::apply_canonical_ledger_update_v2(
                comit_id,
                evm_receipt.as_ref(),
                svm_receipt.as_ref(),
                x3_receipt.as_ref(),
            )?;

            if changes_applied > 0 {
                Self::deposit_event(Event::CanonicalLedgerUpdated {
                    comit_id,
                    changes_applied,
                });
            }

            Self::deposit_event(Event::ComitFinalized { comit_id });
            Ok(())
        }

        /// Register a new asset and its metadata within the Atlas Kernel.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::register_asset())]
        pub fn register_asset(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            symbol: Vec<u8>,
            decimals: u8,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                !AssetRegistry::<T>::contains_key(asset_id),
                Error::<T>::AssetAlreadyRegistered
            );

            // Validate decimals are within reasonable bounds (0-30)
            ensure!(decimals <= 30, Error::<T>::InvalidDecimals);

            // Validate symbol is not empty
            ensure!(!symbol.is_empty(), Error::<T>::EmptySymbol);

            // Validate symbol does not start with dash or underscore
            ensure!(
                !symbol.starts_with(b"-") && !symbol.starts_with(b"_"),
                Error::<T>::InvalidSymbolFormat
            );

            // Validate symbol: must be uppercase ASCII, digits, dash, or underscore
            for &byte in &symbol {
                let valid = byte.is_ascii_uppercase()  // Uppercase letters
                    || byte.is_ascii_digit()  // Digits
                    || byte == b'-'  // Dash
                    || byte == b'_'; // Underscore
                ensure!(valid, Error::<T>::InvalidSymbolCharset);
            }

            let bounded_symbol: AssetSymbolOf<T> = symbol
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::SymbolTooLong)?;

            let metadata = AssetMetadata {
                symbol: bounded_symbol,
                decimals,
            };
            AssetRegistry::<T>::insert(asset_id, metadata);

            Self::deposit_event(Event::AssetRegistered {
                asset_id,
                symbol,
                decimals,
            });
            Ok(())
        }

        /// Update the canonical ledger balance for a specific account and asset.
        /// The optional Comit identifier triggers a finalized event when supplied.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::update_canonical_balance())]
        pub fn update_canonical_balance(
            origin: OriginFor<T>,
            account: T::AccountId,
            asset_id: T::AssetId,
            new_balance: T::Balance,
            comit_id: Option<H256>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                AssetRegistry::<T>::contains_key(asset_id),
                Error::<T>::UnknownAsset
            );

            CanonicalLedger::<T>::insert(&account, asset_id, new_balance);

            if let Some(id) = comit_id {
                Self::deposit_event(Event::ComitFinalized { comit_id: id });
            }

            Ok(())
        }

        /// Authorize an account to submit Comits.
        /// Only callable by root/governance.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::authorize_account())]
        pub fn authorize_account(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            AuthorizedAccounts::<T>::insert(account.clone(), ());
            Self::deposit_event(Event::AccountAuthorized { account });

            Ok(())
        }

        /// Deauthorize an account from submitting Comits.
        /// Only callable by root/governance.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::deauthorize_account())]
        pub fn deauthorize_account(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            AuthorizedAccounts::<T>::remove(&account);
            Self::deposit_event(Event::AccountDeauthorized { account });

            Ok(())
        }

        /// Add a new authority to the current authority set.
        /// Only callable by governance (root or collective).
        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::add_authority())]
        pub fn add_authority(origin: OriginFor<T>, authority: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            Authorities::<T>::try_mutate(|authorities| -> DispatchResult {
                // Check if authority already exists
                ensure!(
                    !authorities.contains(&authority),
                    Error::<T>::AuthorityAlreadyExists
                );

                // Check max authorities limit
                authorities
                    .try_push(authority.clone())
                    .map_err(|_| Error::<T>::ExceedsMaximumAuthorities)?;

                Self::deposit_event(Event::AuthorityAdded { authority });
                Ok(())
            })
        }

        /// Remove an authority from the current authority set.
        /// Only callable by governance (root or collective).
        #[pallet::call_index(6)]
        #[pallet::weight(<T as Config>::WeightInfo::remove_authority())]
        pub fn remove_authority(origin: OriginFor<T>, authority: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            Authorities::<T>::try_mutate(|authorities| -> DispatchResult {
                // Find and remove the authority
                let pos = authorities
                    .iter()
                    .position(|a| a == &authority)
                    .ok_or(Error::<T>::AuthorityNotFound)?;

                // Check minimum authorities constraint (must keep at least MinAuthorities)
                ensure!(
                    authorities.len() > T::MinAuthorities::get() as usize,
                    Error::<T>::BelowMinimumAuthorities
                );
                // Additional safety: never allow single authority in production
                ensure!(
                    authorities.len() > 1 || T::MinAuthorities::get() == 0,
                    Error::<T>::BelowMinimumAuthorities
                );

                authorities.remove(pos);
                Self::deposit_event(Event::AuthorityRemoved { authority });
                Ok(())
            })
        }

        /// Schedule a complete authority set change for the next session.
        /// Only callable by governance (root or collective).
        #[pallet::call_index(7)]
        #[pallet::weight(<T as Config>::WeightInfo::schedule_authority_change())]
        pub fn schedule_authority_change(
            origin: OriginFor<T>,
            new_authorities: Vec<T::AccountId>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            // Validate authority count bounds (check empty first for better error messages)
            ensure!(!new_authorities.is_empty(), Error::<T>::EmptyAuthoritySet);
            let count = new_authorities.len() as u32;
            ensure!(
                count >= T::MinAuthorities::get(),
                Error::<T>::BelowMinimumAuthorities
            );
            ensure!(
                count <= T::MaxAuthorities::get(),
                Error::<T>::ExceedsMaximumAuthorities
            );

            // Convert to BoundedVec
            let bounded_authorities: BoundedVec<T::AccountId, T::MaxAuthorities> = new_authorities
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ExceedsMaximumAuthorities)?;

            PendingAuthorities::<T>::put(Some(bounded_authorities));
            Self::deposit_event(Event::AuthorityChangesScheduled { new_authorities });

            Ok(())
        }

        /// Enact pending authority changes.
        /// Should be called at session boundaries. Only callable by governance.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as Config>::WeightInfo::enact_authority_change())]
        pub fn enact_authority_change(origin: OriginFor<T>) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            // Get pending changes
            let pending = PendingAuthorities::<T>::take().ok_or(Error::<T>::NoPendingChanges)?;

            // Apply the new authority set
            let new_authorities: Vec<T::AccountId> = pending.into_inner();
            let bounded: BoundedVec<T::AccountId, T::MaxAuthorities> = new_authorities
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ExceedsMaximumAuthorities)?;

            Authorities::<T>::put(bounded);
            Self::deposit_event(Event::AuthorityChangesEnacted { new_authorities });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn verify_payloads(
            comit_id: &H256,
            evm_payload: &[u8],
            svm_payload: &[u8],
        ) -> Result<(), DispatchError> {
            let max_evm = T::MaxEvmPayloadLength::get() as usize;
            let max_svm = T::MaxSvmPayloadLength::get() as usize;
            let max_combined = T::MaxCombinedPayloadLength::get() as usize;

            if evm_payload.is_empty() && svm_payload.is_empty() {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::EmptyPayloads { code: 0x04 },
                ));
            }

            if evm_payload.len() > max_evm {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::EvmPayloadTooLarge {
                        code: 0x01,
                        actual_size: evm_payload.len() as u32,
                        max_size: max_evm as u32,
                    },
                ));
            }

            if svm_payload.len() > max_svm {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::SvmPayloadTooLarge {
                        code: 0x02,
                        actual_size: svm_payload.len() as u32,
                        max_size: max_svm as u32,
                    },
                ));
            }

            if evm_payload.len() + svm_payload.len() > max_combined {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::CombinedPayloadTooLarge {
                        code: 0x03,
                        evm_size: evm_payload.len() as u32,
                        svm_size: svm_payload.len() as u32,
                        max_combined: max_combined as u32,
                    },
                ));
            }
            Ok(())
        }

        fn verify_payloads_v2(
            comit_id: &H256,
            evm_payload: &[u8],
            svm_payload: &[u8],
            x3_payload: &[u8],
        ) -> Result<(), DispatchError> {
            let max_evm = T::MaxEvmPayloadLength::get() as usize;
            let max_svm = T::MaxSvmPayloadLength::get() as usize;
            let max_x3 = T::MaxX3PayloadLength::get() as usize;
            let max_combined = T::MaxCombinedPayloadLengthV2::get() as usize;

            if evm_payload.is_empty() && svm_payload.is_empty() && x3_payload.is_empty() {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::EmptyPayloads { code: 0x04 },
                ));
            }

            if evm_payload.len() > max_evm {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::EvmPayloadTooLarge {
                        code: 0x01,
                        actual_size: evm_payload.len() as u32,
                        max_size: max_evm as u32,
                    },
                ));
            }

            if svm_payload.len() > max_svm {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::SvmPayloadTooLarge {
                        code: 0x02,
                        actual_size: svm_payload.len() as u32,
                        max_size: max_svm as u32,
                    },
                ));
            }

            if x3_payload.len() > max_x3 {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::X3PayloadTooLarge {
                        code: 0x07,
                        actual_size: x3_payload.len() as u32,
                        max_size: max_x3 as u32,
                    },
                ));
            }

            if evm_payload.len() + svm_payload.len() + x3_payload.len() > max_combined {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::CombinedPayloadTooLarge {
                        code: 0x03,
                        evm_size: evm_payload.len() as u32,
                        svm_size: svm_payload.len() as u32,
                        max_combined: max_combined as u32,
                    },
                ));
            }

            Ok(())
        }

        /// Encode operation context for authorization checks
        fn encode_submit_comit_context(caller: &T::AccountId, comit_id: H256) -> Vec<u8> {
            let mut context = Vec::new();
            context.extend_from_slice(b"submit_comit");
            context.extend_from_slice(&caller.encode());
            context.extend_from_slice(comit_id.as_bytes());
            context
        }

        fn encode_submit_comit_v2_context(caller: &T::AccountId, comit_id: H256) -> Vec<u8> {
            let mut context = Vec::new();
            context.extend_from_slice(b"submit_comit_v2");
            context.extend_from_slice(&caller.encode());
            context.extend_from_slice(comit_id.as_bytes());
            context
        }

        /// Maximum number of state changes allowed per Comit execution.
        /// Prevents DoS via excessive storage writes.
        const MAX_STATE_CHANGES: usize = 1000;

        /// Apply state changes from execution receipts to the CanonicalLedger.
        /// This aggregates state_changes from EVM and SVM receipts and updates storage.
        /// Tracks decode failures for monitoring (M-2: Unsafe decode operations).
        fn apply_canonical_ledger_update(
            _comit_id: H256,
            evm_receipt: Option<&ExecutionReceipt>,
            svm_receipt: Option<&ExecutionReceipt>,
        ) -> Result<u32, DispatchError> {
            let mut changes_applied = 0u32;
            let mut decode_failures = 0u32;

            // Aggregate state changes from both receipts
            let mut all_changes = Vec::new();
            if let Some(receipt) = evm_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }
            if let Some(receipt) = svm_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }

            // Bound check: prevent excessive state changes (DoS protection)
            if all_changes.len() > Self::MAX_STATE_CHANGES {
                return Err(Error::<T>::TooManyStateChanges.into());
            }

            // Apply each state change to CanonicalLedger
            // Note: In production, state_changes would map to account balances or contract storage
            // For now, we interpret the first 32 bytes of address as AccountId and key/value as asset balance
            for change in all_changes.iter() {
                // Skip invalid address sizes (count as decode failure)
                if change.address.len() < 32 {
                    decode_failures = decode_failures.saturating_add(1);
                    continue;
                }

                // Extract account from address (first 32 bytes)
                let mut account_bytes = [0u8; 32];
                account_bytes.copy_from_slice(&change.address[..32]);
                let account = T::AccountId::decode(&mut &account_bytes[..]).ok();

                if let Some(acc) = account {
                    // Use the key as asset_id (convert H256 to AssetId)
                    let asset_id_bytes = change.key.as_bytes();
                    let asset_id = T::AssetId::decode(&mut &asset_id_bytes[..]).ok();

                    if let Some(asset) = asset_id {
                        // Use the value as balance (convert H256 to Balance)
                        let balance_bytes = change.value.as_bytes();
                        let balance = T::Balance::decode(&mut &balance_bytes[..]).ok();

                        if let Some(bal) = balance {
                            // Update CanonicalLedger with new balance
                            CanonicalLedger::<T>::insert(&acc, asset, bal);
                            changes_applied = changes_applied.saturating_add(1);
                        } else {
                            // Balance decode failed (M-2: track decode failures)
                            decode_failures = decode_failures.saturating_add(1);
                        }
                    } else {
                        // AssetId decode failed (M-2: track decode failures)
                        decode_failures = decode_failures.saturating_add(1);
                    }
                } else {
                    // AccountId decode failed (M-2: track decode failures)
                    decode_failures = decode_failures.saturating_add(1);
                }
            }

            // Update global decode failure counter for monitoring (M-2)
            if decode_failures > 0 {
                DecodeFailureCount::<T>::mutate(|count| {
                    *count = count.saturating_add(decode_failures);
                });
            }

            Ok(changes_applied)
        }

        /// Minimum fee floor to prevent zero-cost transaction attacks.
        const MIN_FEE: u32 = 1;

        /// Calculate the total execution fee for a Comit based on gas/compute usage.
        /// Uses checked arithmetic to prevent overflow.
        /// Uses ceiling division to prevent zero-fee attacks.
        pub fn calculate_execution_fee(
            evm_gas_used: u64,
            svm_compute_units: u64,
            base_fee: T::Balance,
        ) -> Result<T::Balance, DispatchError> {
            // Gas/compute unit pricing (configurable in production)
            // EVM: 1 unit per 1000 gas (ceiling division)
            // SVM: 1 unit per 1000 compute units (ceiling division)
            // Using saturating_add(999) / 1000 for ceiling division to prevent zero-fee attacks
            let evm_units_u64 = evm_gas_used.saturating_add(999) / 1000;
            let svm_units_u64 = svm_compute_units.saturating_add(999) / 1000;

            let evm_units = T::Balance::from(evm_units_u64 as u32);
            let svm_units = T::Balance::from(svm_units_u64 as u32);

            // Total fee = base + EVM units + SVM units
            // Use checked_add to prevent overflow
            let total_fee = base_fee
                .checked_add(&evm_units)
                .and_then(|t| t.checked_add(&svm_units))
                .ok_or(Error::<T>::FeeOverflow)?;

            // Enforce minimum fee floor to prevent zero-cost attacks
            let min_fee = T::Balance::from(Self::MIN_FEE);
            let final_fee = if total_fee < min_fee {
                min_fee
            } else {
                total_fee
            };

            Ok(final_fee)
        }

        /// Authorization check for dual-VM operations
        /// Enforces allowlist-based access control unless dev-bypass feature is enabled.
        ///
        /// Authorization Semantics:
        /// - With `dev-bypass` feature: All signed callers are accepted (development only)
        /// - Without `dev-bypass` feature (production):
        ///   - Caller MUST be in AuthorizedAccounts storage
        ///   - Empty AuthorizedAccounts = No one is authorized (secure by default)
        ///   - Use `authorize_account` extrinsic to add accounts to allowlist
        ///   
        /// This explicit authorization model prevents unauthorized access and ensures
        /// governance has full control over who can submit Comits.
        fn auth_check(
            caller: &T::AccountId,
            _operation_context: &[u8],
        ) -> Result<(), DispatchError> {
            #[cfg(feature = "dev-bypass")]
            {
                // Development bypass: accept all signed callers
                return Ok(());
            }

            #[cfg(not(feature = "dev-bypass"))]
            {
                // Production: check authorization list
                // If no authorized accounts exist, reject (explicit authorization required)
                if AuthorizedAccounts::<T>::contains_key(caller) {
                    Ok(())
                } else {
                    Err(Error::<T>::Unauthorized.into())
                }
            }
        }

        /// Compute prepare_root for a Comit from its input parameters.
        /// This is the canonical algorithm for generating the prepare_root commitment.
        /// Exported as public for test use (L-3: Avoid test helper duplication).
        ///
        /// # Algorithm
        /// The prepare_root is computed as Blake2-256 hash of concatenated:
        /// - comit_id (32 bytes)
        /// - evm_payload (variable length)
        /// - svm_payload (variable length)
        /// - nonce (8 bytes, little-endian)
        /// - fee (SCALE-encoded)
        pub fn compute_prepare_root(
            comit_id: H256,
            evm_payload: &[u8],
            svm_payload: &[u8],
            nonce: u64,
            fee: T::Balance,
        ) -> H256 {
            let mut data = Vec::new();
            data.extend_from_slice(comit_id.as_bytes());
            data.extend_from_slice(evm_payload);
            data.extend_from_slice(svm_payload);
            data.extend_from_slice(&nonce.to_le_bytes());
            data.extend_from_slice(&fee.encode());
            H256::from(blake2_256(&data))
        }

        /// Compute prepare_root for a v2 Comit from its input parameters.
        ///
        /// Canonical algorithm: Blake2-256 over concatenated:
        /// - comit_id (32)
        /// - evm_payload
        /// - svm_payload
        /// - x3_payload
        /// - nonce (8 LE)
        /// - fee (SCALE)
        pub fn compute_prepare_root_v2(
            comit_id: H256,
            evm_payload: &[u8],
            svm_payload: &[u8],
            x3_payload: &[u8],
            nonce: u64,
            fee: T::Balance,
        ) -> H256 {
            let mut data = Vec::new();
            data.extend_from_slice(comit_id.as_bytes());
            data.extend_from_slice(evm_payload);
            data.extend_from_slice(svm_payload);
            data.extend_from_slice(x3_payload);
            data.extend_from_slice(&nonce.to_le_bytes());
            data.extend_from_slice(&fee.encode());
            H256::from(blake2_256(&data))
        }

        /// Verify prepare_root against actual VM receipts (comprehensive dual-VM commitment)
        ///
        /// # SECURITY NOTICE (H-1 Audit Finding - Design Decision)
        ///
        /// The `prepare_root` is intentionally a commitment to INPUTS only, not OUTPUTS.
        /// This design choice enables:
        /// 1. Client-side pre-computation: Users can compute prepare_root before submission
        /// 2. Deterministic authorization: Wallets can sign based on known inputs
        /// 3. Replay protection: Combined with nonce prevents transaction replay
        ///
        /// ## Trade-offs
        /// - Pro: Simpler client integration, no simulation required
        /// - Con: Cannot verify execution results match expectations
        ///
        /// ## Mitigation for High-Value Transactions
        /// For transactions requiring output verification, implement:
        /// - Application-layer expected_output_hash verification
        /// - Multi-sig validation with result confirmation
        /// - Post-execution audit trail comparison
        ///
        /// The execution receipts are passed to allow future extensions but are
        /// deliberately unused in the current implementation per this design.
        fn verify_dual_vm_with_receipts(
            comit: &ComitOf<T>,
            _evm_receipt: Option<&ExecutionReceipt>,
            _svm_receipt: Option<&ExecutionReceipt>,
        ) -> Result<(), ComitFailureReason> {
            // Reject zero prepare_root unless explicitly allowed by dev-bypass feature
            #[cfg(not(feature = "dev-bypass"))]
            {
                if comit.prepare_root == H256::zero() {
                    return Err(ComitFailureReason::Verification {
                        code: 0x06,
                        reason: blake2_256(b"zero_prepare_root_not_allowed"),
                    });
                }
            }

            // Build canonical dual-VM commitment WITHOUT receipt data.
            // The prepare_root is a commitment to the input payloads and execution parameters,
            // NOT the execution results. This allows clients to compute the prepare_root
            // beforehand and use it to authorize the Comit submission.
            //
            // See function-level documentation for full security rationale (H-1 audit finding).
            let computed_root = Self::compute_prepare_root(
                comit.comit_id,
                &comit.evm_payload,
                &comit.svm_payload,
                comit.nonce,
                comit.fee,
            );

            if computed_root == comit.prepare_root {
                Ok(())
            } else {
                // Hash the mismatch reason for diagnostic
                let mut reason_data = Vec::new();
                reason_data.extend_from_slice(comit.comit_id.as_bytes());
                reason_data.extend_from_slice(computed_root.as_bytes());
                reason_data.extend_from_slice(comit.prepare_root.as_bytes());
                let reason_hash = blake2_256(&reason_data);

                Err(ComitFailureReason::Verification {
                    code: 0x06,
                    reason: reason_hash,
                })
            }
        }

        fn verify_triple_vm_with_receipts(
            comit: &ComitV2Of<T>,
            _evm_receipt: Option<&ExecutionReceipt>,
            _svm_receipt: Option<&ExecutionReceipt>,
            _x3_receipt: Option<&ExecutionReceipt>,
        ) -> Result<(), ComitFailureReason> {
            #[cfg(not(feature = "dev-bypass"))]
            {
                if comit.prepare_root == H256::zero() {
                    return Err(ComitFailureReason::Verification {
                        code: 0x06,
                        reason: blake2_256(b"zero_prepare_root_not_allowed"),
                    });
                }
            }

            let computed_root = Self::compute_prepare_root_v2(
                comit.comit_id,
                &comit.evm_payload,
                &comit.svm_payload,
                &comit.x3_payload,
                comit.nonce,
                comit.fee,
            );

            if computed_root == comit.prepare_root {
                Ok(())
            } else {
                let mut reason_data = Vec::new();
                reason_data.extend_from_slice(comit.comit_id.as_bytes());
                reason_data.extend_from_slice(computed_root.as_bytes());
                reason_data.extend_from_slice(comit.prepare_root.as_bytes());
                let reason_hash = blake2_256(&reason_data);

                Err(ComitFailureReason::Verification {
                    code: 0x06,
                    reason: reason_hash,
                })
            }
        }

        fn fail_with_reason(_comit_id: H256, reason: ComitFailureReason) -> DispatchError {
            let error = Self::reason_to_error(&reason);
            // Note: We do NOT emit ComitFailed event here because:
            // In Substrate, when an extrinsic returns Err, all state changes (including events)
            // are rolled back automatically. Therefore, emitting an event before returning an
            // error is futile - it will never appear in the final block.
            // Failure information is instead conveyed through the error code itself.
            error.into()
        }

        fn reason_to_error(reason: &ComitFailureReason) -> Error<T> {
            match reason {
                ComitFailureReason::EvmPayloadTooLarge { .. } => Error::<T>::PayloadTooLarge,
                ComitFailureReason::SvmPayloadTooLarge { .. } => Error::<T>::PayloadTooLarge,
                ComitFailureReason::X3PayloadTooLarge { .. } => Error::<T>::PayloadTooLarge,
                ComitFailureReason::CombinedPayloadTooLarge { .. } => Error::<T>::PayloadTooLarge,
                ComitFailureReason::EmptyPayloads { .. } => Error::<T>::EmptyPayloads,
                ComitFailureReason::InvalidNonce { .. } => Error::<T>::InvalidNonce,
                ComitFailureReason::Verification { .. } => Error::<T>::ComitVerificationFailed,
                ComitFailureReason::EvmExecutionFailed { .. } => Error::<T>::EvmExecutionFailed,
                ComitFailureReason::SvmExecutionFailed { .. } => Error::<T>::SvmExecutionFailed,
                ComitFailureReason::X3ExecutionFailed { .. } => Error::<T>::X3ExecutionFailed,
            }
        }

        /// Calculate the total execution fee for a v2 Comit based on gas/compute usage.
        pub fn calculate_execution_fee_v2(
            evm_gas_used: u64,
            svm_compute_units: u64,
            x3_gas_used: u64,
            base_fee: T::Balance,
        ) -> Result<T::Balance, DispatchError> {
            let evm_units_u64 = evm_gas_used.saturating_add(999) / 1000;
            let svm_units_u64 = svm_compute_units.saturating_add(999) / 1000;
            let x3_units_u64 = x3_gas_used.saturating_add(999) / 1000;

            let evm_units = T::Balance::from(evm_units_u64 as u32);
            let svm_units = T::Balance::from(svm_units_u64 as u32);
            let x3_units = T::Balance::from(x3_units_u64 as u32);

            let total_fee = base_fee
                .checked_add(&evm_units)
                .and_then(|t| t.checked_add(&svm_units))
                .and_then(|t| t.checked_add(&x3_units))
                .ok_or(Error::<T>::FeeOverflow)?;

            let min_fee = T::Balance::from(Self::MIN_FEE);
            Ok(if total_fee < min_fee {
                min_fee
            } else {
                total_fee
            })
        }

        fn apply_canonical_ledger_update_v2(
            _comit_id: H256,
            evm_receipt: Option<&ExecutionReceipt>,
            svm_receipt: Option<&ExecutionReceipt>,
            x3_receipt: Option<&ExecutionReceipt>,
        ) -> Result<u32, DispatchError> {
            let mut changes_applied = 0u32;
            let mut decode_failures = 0u32;

            let mut all_changes = Vec::new();
            if let Some(receipt) = evm_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }
            if let Some(receipt) = svm_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }
            if let Some(receipt) = x3_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }

            if all_changes.len() > Self::MAX_STATE_CHANGES {
                return Err(Error::<T>::TooManyStateChanges.into());
            }

            for change in all_changes.iter() {
                if change.address.len() < 32 {
                    decode_failures = decode_failures.saturating_add(1);
                    continue;
                }

                let mut account_bytes = [0u8; 32];
                account_bytes.copy_from_slice(&change.address[..32]);
                let account = T::AccountId::decode(&mut &account_bytes[..]).ok();

                if let Some(acc) = account {
                    let asset_id_bytes = change.key.as_bytes();
                    let asset_id = T::AssetId::decode(&mut &asset_id_bytes[..]).ok();

                    if let Some(asset) = asset_id {
                        let balance_bytes = change.value.as_bytes();
                        let balance = T::Balance::decode(&mut &balance_bytes[..]).ok();

                        if let Some(bal) = balance {
                            CanonicalLedger::<T>::insert(&acc, asset, bal);
                            changes_applied = changes_applied.saturating_add(1);
                        } else {
                            decode_failures = decode_failures.saturating_add(1);
                        }
                    } else {
                        decode_failures = decode_failures.saturating_add(1);
                    }
                } else {
                    decode_failures = decode_failures.saturating_add(1);
                }
            }

            if decode_failures > 0 {
                DecodeFailureCount::<T>::mutate(|count| {
                    *count = count.saturating_add(decode_failures);
                });
            }

            Ok(changes_applied)
        }

        /// Execute dual-VM transactions and return the unified state
        #[allow(dead_code)]
        fn do_execute_dual_tx(
            evm_tx: Option<Vec<u8>>,
            svm_tx: Option<Vec<u8>>,
        ) -> Result<SphereState, DispatchError> {
            // Execute transactions on both VMs in parallel (when implemented)
            let _evm_receipt = evm_tx.map(|_tx| ExecutionReceipt {
                success: true,
                gas_used: 21000,
                return_data: Vec::new(),
                logs: Vec::new(),
                state_changes: Vec::new(),
            });

            let _svm_receipt = svm_tx.map(|_tx| ExecutionReceipt {
                success: true,
                gas_used: 5000,
                return_data: Vec::new(),
                logs: Vec::new(),
                state_changes: Vec::new(),
            });

            // Merge receipts into unified state
            Ok(SphereState {
                state_root: H256::default(),
                block_number: 0,
                timestamp: 0,
            })
        }
    }

    /// Implementation of the DualVmDispatcher trait for the Atlas Kernel pallet.
    /// This provides the core coordination logic for executing transactions across
    /// both EVM and SVM runtimes and merging their execution results.
    impl<T: Config> DualVmDispatcher for Pallet<T> {
        type AccountId = T::AccountId;
        type Balance = T::Balance;

        fn execute_evm_tx(&self, tx: Vec<u8>) -> Result<ExecutionReceipt, DispatchError> {
            // Execute via configured EVM adapter (real or mock based on runtime)
            T::EvmAdapter::execute(&tx, 10_000_000)
        }

        fn execute_svm_tx(&self, tx: Vec<u8>) -> Result<ExecutionReceipt, DispatchError> {
            // Execute via configured SVM adapter (real or mock based on runtime)
            T::SvmAdapter::execute(&tx, 200_000)
        }

        fn execute_dual_tx(
            &self,
            evm_tx: Option<Vec<u8>>,
            svm_tx: Option<Vec<u8>>,
        ) -> Result<SphereState, DispatchError> {
            // Execute transactions on both VMs in parallel (when implemented)
            let evm_receipt = if let Some(tx) = evm_tx {
                Some(self.execute_evm_tx(tx)?)
            } else {
                None
            };

            let svm_receipt = if let Some(tx) = svm_tx {
                Some(self.execute_svm_tx(tx)?)
            } else {
                None
            };

            // Merge execution results into unified sphere state
            Ok(self.merge_receipts(evm_receipt.as_ref(), svm_receipt.as_ref()))
        }

        /// Merge EVM and SVM execution receipts into a unified SphereState.
        ///
        /// This function creates a deterministic state root by hashing all execution
        /// data from both VMs in a canonical order:
        /// 1. EVM receipt data (success, gas, return data, logs, state changes)
        /// 2. SVM receipt data (success, compute units, return data, logs, state changes)
        ///
        /// The resulting state root provides:
        /// - Deterministic replay: Same inputs always produce same state root
        /// - Cross-VM commitment: Both VM results are included in a single hash
        /// - Auditability: External verifiers can recompute the state root
        fn merge_receipts(
            &self,
            evm_receipt: Option<&ExecutionReceipt>,
            svm_receipt: Option<&ExecutionReceipt>,
        ) -> SphereState {
            let mut state_data = Vec::new();

            // Include EVM receipt data
            if let Some(receipt) = evm_receipt {
                state_data.extend_from_slice(&receipt.success.encode());
                state_data.extend_from_slice(&receipt.gas_used.encode());
                state_data.extend_from_slice(&receipt.return_data);
                for log in &receipt.logs {
                    state_data.extend_from_slice(&log.address);
                    state_data.extend_from_slice(&log.data);
                }
                for change in &receipt.state_changes {
                    state_data.extend_from_slice(&change.address);
                    state_data.extend_from_slice(change.key.as_bytes());
                    state_data.extend_from_slice(change.value.as_bytes());
                }
            }

            // Include SVM receipt data
            if let Some(receipt) = svm_receipt {
                state_data.extend_from_slice(&receipt.success.encode());
                state_data.extend_from_slice(&receipt.gas_used.encode());
                state_data.extend_from_slice(&receipt.return_data);
                for log in &receipt.logs {
                    state_data.extend_from_slice(&log.address);
                    state_data.extend_from_slice(&log.data);
                }
                for change in &receipt.state_changes {
                    state_data.extend_from_slice(&change.address);
                    state_data.extend_from_slice(change.key.as_bytes());
                    state_data.extend_from_slice(change.value.as_bytes());
                }
            }

            // Get current block number from frame_system
            let current_block = <frame_system::Pallet<T>>::block_number();
            // Get current timestamp from pallet_timestamp using UnixTime trait
            let current_timestamp = <pallet_timestamp::Pallet<T> as UnixTime>::now().as_secs();
            // Generate deterministic state root
            let state_root = H256::from(blake2_256(&state_data));

            SphereState {
                state_root,
                block_number: current_block.saturated_into(),
                timestamp: current_timestamp,
            }
        }

        /// Check if an account is authorized to execute a specific cross-VM operation.
        /// Delegates to the pallet's auth_check method for consistent authorization.
        fn auth_check(
            &self,
            caller: &Self::AccountId,
            operation: &[u8],
        ) -> Result<(), DispatchError> {
            // Delegate to pallet's auth_check for consistent authorization behavior
            // This ensures trait-based calls respect the same AuthorizedAccounts storage
            Self::auth_check(caller, operation)
        }

        /// Calculate execution fees based on gas and compute unit consumption.
        ///
        /// Uses checked arithmetic to prevent overflow in fee calculations.
        /// Uses ceiling division and minimum fee floor to prevent zero-fee attacks.
        /// Returns the total fee required for the transaction.
        fn fee_accounting(
            &self,
            evm_gas_used: u64,
            svm_compute_units: u64,
            base_fee: Self::Balance,
        ) -> Result<Self::Balance, DispatchError> {
            // Delegate to pallet's calculate_execution_fee for consistent behavior
            Self::calculate_execution_fee(evm_gas_used, svm_compute_units, base_fee)
        }

        /// Update the canonical ledger with state changes from a successful comit.
        ///
        /// This function validates and records raw VM state changes from comit execution.
        /// State changes are indexed by comit_id for auditability and external indexers.
        ///
        /// Note: Raw VM state changes (storage slots, account data) are low-level data.
        /// Higher-level balance updates (CanonicalLedger) should be performed via
        /// the `update_canonical_balance` governance extrinsic after off-chain
        /// interpretation of state changes (e.g., detecting ERC20 balance changes).
        fn canonical_ledger_update(
            &self,
            comit_id: H256,
            state_changes: &[StateChange],
        ) -> Result<(), DispatchError> {
            // Validate all state changes are well-formed
            for change in state_changes {
                // Address must not be empty
                if change.address.is_empty() {
                    return Err(DispatchError::Other("Invalid state change: empty address"));
                }
                // Address must be valid EVM (20 bytes) or SVM (32 bytes) format
                let addr_len = change.address.len();
                if addr_len != 20 && addr_len != 32 {
                    return Err(DispatchError::Other(
                        "Invalid state change: address must be 20 bytes (EVM) or 32 bytes (SVM)",
                    ));
                }
            }

            // Record the state changes count for this comit
            let changes_count = state_changes.len() as u32;

            // Emit event for external indexers and auditability
            // Off-chain services can subscribe to this event to interpret state changes
            // and call update_canonical_balance for balance-related changes
            Self::deposit_event(Event::CanonicalLedgerUpdated {
                comit_id,
                changes_applied: changes_count,
            });

            Ok(())
        }
    }

    /// Asset metadata stored alongside each asset id.
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(Symbol))]
    pub struct AssetMetadata<Symbol: MaxEncodedLen> {
        pub symbol: Symbol,
        pub decimals: u8,
    }
}

// WeightInfo trait and implementations are now in weights.rs module
// Re-exported via `pub use weights::WeightInfo;` at module root

// Runtime API definitions for querying Atlas Kernel state
sp_api::decl_runtime_apis! {
    /// Runtime API for querying Atlas Kernel pallet state
    pub trait AtlasKernelRuntimeApi<AccountId, Balance, AssetId> where
        AccountId: Codec,
        Balance: Codec,
        AssetId: Codec,
    {
        /// Get the canonical balance for a specific account and asset
        fn get_canonical_balance(account: AccountId, asset_id: AssetId) -> Balance;

        /// Get asset metadata (symbol, decimals) for a specific asset
        fn get_asset_metadata(asset_id: AssetId) -> Option<(Vec<u8>, u8)>;

        /// Check if an account is authorized to submit Comits
        fn is_authorized(account: AccountId) -> bool;

        /// Get all authorized accounts
        fn get_authorized_accounts() -> Vec<AccountId>;

        /// Get the current authority set
        fn get_authorities() -> Vec<AccountId>;

        /// Map an EVM 20-byte address into a runtime AccountId (Option)
        fn map_evm_address(address: Vec<u8>) -> Option<AccountId>;

        /// Query EVM-specific canonical balance by EVM address
        fn get_evm_balance(evm_address: Vec<u8>, asset_id: AssetId) -> Option<Balance>;

        /// Query contract bytecode for an EVM address
        fn get_evm_code(evm_address: Vec<u8>) -> Vec<u8>;

        /// Query EVM storage at a specific storage key for an EVM address
        fn get_evm_storage(evm_address: Vec<u8>, storage_key: H256) -> Option<H256>;
    }
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod chaos_tests;
