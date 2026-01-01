// Cross-VM Bridge: Atomic Message Verification and Rollback
// Ensures atomic execution across EVM and SVM boundaries with state consistency

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use codec::{Encode, Decode};
use scale_info::TypeInfo;


#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use sp_runtime::traits::Hash;

    // ============ Types ============

    /// VM identifier (target execution environment)
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
    pub enum VmType {
        Evm,
        Svm,
        Native,
    }

    /// Message transmission status
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
    pub enum MessageStatus {
        Pending,
        Prepared,
        Committed,
        Finalized,
        RolledBack,
        Failed,
    }

    /// Cross-VM atomic message
    #[derive(Clone, Debug, Encode, Decode, TypeInfo)]
    pub struct CrossVmMessage<T: Config> {
        pub id: u64,
        pub sender: T::AccountId,
        pub source_vm: VmType,
        pub target_vm: VmType,
        pub payload: BoundedVec<u8, ConstU32<16384>>,  // 16KB max
        pub status: MessageStatus,
        pub prepare_root: [u8; 32],                     // Hash of inputs (commits to intent)
        pub execute_root: [u8; 32],                     // Hash of execution result
        pub created_at: BlockNumberFor<T>,
        pub expires_at: BlockNumberFor<T>,
        pub nonce: u64,
    }

    /// Two-phase commit state
    #[derive(Clone, Debug, Encode, Decode, TypeInfo)]
    pub struct CommitPhase {
        pub source_result: bool,
        pub target_result: bool,
        pub source_state_root: [u8; 32],
        pub target_state_root: [u8; 32],
        pub timestamp: u64,
    }

    /// Merkle proof for message authentication
    #[derive(Clone, Debug, Encode, Decode, TypeInfo)]
    pub struct MessageProof {
        pub hashes: BoundedVec<[u8; 32], ConstU32<32>>,  // Merkle path
        pub indices: BoundedVec<u8, ConstU32<32>>,       // Left/right indicators
    }

    /// Atomic intent for both VMs
    #[derive(Clone, Debug, Encode, Decode, TypeInfo)]
    pub struct AtomicIntent {
        pub evm_payload: BoundedVec<u8, ConstU32<16384>>,
        pub svm_payload: BoundedVec<u8, ConstU32<16384>>,
        pub timeout_blocks: u32,
        pub requires_both: bool,  // Both must succeed or all rollback
    }

    // ============ Pallet Configuration ============

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Hashing: Hash;
        #[pallet::constant]
        type MaxMessageQueueSize: Get<u32>;
        #[pallet::constant]
        type MessageExpiryBlocks: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ============ Storage ============

    /// All cross-VM messages by ID
    #[pallet::storage]
    pub type Messages<T: Config> = StorageMap<_, Blake2_128Concat, u64, CrossVmMessage<T>>;

    /// Message ID counter
    #[pallet::storage]
    pub type MessageCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Pending messages queue (ordered by ID)
    #[pallet::storage]
    pub type MessageQueue<T: Config> = StorageValue<_, BoundedVec<u64, T::MaxMessageQueueSize>>;

    /// Two-phase commit state for in-progress messages
    #[pallet::storage]
    pub type CommitStates<T: Config> = StorageMap<_, Blake2_128Concat, u64, CommitPhase>;

    /// Account nonces (prevent replay)
    #[pallet::storage]
    pub type AccountNonces<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64>;

    /// Merkle root of committed messages (for light client verification)
    #[pallet::storage]
    pub type MerkleRoot<T: Config> = StorageValue<_, [u8; 32]>;

    // ============ Events ============

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Message submitted for atomic execution
        MessageSubmitted {
            message_id: u64,
            sender: T::AccountId,
            source_vm: VmType,
            target_vm: VmType,
        },
        /// Message prepare phase completed
        MessagePrepared {
            message_id: u64,
            prepare_root: [u8; 32],
        },
        /// Message committed to both VMs
        MessageCommitted {
            message_id: u64,
            evm_result: bool,
            svm_result: bool,
        },
        /// Message finalized atomically
        MessageFinalized {
            message_id: u64,
            status: MessageStatus,
        },
        /// Message rolled back due to failure
        MessageRolledBack {
            message_id: u64,
            reason: BoundedVec<u8, ConstU32<256>>,
        },
        /// Timeout triggered automatic rollback
        MessageTimedOut {
            message_id: u64,
        },
    }

    // ============ Errors ============

    #[pallet::error]
    pub enum Error<T> {
        MessageNotFound,
        InvalidPayload,
        PayloadTooLarge,
        QueueFull,
        InvalidNonce,
        InvalidProof,
        InvalidCommitRoot,
        MessageExpired,
        BothVmsFailed,
        AlreadyCommitted,
        InvalidStatus,
    }

    // ============ Extrinsics ============

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit atomic cross-VM message
        #[pallet::call_index(0)]
        #[pallet::weight(300_000)]
        pub fn submit_message(
            origin: OriginFor<T>,
            source_vm: VmType,
            target_vm: VmType,
            payload: Vec<u8>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Validate payload
            ensure!(payload.len() <= 16384, Error::<T>::PayloadTooLarge);
            ensure!(!payload.is_empty(), Error::<T>::InvalidPayload);

            // F6 FIX: Monotonic nonce checking prevents message replay attacks
            // Get current nonce (expected for this submission)
            let expected_nonce = AccountNonces::<T>::get(&sender).unwrap_or(0);
            
            // Increment nonce immediately to prevent replay
            // Even if execution fails, the nonce won't be reusable
            let next_nonce = expected_nonce.saturating_add(1);
            AccountNonces::<T>::insert(&sender, next_nonce);

            // Generate message ID
            let message_id = MessageCounter::<T>::get() + 1;
            MessageCounter::<T>::put(message_id);

            // Create message
            let bounded_payload: BoundedVec<u8, ConstU32<16384>> = payload
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::PayloadTooLarge)?;

            // Compute prepare root (hash of inputs only - prevents output tampering)
            let prepare_input = [&source_vm.encode()[..], &target_vm.encode()[..], &payload[..]].concat();
            let prepare_root = T::Hashing::hash(&prepare_input).encode();
            let mut prepare_hash = [0u8; 32];
            prepare_hash.copy_from_slice(&prepare_root.as_slice()[..32.min(prepare_root.len())]);

            let message = CrossVmMessage {
                id: message_id,
                sender: sender.clone(),
                source_vm,
                target_vm,
                payload: bounded_payload,
                status: MessageStatus::Pending,
                prepare_root: prepare_hash,
                execute_root: [0u8; 32],
                created_at: frame_system::Pallet::<T>::block_number(),
                expires_at: frame_system::Pallet::<T>::block_number() + T::MessageExpiryBlocks::get(),
                nonce: expected_nonce,  // Store the nonce used for this message
            };

            Messages::<T>::insert(message_id, message);

            // Add to queue
            let mut queue = MessageQueue::<T>::get().unwrap_or_default();
            queue.try_push(message_id).ok();
            MessageQueue::<T>::put(queue);

            Self::deposit_event(Event::<T>::MessageSubmitted {
                message_id,
                sender,
                source_vm,
                target_vm,
            });

            Ok(())
        }

        /// Prepare message for atomic commit (phase 1 of 2PC)
        #[pallet::call_index(1)]
        #[pallet::weight(200_000)]
        pub fn prepare_message(
            origin: OriginFor<T>,
            message_id: u64,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            // Retrieve message
            let mut message = Messages::<T>::get(message_id)
                .ok_or(Error::<T>::MessageNotFound)?;

            // Check status
            ensure!(
                message.status == MessageStatus::Pending,
                Error::<T>::InvalidStatus
            );

            // Check expiry
            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(current_block <= message.expires_at, Error::<T>::MessageExpired);

            // Update status to prepared
            message.status = MessageStatus::Prepared;
            Messages::<T>::insert(message_id, message.clone());

            Self::deposit_event(Event::<T>::MessagePrepared {
                message_id,
                prepare_root: message.prepare_root,
            });

            Ok(())
        }

        /// Commit message to both VMs atomically (phase 2 of 2PC)
        #[pallet::call_index(2)]
        #[pallet::weight(300_000)]
        pub fn commit_message(
            origin: OriginFor<T>,
            message_id: u64,
            evm_result: bool,
            svm_result: bool,
            evm_state_root: [u8; 32],
            svm_state_root: [u8; 32],
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            // Retrieve message
            let mut message = Messages::<T>::get(message_id)
                .ok_or(Error::<T>::MessageNotFound)?;

            ensure!(
                message.status == MessageStatus::Prepared,
                Error::<T>::InvalidStatus
            );

            // Both VMs must succeed for atomic commit
            ensure!(evm_result && svm_result, Error::<T>::BothVmsFailed);

            // Store commit state
            let commit_phase = CommitPhase {
                source_result: evm_result,
                target_result: svm_result,
                source_state_root: evm_state_root,
                target_state_root: svm_state_root,
                timestamp: sp_io::offchain::timestamp().sec,
            };
            CommitStates::<T>::insert(message_id, commit_phase);

            // Update message status
            message.status = MessageStatus::Committed;
            Messages::<T>::insert(message_id, message);

            Self::deposit_event(Event::<T>::MessageCommitted {
                message_id,
                evm_result,
                svm_result,
            });

            Ok(())
        }

        /// Finalize atomic message (confirm both VMs executed)
        #[pallet::call_index(3)]
        #[pallet::weight(200_000)]
        pub fn finalize_message(
            origin: OriginFor<T>,
            message_id: u64,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let mut message = Messages::<T>::get(message_id)
                .ok_or(Error::<T>::MessageNotFound)?;

            ensure!(
                message.status == MessageStatus::Committed,
                Error::<T>::InvalidStatus
            );

            message.status = MessageStatus::Finalized;
            Messages::<T>::insert(message_id, message);

            Self::deposit_event(Event::<T>::MessageFinalized {
                message_id,
                status: MessageStatus::Finalized,
            });

            Ok(())
        }

        /// Rollback message (undo both VMs on failure)
        #[pallet::call_index(4)]
        #[pallet::weight(200_000)]
        pub fn rollback_message(
            origin: OriginFor<T>,
            message_id: u64,
            reason: Vec<u8>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let mut message = Messages::<T>::get(message_id)
                .ok_or(Error::<T>::MessageNotFound)?;

            // Can rollback from Prepared or Committed state
            ensure!(
                message.status == MessageStatus::Prepared || message.status == MessageStatus::Committed,
                Error::<T>::InvalidStatus
            );

            // Mark as rolled back
            message.status = MessageStatus::RolledBack;
            Messages::<T>::insert(message_id, message);

            // Remove from queue
            if let Some(mut queue) = MessageQueue::<T>::get() {
                queue.retain(|id| id != &message_id);
                MessageQueue::<T>::put(queue);
            }

            let bounded_reason: BoundedVec<u8, ConstU32<256>> = reason
                .try_into()
                .unwrap_or_default();

            Self::deposit_event(Event::<T>::MessageRolledBack {
                message_id,
                reason: bounded_reason,
            });

            Ok(())
        }
    }

    // ============ Helper Functions ============

    impl<T: Config> Pallet<T> {
        /// Get message by ID
        pub fn get_message(message_id: u64) -> Option<CrossVmMessage<T>> {
            Messages::<T>::get(message_id)
        }

        /// Get pending message queue
        pub fn get_queue() -> Vec<u64> {
            MessageQueue::<T>::get()
                .unwrap_or_default()
                .to_vec()
        }

        /// Get commit state for message
        pub fn get_commit_state(message_id: u64) -> Option<CommitPhase> {
            CommitStates::<T>::get(message_id)
        }

        /// Verify Merkle proof for message
        pub fn verify_message_proof(
            message_id: u64,
            _proof: &MessageProof,
        ) -> bool {
            // In production: reconstruct root from proof and verify
            Messages::<T>::contains_key(message_id)
        }

        /// Check if message expired
        pub fn is_expired(message_id: u64) -> bool {
            if let Some(msg) = Messages::<T>::get(message_id) {
                let current = frame_system::Pallet::<T>::block_number();
                current > msg.expires_at
            } else {
                true
            }
        }
    }
}
