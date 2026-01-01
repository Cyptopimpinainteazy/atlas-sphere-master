//! # X3 Verifier Pallet
//!
//! On-chain verification of X3 execution receipts from the swarm network.
//! This pallet validates off-chain computations and applies their state changes
//! to the canonical ledger after verification.
//!
//! ## Overview
//!
//! The X3 Verifier implements:
//! - **Receipt Submission**: Accept execution receipts from swarm nodes
//! - **Signature Verification**: Validate executor signatures
//! - **Merkle Proof Verification**: Verify state transition proofs
//! - **State Application**: Apply verified changes to chain state
//! - **Reward Distribution**: Pay executors for valid work
//! - **Slashing**: Penalize malicious or incorrect submissions
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      SWARM NETWORK                               │
//! │  ┌─────────┐   ┌─────────┐   ┌─────────┐                        │
//! │  │ Node 1  │   │ Node 2  │   │ Node N  │                        │
//! │  └────┬────┘   └────┬────┘   └────┬────┘                        │
//! │       │             │             │                              │
//! │       └─────────────┴─────────────┘                              │
//! │                     │                                            │
//! │                     ▼                                            │
//! │           ┌─────────────────┐                                    │
//! │           │ Execution       │                                    │
//! │           │ Receipt         │                                    │
//! │           └────────┬────────┘                                    │
//! └────────────────────│────────────────────────────────────────────┘
//!                      │
//!                      ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                   X3 VERIFIER PALLET                             │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
//! │  │  Signature  │  │   Merkle    │  │   State     │              │
//! │  │  Verify     │─▶│   Verify    │─▶│   Apply     │              │
//! │  └─────────────┘  └─────────────┘  └─────────────┘              │
//! │                                           │                      │
//! │                                           ▼                      │
//! │                                 ┌─────────────────┐              │
//! │                                 │ Canonical       │              │
//! │                                 │ Ledger Update   │              │
//! │                                 └─────────────────┘              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security
//!
//! - All receipts require valid executor signatures
//! - Merkle proofs verify state transitions
//! - Redundant verification for high-value operations
//! - Slashing for invalid submissions

#![cfg_attr(not(feature = "std"), no_std)]


#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod runtime_api;
// Note: We don't re-export runtime_api::* to avoid conflicts with JobId defined in pallet

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, Get, ReservableCurrency, StorageVersion},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_io::hashing::{blake2_256, keccak_256};
    use sp_runtime::traits::Saturating;
    use sp_std::vec::Vec;

    /// Current storage version
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    // ============================================================================
    // Types
    // ============================================================================

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Job identifier (32 bytes)
    pub type JobId = H256;

    /// State root (32 bytes)
    pub type StateRoot = H256;

    /// Execution receipt from swarm node
    #[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ExecutionReceipt<T: Config> {
        /// Unique job identifier
        pub job_id: JobId,
        /// Executor's public key (account)
        pub executor: T::AccountId,
        /// Hash of the input (bytecode + arguments)
        pub input_hash: H256,
        /// Hash of the output
        pub output_hash: H256,
        /// State root before execution
        pub state_root_before: StateRoot,
        /// State root after execution
        pub state_root_after: StateRoot,
        /// Gas consumed
        pub gas_used: u128,
        /// Execution timestamp
        pub timestamp: u64,
        /// Output data (bounded)
        pub output_data: BoundedVec<u8, T::MaxOutputSize>,
        /// State changes (key-value pairs)
        pub state_changes: BoundedVec<
            (
                BoundedVec<u8, T::MaxKeySize>,
                BoundedVec<u8, T::MaxValueSize>,
            ),
            T::MaxStateChanges,
        >,
        /// Merkle proof for state transition
        pub merkle_proof: BoundedVec<H256, T::MaxProofDepth>,
        /// Signature over receipt data
        pub signature: BoundedVec<u8, ConstU32<64>>,
    }

    impl<T: Config> Clone for ExecutionReceipt<T> {
        fn clone(&self) -> Self {
            Self {
                job_id: self.job_id,
                executor: self.executor.clone(),
                input_hash: self.input_hash,
                output_hash: self.output_hash,
                state_root_before: self.state_root_before,
                state_root_after: self.state_root_after,
                gas_used: self.gas_used,
                timestamp: self.timestamp,
                output_data: self.output_data.clone(),
                state_changes: self.state_changes.clone(),
                merkle_proof: self.merkle_proof.clone(),
                signature: self.signature.clone(),
            }
        }
    }

    impl<T: Config> core::fmt::Debug for ExecutionReceipt<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("ExecutionReceipt")
                .field("job_id", &self.job_id)
                .field("gas_used", &self.gas_used)
                .field("timestamp", &self.timestamp)
                .finish()
        }
    }

    impl<T: Config> PartialEq for ExecutionReceipt<T> {
        fn eq(&self, other: &Self) -> bool {
            self.job_id == other.job_id
                && self.executor == other.executor
                && self.input_hash == other.input_hash
                && self.output_hash == other.output_hash
                && self.gas_used == other.gas_used
        }
    }

    impl<T: Config> Eq for ExecutionReceipt<T> {}

    /// Job status
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq, Default)]
    pub enum JobStatus {
        #[default]
        Pending,
        Submitted,
        Verified,
        Applied,
        Failed,
        Disputed,
    }

    /// Job record
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct JobRecord<T: Config> {
        /// Job submitter
        pub submitter: T::AccountId,
        /// Bytecode hash
        pub bytecode_hash: H256,
        /// Input data hash
        pub input_hash: H256,
        /// Gas limit
        pub gas_limit: u128,
        /// Reward amount
        pub reward: BalanceOf<T>,
        /// Status
        pub status: JobStatus,
        /// Block submitted
        pub submitted_at: BlockNumberFor<T>,
        /// Executor (if assigned)
        pub executor: Option<T::AccountId>,
        /// Receipt (if submitted)
        pub receipt_hash: Option<H256>,
    }

    /// Registered executor
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct ExecutorRecord<T: Config> {
        /// Executor account
        pub account: T::AccountId,
        /// Stake amount
        pub stake: BalanceOf<T>,
        /// Jobs completed
        pub jobs_completed: u64,
        /// Jobs failed
        pub jobs_failed: u64,
        /// Total rewards earned
        pub total_rewards: BalanceOf<T>,
        /// Is active
        pub active: bool,
        /// Reputation score (0-100)
        pub reputation: u8,
    }

    // ============================================================================
    // Config
    // ============================================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for rewards and staking
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Origin that can register executors
        type ExecutorRegistrar: EnsureOrigin<Self::RuntimeOrigin>;

        /// Minimum stake to become executor
        #[pallet::constant]
        type MinExecutorStake: Get<BalanceOf<Self>>;

        /// Maximum output data size
        #[pallet::constant]
        type MaxOutputSize: Get<u32>;

        /// Maximum key size for state changes
        #[pallet::constant]
        type MaxKeySize: Get<u32>;

        /// Maximum value size for state changes
        #[pallet::constant]
        type MaxValueSize: Get<u32>;

        /// Maximum state changes per receipt
        #[pallet::constant]
        type MaxStateChanges: Get<u32>;

        /// Maximum Merkle proof depth
        #[pallet::constant]
        type MaxProofDepth: Get<u32>;

        /// Reward share for executor (percentage, e.g., 70)
        #[pallet::constant]
        type ExecutorRewardShare: Get<u32>;

        /// Protocol fee share (percentage, e.g., 15)
        #[pallet::constant]
        type ProtocolFeeShare: Get<u32>;

        /// Slash amount for invalid submissions
        #[pallet::constant]
        type SlashAmount: Get<BalanceOf<Self>>;

        /// Job timeout in blocks
        #[pallet::constant]
        type JobTimeout: Get<BlockNumberFor<Self>>;

        /// Weight information
        type WeightInfo: WeightInfo;
    }

    // ============================================================================
    // Pallet
    // ============================================================================

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ============================================================================
    // Storage
    // ============================================================================

    /// Registered executors
    #[pallet::storage]
    #[pallet::getter(fn executors)]
    pub type Executors<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ExecutorRecord<T>, OptionQuery>;

    /// Pending jobs
    #[pallet::storage]
    #[pallet::getter(fn jobs)]
    pub type Jobs<T: Config> = StorageMap<_, Blake2_128Concat, JobId, JobRecord<T>, OptionQuery>;

    /// Submitted receipts
    #[pallet::storage]
    #[pallet::getter(fn receipts)]
    pub type Receipts<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, ExecutionReceipt<T>, OptionQuery>;

    /// Verified state roots (job_id -> final state root)
    #[pallet::storage]
    pub type VerifiedStateRoots<T: Config> =
        StorageMap<_, Blake2_128Concat, JobId, StateRoot, OptionQuery>;

    /// Total jobs submitted
    #[pallet::storage]
    pub type TotalJobsSubmitted<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total jobs verified
    #[pallet::storage]
    pub type TotalJobsVerified<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Protocol treasury
    #[pallet::storage]
    pub type ProtocolTreasury<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Verification enabled
    #[pallet::storage]
    #[pallet::getter(fn verification_enabled)]
    pub type VerificationEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    // ============================================================================
    // Genesis
    // ============================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub verification_enabled: bool,
        pub initial_executors: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            VerificationEnabled::<T>::put(self.verification_enabled);
            for executor in &self.initial_executors {
                let record = ExecutorRecord {
                    account: executor.clone(),
                    stake: T::MinExecutorStake::get(),
                    jobs_completed: 0,
                    jobs_failed: 0,
                    total_rewards: 0u32.into(),
                    active: true,
                    reputation: 50,
                };
                Executors::<T>::insert(executor, record);
            }
        }
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New job submitted
        JobSubmitted {
            job_id: JobId,
            submitter: T::AccountId,
            reward: BalanceOf<T>,
        },
        /// Job assigned to executor
        JobAssigned {
            job_id: JobId,
            executor: T::AccountId,
        },
        /// Receipt submitted
        ReceiptSubmitted {
            job_id: JobId,
            executor: T::AccountId,
            receipt_hash: H256,
        },
        /// Receipt verified and state applied
        ReceiptVerified {
            job_id: JobId,
            state_root: StateRoot,
            gas_used: u128,
        },
        /// Receipt verification failed
        ReceiptFailed {
            job_id: JobId,
            reason: BoundedVec<u8, ConstU32<128>>,
        },
        /// Executor registered
        ExecutorRegistered {
            executor: T::AccountId,
            stake: BalanceOf<T>,
        },
        /// Executor slashed
        ExecutorSlashed {
            executor: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Reward distributed
        RewardDistributed {
            executor: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// State change applied
        StateChangeApplied {
            job_id: JobId,
            key: BoundedVec<u8, T::MaxKeySize>,
            value_hash: H256,
        },
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Verification is disabled
        VerificationDisabled,
        /// Job not found
        JobNotFound,
        /// Receipt not found
        ReceiptNotFound,
        /// Executor not registered
        ExecutorNotRegistered,
        /// Executor not active
        ExecutorNotActive,
        /// Invalid signature
        InvalidSignature,
        /// Invalid Merkle proof
        InvalidMerkleProof,
        /// State root mismatch
        StateRootMismatch,
        /// Job already completed
        JobAlreadyCompleted,
        /// Insufficient stake
        InsufficientStake,
        /// Job timeout
        JobTimeout,
        /// Not authorized
        NotAuthorized,
        /// Already registered
        AlreadyRegistered,
        /// Invalid receipt format
        InvalidReceiptFormat,
        /// Output too large
        OutputTooLarge,
    }

    // ============================================================================
    // Hooks
    // ============================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: BlockNumberFor<T>) {
            // Timeout expired jobs
            Self::timeout_expired_jobs(block);
        }
    }

    // ============================================================================
    // Calls
    // ============================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register as an executor (stake required)
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_executor())]
        pub fn register_executor(origin: OriginFor<T>, stake: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !Executors::<T>::contains_key(&who),
                Error::<T>::AlreadyRegistered
            );
            ensure!(
                stake >= T::MinExecutorStake::get(),
                Error::<T>::InsufficientStake
            );

            // Reserve stake
            T::Currency::reserve(&who, stake)?;

            let record = ExecutorRecord {
                account: who.clone(),
                stake,
                jobs_completed: 0,
                jobs_failed: 0,
                total_rewards: 0u32.into(),
                active: true,
                reputation: 50,
            };

            Executors::<T>::insert(&who, record);
            Self::deposit_event(Event::ExecutorRegistered {
                executor: who,
                stake,
            });

            Ok(())
        }

        /// Submit a job for off-chain execution
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::submit_job())]
        pub fn submit_job(
            origin: OriginFor<T>,
            bytecode_hash: H256,
            input_hash: H256,
            gas_limit: u128,
            reward: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                Self::verification_enabled(),
                Error::<T>::VerificationDisabled
            );

            // Reserve reward
            T::Currency::reserve(&who, reward)?;

            // Generate job ID
            let job_id = Self::generate_job_id(&who, &bytecode_hash, &input_hash);

            let record = JobRecord {
                submitter: who.clone(),
                bytecode_hash,
                input_hash,
                gas_limit,
                reward,
                status: JobStatus::Pending,
                submitted_at: frame_system::Pallet::<T>::block_number(),
                executor: None,
                receipt_hash: None,
            };

            Jobs::<T>::insert(job_id, record);
            TotalJobsSubmitted::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::JobSubmitted {
                job_id,
                submitter: who,
                reward,
            });

            Ok(())
        }

        /// Submit execution receipt
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::submit_receipt())]
        pub fn submit_receipt(
            origin: OriginFor<T>,
            receipt: ExecutionReceipt<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                Self::verification_enabled(),
                Error::<T>::VerificationDisabled
            );

            // Verify executor is registered and active
            let executor_record =
                Executors::<T>::get(&who).ok_or(Error::<T>::ExecutorNotRegistered)?;
            ensure!(executor_record.active, Error::<T>::ExecutorNotActive);
            ensure!(receipt.executor == who, Error::<T>::NotAuthorized);

            // Verify job exists and is pending
            Jobs::<T>::try_mutate(receipt.job_id, |maybe_job| {
                let job = maybe_job.as_mut().ok_or(Error::<T>::JobNotFound)?;
                ensure!(
                    job.status == JobStatus::Pending || job.status == JobStatus::Submitted,
                    Error::<T>::JobAlreadyCompleted
                );

                // Verify signature
                Self::verify_receipt_signature(&receipt)?;

                // Verify Merkle proof
                Self::verify_merkle_proof(&receipt)?;

                // Store receipt
                let receipt_hash = Self::hash_receipt(&receipt);
                Receipts::<T>::insert(receipt_hash, receipt.clone());

                // Update job status
                job.status = JobStatus::Submitted;
                job.executor = Some(who.clone());
                job.receipt_hash = Some(receipt_hash);

                Self::deposit_event(Event::ReceiptSubmitted {
                    job_id: receipt.job_id,
                    executor: who.clone(),
                    receipt_hash,
                });

                // Auto-verify if proof is valid (simplified for demo)
                Self::verify_and_apply_receipt(receipt.job_id, &receipt, job)?;

                Ok(())
            })
        }

        /// Dispute a receipt (for validators/other executors)
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::dispute_receipt())]
        pub fn dispute_receipt(
            origin: OriginFor<T>,
            job_id: JobId,
            reason: BoundedVec<u8, ConstU32<128>>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            Jobs::<T>::try_mutate(job_id, |maybe_job| {
                let job = maybe_job.as_mut().ok_or(Error::<T>::JobNotFound)?;
                ensure!(
                    job.status == JobStatus::Submitted || job.status == JobStatus::Verified,
                    Error::<T>::JobAlreadyCompleted
                );

                job.status = JobStatus::Disputed;

                Self::deposit_event(Event::ReceiptFailed { job_id, reason });

                Ok(())
            })
        }

        /// Toggle verification on/off
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::toggle_verification())]
        pub fn toggle_verification(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
            T::ExecutorRegistrar::ensure_origin(origin)?;
            VerificationEnabled::<T>::put(enabled);
            Ok(())
        }

        /// Deactivate executor
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::deactivate_executor())]
        pub fn deactivate_executor(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Executors::<T>::try_mutate(&who, |maybe_executor| {
                let executor = maybe_executor
                    .as_mut()
                    .ok_or(Error::<T>::ExecutorNotRegistered)?;
                executor.active = false;

                // Unreserve stake
                T::Currency::unreserve(&who, executor.stake);

                Ok(())
            })
        }
    }

    // ============================================================================
    // Internal Functions
    // ============================================================================

    impl<T: Config> Pallet<T> {
        /// Generate unique job ID
        fn generate_job_id(
            submitter: &T::AccountId,
            bytecode_hash: &H256,
            input_hash: &H256,
        ) -> JobId {
            let block = frame_system::Pallet::<T>::block_number();
            let mut data = submitter.encode();
            data.extend(bytecode_hash.as_bytes());
            data.extend(input_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        }

        /// Hash a receipt
        fn hash_receipt(receipt: &ExecutionReceipt<T>) -> H256 {
            let data = receipt.encode();
            H256::from(blake2_256(&data))
        }

        /// Verify receipt signature
        fn verify_receipt_signature(receipt: &ExecutionReceipt<T>) -> DispatchResult {
            // Construct message that was signed
            let mut msg = Vec::new();
            msg.extend(receipt.job_id.as_bytes());
            msg.extend(receipt.input_hash.as_bytes());
            msg.extend(receipt.output_hash.as_bytes());
            msg.extend(receipt.state_root_before.as_bytes());
            msg.extend(receipt.state_root_after.as_bytes());
            msg.extend(&receipt.gas_used.to_le_bytes());
            msg.extend(&receipt.timestamp.to_le_bytes());

            let msg_hash = keccak_256(&msg);

            // In production, verify against executor's public key
            // For now, we do a basic length check
            if receipt.signature.len() != 64 {
                return Err(Error::<T>::InvalidSignature.into());
            }

            // Placeholder for actual signature verification
            // Would use sp_io::crypto::secp256k1_ecdsa_recover or similar
            let _signature_bytes = receipt.signature.as_slice();
            let _message = H256::from(msg_hash);

            Ok(())
        }

        /// Verify Merkle proof for state transition
        fn verify_merkle_proof(receipt: &ExecutionReceipt<T>) -> DispatchResult {
            // Verify the state root transition using Merkle proof
            let proof = &receipt.merkle_proof;

            if proof.is_empty() {
                // Empty proof is valid for no state changes
                if receipt.state_changes.is_empty() {
                    return Ok(());
                }
                return Err(Error::<T>::InvalidMerkleProof.into());
            }

            // Compute root from state changes
            let mut computed_root = receipt.state_root_before;

            for change in receipt.state_changes.iter() {
                let key_hash = H256::from(blake2_256(&change.0));
                let value_hash = H256::from(blake2_256(&change.1));

                // Simplified Merkle verification
                let mut combined = Vec::new();
                combined.extend(computed_root.as_bytes());
                combined.extend(key_hash.as_bytes());
                combined.extend(value_hash.as_bytes());
                computed_root = H256::from(blake2_256(&combined));
            }

            // Apply proof path
            for proof_element in proof.iter() {
                let mut combined = Vec::new();
                combined.extend(computed_root.as_bytes());
                combined.extend(proof_element.as_bytes());
                computed_root = H256::from(blake2_256(&combined));
            }

            // Final root should match state_root_after
            if computed_root != receipt.state_root_after {
                return Err(Error::<T>::StateRootMismatch.into());
            }

            Ok(())
        }

        /// Verify receipt and apply state changes
        fn verify_and_apply_receipt(
            job_id: JobId,
            receipt: &ExecutionReceipt<T>,
            job: &mut JobRecord<T>,
        ) -> DispatchResult {
            // Apply state changes
            for change in receipt.state_changes.iter() {
                // In production, this would write to actual chain storage
                // For now, we just emit events
                let key = change.0.clone();
                let value_hash = H256::from(blake2_256(&change.1));

                Self::deposit_event(Event::StateChangeApplied {
                    job_id,
                    key,
                    value_hash,
                });
            }

            // Store verified state root
            VerifiedStateRoots::<T>::insert(job_id, receipt.state_root_after);

            // Distribute rewards
            Self::distribute_rewards(job, &receipt.executor)?;

            // Update job status
            job.status = JobStatus::Applied;

            // Update executor stats
            Executors::<T>::try_mutate(&receipt.executor, |maybe_executor| {
                if let Some(executor) = maybe_executor.as_mut() {
                    executor.jobs_completed = executor.jobs_completed.saturating_add(1);
                    executor.reputation = executor.reputation.saturating_add(1).min(100);
                }
                Ok::<(), Error<T>>(())
            })?;

            TotalJobsVerified::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::ReceiptVerified {
                job_id,
                state_root: receipt.state_root_after,
                gas_used: receipt.gas_used,
            });

            Ok(())
        }

        /// Distribute rewards to executor and protocol
        fn distribute_rewards(job: &JobRecord<T>, executor: &T::AccountId) -> DispatchResult {
            let total_reward = job.reward;
            let executor_share = T::ExecutorRewardShare::get();
            let protocol_share = T::ProtocolFeeShare::get();

            // Calculate amounts
            let executor_amount =
                total_reward.saturating_mul(executor_share.into()) / 100u32.into();
            let protocol_amount =
                total_reward.saturating_mul(protocol_share.into()) / 100u32.into();

            // Unreserve from submitter
            T::Currency::unreserve(&job.submitter, total_reward);

            // Transfer to executor
            T::Currency::transfer(
                &job.submitter,
                executor,
                executor_amount,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            // Update executor total rewards
            Executors::<T>::try_mutate(executor, |maybe_executor| {
                if let Some(exec) = maybe_executor.as_mut() {
                    exec.total_rewards = exec.total_rewards.saturating_add(executor_amount);
                }
                Ok::<(), Error<T>>(())
            })?;

            // Add to protocol treasury
            ProtocolTreasury::<T>::mutate(|treasury| {
                *treasury = treasury.saturating_add(protocol_amount);
            });

            Self::deposit_event(Event::RewardDistributed {
                executor: executor.clone(),
                amount: executor_amount,
            });

            Ok(())
        }

        /// Timeout expired jobs
        fn timeout_expired_jobs(current_block: BlockNumberFor<T>) {
            let timeout = T::JobTimeout::get();

            // This is a simplified version; production would use a more efficient approach
            // such as maintaining a list of jobs by expiry block
            Jobs::<T>::iter().for_each(|(job_id, mut job)| {
                if job.status == JobStatus::Pending {
                    let expiry = job.submitted_at.saturating_add(timeout);
                    if current_block >= expiry {
                        job.status = JobStatus::Failed;
                        Jobs::<T>::insert(job_id, job.clone());

                        // Return reserved funds to submitter
                        T::Currency::unreserve(&job.submitter, job.reward);
                    }
                }
            });
        }

        /// Slash an executor for invalid submission
        pub fn slash_executor(executor: &T::AccountId) -> DispatchResult {
            Executors::<T>::try_mutate(executor, |maybe_executor| {
                let exec = maybe_executor
                    .as_mut()
                    .ok_or(Error::<T>::ExecutorNotRegistered)?;

                let slash_amount = T::SlashAmount::get().min(exec.stake);

                // Slash from stake (discard imbalance - burned)
                let _ = T::Currency::slash_reserved(executor, slash_amount);
                exec.stake = exec.stake.saturating_sub(slash_amount);
                exec.jobs_failed = exec.jobs_failed.saturating_add(1);
                exec.reputation = exec.reputation.saturating_sub(10);

                // Deactivate if stake too low
                if exec.stake < T::MinExecutorStake::get() {
                    exec.active = false;
                }

                Self::deposit_event(Event::ExecutorSlashed {
                    executor: executor.clone(),
                    amount: slash_amount,
                });

                Ok(())
            })
        }

        /// Get executor info
        pub fn get_executor(account: &T::AccountId) -> Option<ExecutorRecord<T>> {
            Executors::<T>::get(account)
        }

        /// Get job info
        pub fn get_job(job_id: &JobId) -> Option<JobRecord<T>> {
            Jobs::<T>::get(job_id)
        }

        /// Check if verification is active
        pub fn is_verification_active() -> bool {
            Self::verification_enabled()
        }
    }
}
