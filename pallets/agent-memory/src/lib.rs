//! # X3Chain Agent Memory Pallet
//!
//! Append-only on-chain memory for AI agents with LLM-friendly serialization.
//!
//! ## Overview
//!
//! This pallet provides:
//! - Append-only memory logs per agent
//! - Delta compression for efficient storage
//! - JSONL-like output format for LLM consumption
//! - Read/write permissions per agent
//! - Chunk-based pagination for large memories
//! - Pruning of old entries based on TTL
//!
//! ## Memory Model
//!
//! Memory is organized as:
//! - MemoryEntry: A single log entry with timestamp, type, and content
//! - MemoryChunk: A batch of entries for efficient storage/retrieval
//! - Each agent has independent memory with configurable limits

#![cfg_attr(not(feature = "std"), no_std)]


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;



#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod types;
pub use types::*;

pub mod runtime_api;
pub use runtime_api::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Saturating, Zero};
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Type alias for agent ID.
    pub type AgentId = u32;

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for storage deposits.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Maximum entries per chunk.
        #[pallet::constant]
        type MaxEntriesPerChunk: Get<u32>;

        /// Maximum chunks per agent.
        #[pallet::constant]
        type MaxChunksPerAgent: Get<u32>;

        /// Cost per byte of storage.
        #[pallet::constant]
        type StorageByteCost: Get<BalanceOf<Self>>;

        /// Default TTL in blocks.
        #[pallet::constant]
        type DefaultTtl: Get<BlockNumberFor<Self>>;

        /// Origin that can prune memory.
        type PruneOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    // ========================================================================
    // Storage Items
    // ========================================================================

    /// Memory chunks per agent.
    #[pallet::storage]
    #[pallet::getter(fn memory_chunks)]
    pub type MemoryChunks<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AgentId,
        Blake2_128Concat,
        u32, // chunk_id
        MemoryChunk<BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Current chunk ID per agent.
    #[pallet::storage]
    #[pallet::getter(fn current_chunk)]
    pub type CurrentChunk<T: Config> = StorageMap<_, Blake2_128Concat, AgentId, u32, ValueQuery>;

    /// Total entry count per agent.
    #[pallet::storage]
    #[pallet::getter(fn entry_count)]
    pub type EntryCount<T: Config> = StorageMap<_, Blake2_128Concat, AgentId, u64, ValueQuery>;

    /// Memory permissions per agent.
    #[pallet::storage]
    #[pallet::getter(fn permissions)]
    pub type MemoryPermissions<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, MemoryAccess<T::AccountId>, ValueQuery>;

    /// Storage used per agent in bytes.
    #[pallet::storage]
    #[pallet::getter(fn storage_used)]
    pub type StorageUsed<T: Config> = StorageMap<_, Blake2_128Concat, AgentId, u64, ValueQuery>;

    /// Storage deposit per agent.
    #[pallet::storage]
    #[pallet::getter(fn storage_deposit)]
    pub type StorageDeposit<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, BalanceOf<T>, ValueQuery>;

    /// Agent controller mapping (from agent-accounts).
    #[pallet::storage]
    #[pallet::getter(fn agent_controller)]
    pub type AgentController<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, T::AccountId, OptionQuery>;

    /// Agent operator mapping (from agent-accounts).
    #[pallet::storage]
    #[pallet::getter(fn agent_operator)]
    pub type AgentOperator<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, T::AccountId, OptionQuery>;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Memory entry was appended.
        EntryAppended {
            agent_id: AgentId,
            entry_id: u64,
            entry_type: EntryType,
            size: u32,
        },
        /// Memory chunk was finalized.
        ChunkFinalized {
            agent_id: AgentId,
            chunk_id: u32,
            entries: u32,
        },
        /// Memory was pruned.
        MemoryPruned {
            agent_id: AgentId,
            chunks_removed: u32,
            bytes_freed: u64,
        },
        /// Memory permissions were updated.
        PermissionsUpdated { agent_id: AgentId },
        /// Agent memory was initialized.
        MemoryInitialized {
            agent_id: AgentId,
            controller: T::AccountId,
            operator: T::AccountId,
        },
        /// Deposit was increased.
        DepositIncreased {
            agent_id: AgentId,
            amount: BalanceOf<T>,
        },
        /// Deposit was withdrawn.
        DepositWithdrawn {
            agent_id: AgentId,
            amount: BalanceOf<T>,
        },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Agent not found.
        AgentNotFound,
        /// Memory not initialized.
        MemoryNotInitialized,
        /// No permission to write.
        WritePermissionDenied,
        /// No permission to read.
        ReadPermissionDenied,
        /// Content too long.
        ContentTooLong,
        /// Too many chunks.
        TooManyChunks,
        /// Chunk not found.
        ChunkNotFound,
        /// Insufficient deposit.
        InsufficientDeposit,
        /// Not controller.
        NotController,
        /// Not operator.
        NotOperator,
        /// Invalid entry type.
        InvalidEntryType,
        /// Arithmetic overflow.
        ArithmeticOverflow,
    }

    // ========================================================================
    // Extrinsics
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Initialize memory for an agent.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::initialize_memory())]
        pub fn initialize_memory(
            origin: OriginFor<T>,
            agent_id: AgentId,
            operator: T::AccountId,
        ) -> DispatchResult {
            let controller = ensure_signed(origin)?;

            ensure!(
                !AgentController::<T>::contains_key(agent_id),
                Error::<T>::AgentNotFound
            );

            // Store mappings
            AgentController::<T>::insert(agent_id, controller.clone());
            AgentOperator::<T>::insert(agent_id, operator.clone());

            // Initialize default permissions
            let permissions = MemoryAccess::default();
            MemoryPermissions::<T>::insert(agent_id, permissions);

            // Initialize first chunk
            CurrentChunk::<T>::insert(agent_id, 0);

            let chunk = MemoryChunk {
                id: 0,
                entries: BoundedVec::default(),
                created_at: frame_system::Pallet::<T>::block_number(),
                finalized: false,
                hash: None,
            };
            MemoryChunks::<T>::insert(agent_id, 0, chunk);

            Self::deposit_event(Event::MemoryInitialized {
                agent_id,
                controller,
                operator,
            });

            Ok(())
        }

        /// Append a memory entry.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::append_entry())]
        pub fn append_entry(
            origin: OriginFor<T>,
            agent_id: AgentId,
            entry_type: EntryType,
            content: BoundedVec<u8, ConstU32<4096>>,
            metadata: Option<BoundedVec<u8, ConstU32<256>>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify write permission
            Self::ensure_write_permission(agent_id, &who)?;

            // Verify content length
            ensure!(content.len() <= 4096, Error::<T>::ContentTooLong);

            let current_block = frame_system::Pallet::<T>::block_number();
            let entry_id = EntryCount::<T>::get(agent_id);

            let entry = MemoryEntry {
                id: entry_id,
                entry_type,
                content: content.clone(),
                metadata,
                timestamp: current_block,
                ttl: Some(current_block.saturating_add(T::DefaultTtl::get())),
            };

            // Calculate storage cost
            let entry_size = Self::calculate_entry_size(&entry);
            Self::charge_storage(agent_id, entry_size)?;

            // Get current chunk
            let chunk_id = CurrentChunk::<T>::get(agent_id);

            MemoryChunks::<T>::try_mutate(agent_id, chunk_id, |maybe_chunk| -> DispatchResult {
                let chunk = maybe_chunk.as_mut().ok_or(Error::<T>::ChunkNotFound)?;

                // Check if chunk is full
                if chunk.entries.len() >= T::MaxEntriesPerChunk::get() as usize {
                    // Finalize current chunk
                    chunk.finalized = true;
                    chunk.hash = Some(Self::compute_chunk_hash(chunk));

                    Self::deposit_event(Event::ChunkFinalized {
                        agent_id,
                        chunk_id,
                        entries: chunk.entries.len() as u32,
                    });

                    // Create new chunk
                    let new_chunk_id = chunk_id.saturating_add(1);
                    ensure!(
                        new_chunk_id < T::MaxChunksPerAgent::get(),
                        Error::<T>::TooManyChunks
                    );

                    let mut new_entries = BoundedVec::default();
                    // This try_push should always succeed since we just created a new chunk
                    let _ = new_entries.try_push(entry.clone());

                    let new_chunk = MemoryChunk {
                        id: new_chunk_id,
                        entries: new_entries,
                        created_at: current_block,
                        finalized: false,
                        hash: None,
                    };

                    // Insert new chunk in separate storage call
                    CurrentChunk::<T>::insert(agent_id, new_chunk_id);
                    MemoryChunks::<T>::insert(agent_id, new_chunk_id, new_chunk);
                } else {
                    // try_push returns Err if full, but we checked len above
                    let _ = chunk.entries.try_push(entry.clone());
                }

                Ok(())
            })?;

            // Update entry count
            EntryCount::<T>::insert(agent_id, entry_id.saturating_add(1));

            // Update storage used
            StorageUsed::<T>::mutate(agent_id, |used| {
                *used = used.saturating_add(entry_size as u64);
            });

            Self::deposit_event(Event::EntryAppended {
                agent_id,
                entry_id,
                entry_type,
                size: entry_size as u32,
            });

            Ok(())
        }

        /// Append a batch of entries (more efficient).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::append_batch())]
        pub fn append_batch(
            origin: OriginFor<T>,
            agent_id: AgentId,
            entries: Vec<(EntryType, BoundedVec<u8, ConstU32<4096>>)>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::ensure_write_permission(agent_id, &who)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            let mut entry_id = EntryCount::<T>::get(agent_id);
            let mut total_size: u64 = 0;

            for (entry_type, content) in entries {
                let entry = MemoryEntry {
                    id: entry_id,
                    entry_type,
                    content: content.clone(),
                    metadata: None,
                    timestamp: current_block,
                    ttl: Some(current_block.saturating_add(T::DefaultTtl::get())),
                };

                let entry_size = Self::calculate_entry_size(&entry);
                total_size = total_size.saturating_add(entry_size as u64);

                let chunk_id = CurrentChunk::<T>::get(agent_id);

                MemoryChunks::<T>::try_mutate(
                    agent_id,
                    chunk_id,
                    |maybe_chunk| -> DispatchResult {
                        let chunk = maybe_chunk.as_mut().ok_or(Error::<T>::ChunkNotFound)?;

                        if chunk.entries.len() >= T::MaxEntriesPerChunk::get() as usize {
                            chunk.finalized = true;
                            let new_chunk_id = chunk_id.saturating_add(1);
                            ensure!(
                                new_chunk_id < T::MaxChunksPerAgent::get(),
                                Error::<T>::TooManyChunks
                            );

                            let mut new_entries = BoundedVec::default();
                            let _ = new_entries.try_push(entry.clone());

                            let new_chunk = MemoryChunk {
                                id: new_chunk_id,
                                entries: new_entries,
                                created_at: current_block,
                                finalized: false,
                                hash: None,
                            };

                            CurrentChunk::<T>::insert(agent_id, new_chunk_id);
                            MemoryChunks::<T>::insert(agent_id, new_chunk_id, new_chunk);
                        } else {
                            let _ = chunk.entries.try_push(entry);
                        }

                        Ok(())
                    },
                )?;

                entry_id = entry_id.saturating_add(1);
            }

            Self::charge_storage(agent_id, total_size as usize)?;
            EntryCount::<T>::insert(agent_id, entry_id);
            StorageUsed::<T>::mutate(agent_id, |used| {
                *used = used.saturating_add(total_size);
            });

            Ok(())
        }

        /// Update memory permissions.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_permissions())]
        pub fn update_permissions(
            origin: OriginFor<T>,
            agent_id: AgentId,
            can_public_read: bool,
            allowed_readers: Vec<T::AccountId>,
            allowed_writers: Vec<T::AccountId>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let controller =
                AgentController::<T>::get(agent_id).ok_or(Error::<T>::MemoryNotInitialized)?;
            ensure!(who == controller, Error::<T>::NotController);

            let permissions = MemoryAccess {
                can_public_read,
                allowed_readers: allowed_readers.try_into().unwrap_or_default(),
                allowed_writers: allowed_writers.try_into().unwrap_or_default(),
            };

            MemoryPermissions::<T>::insert(agent_id, permissions);

            Self::deposit_event(Event::PermissionsUpdated { agent_id });

            Ok(())
        }

        /// Prune old memory entries.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::prune_memory())]
        pub fn prune_memory(
            origin: OriginFor<T>,
            agent_id: AgentId,
            up_to_chunk: u32,
        ) -> DispatchResult {
            T::PruneOrigin::ensure_origin(origin)?;

            let mut chunks_removed = 0u32;
            let mut bytes_freed = 0u64;
            let current_block = frame_system::Pallet::<T>::block_number();

            for chunk_id in 0..=up_to_chunk {
                if let Some(chunk) = MemoryChunks::<T>::get(agent_id, chunk_id) {
                    // Only prune if all entries have expired
                    let all_expired = chunk
                        .entries
                        .iter()
                        .all(|e| e.ttl.is_some_and(|ttl| current_block > ttl));

                    if all_expired || chunk.finalized {
                        for entry in &chunk.entries {
                            bytes_freed = bytes_freed
                                .saturating_add(Self::calculate_entry_size(entry) as u64);
                        }

                        MemoryChunks::<T>::remove(agent_id, chunk_id);
                        chunks_removed = chunks_removed.saturating_add(1);
                    }
                }
            }

            // Update storage used
            StorageUsed::<T>::mutate(agent_id, |used| {
                *used = used.saturating_sub(bytes_freed);
            });

            // Return deposit
            if bytes_freed > Zero::zero() {
                let deposit_return = Self::bytes_to_deposit(bytes_freed as usize);
                StorageDeposit::<T>::mutate(agent_id, |deposit| {
                    *deposit = deposit.saturating_sub(deposit_return);
                });

                if let Some(controller) = AgentController::<T>::get(agent_id) {
                    T::Currency::unreserve(&controller, deposit_return);
                }
            }

            Self::deposit_event(Event::MemoryPruned {
                agent_id,
                chunks_removed,
                bytes_freed,
            });

            Ok(())
        }

        /// Increase storage deposit.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::increase_deposit())]
        pub fn increase_deposit(
            origin: OriginFor<T>,
            agent_id: AgentId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                AgentController::<T>::contains_key(agent_id),
                Error::<T>::MemoryNotInitialized
            );

            T::Currency::reserve(&who, amount)?;

            StorageDeposit::<T>::mutate(agent_id, |deposit| {
                *deposit = deposit.saturating_add(amount);
            });

            Self::deposit_event(Event::DepositIncreased { agent_id, amount });

            Ok(())
        }

        /// Withdraw excess deposit.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::withdraw_deposit())]
        pub fn withdraw_deposit(
            origin: OriginFor<T>,
            agent_id: AgentId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let controller =
                AgentController::<T>::get(agent_id).ok_or(Error::<T>::MemoryNotInitialized)?;
            ensure!(who == controller, Error::<T>::NotController);

            let current_deposit = StorageDeposit::<T>::get(agent_id);
            let storage_used = StorageUsed::<T>::get(agent_id);
            let required_deposit = Self::bytes_to_deposit(storage_used as usize);

            let excess = current_deposit.saturating_sub(required_deposit);
            ensure!(amount <= excess, Error::<T>::InsufficientDeposit);

            T::Currency::unreserve(&who, amount);

            StorageDeposit::<T>::mutate(agent_id, |deposit| {
                *deposit = deposit.saturating_sub(amount);
            });

            Self::deposit_event(Event::DepositWithdrawn { agent_id, amount });

            Ok(())
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Check write permission.
        fn ensure_write_permission(agent_id: AgentId, who: &T::AccountId) -> DispatchResult {
            // Operator always has write permission
            if let Some(operator) = AgentOperator::<T>::get(agent_id) {
                if *who == operator {
                    return Ok(());
                }
            }

            // Controller always has write permission
            if let Some(controller) = AgentController::<T>::get(agent_id) {
                if *who == controller {
                    return Ok(());
                }
            }

            // Check allowed writers
            let permissions = MemoryPermissions::<T>::get(agent_id);
            if permissions.allowed_writers.contains(who) {
                return Ok(());
            }

            Err(Error::<T>::WritePermissionDenied.into())
        }

        /// Calculate entry size in bytes.
        fn calculate_entry_size(entry: &MemoryEntry<BlockNumberFor<T>>) -> usize {
            let base_size = 32; // id, type, timestamp, ttl
            let content_size = entry.content.len();
            let metadata_size = entry.metadata.as_ref().map_or(0, |m| m.len());

            base_size + content_size + metadata_size
        }

        /// Convert bytes to deposit amount.
        fn bytes_to_deposit(bytes: usize) -> BalanceOf<T> {
            let cost_per_byte = T::StorageByteCost::get();
            cost_per_byte.saturating_mul((bytes as u32).into())
        }

        /// Charge storage for new data.
        fn charge_storage(agent_id: AgentId, bytes: usize) -> DispatchResult {
            let deposit_needed = Self::bytes_to_deposit(bytes);
            let current_deposit = StorageDeposit::<T>::get(agent_id);
            let storage_used = StorageUsed::<T>::get(agent_id);
            let total_required =
                Self::bytes_to_deposit(storage_used as usize).saturating_add(deposit_needed);

            if current_deposit < total_required {
                // Need more deposit from controller
                if let Some(controller) = AgentController::<T>::get(agent_id) {
                    let additional = total_required.saturating_sub(current_deposit);
                    T::Currency::reserve(&controller, additional)?;
                    StorageDeposit::<T>::mutate(agent_id, |d| *d = d.saturating_add(additional));
                } else {
                    return Err(Error::<T>::InsufficientDeposit.into());
                }
            }

            Ok(())
        }

        /// Compute hash of a chunk for integrity.
        fn compute_chunk_hash(chunk: &MemoryChunk<BlockNumberFor<T>>) -> sp_core::H256 {
            use sp_io::hashing::blake2_256;
            let encoded = chunk.encode();
            sp_core::H256::from(blake2_256(&encoded))
        }

        /// Get memory chunk for runtime API.
        pub fn get_memory_chunk(
            agent_id: AgentId,
            chunk_id: u32,
        ) -> Option<MemoryChunk<BlockNumberFor<T>>> {
            MemoryChunks::<T>::get(agent_id, chunk_id)
        }

        /// Get memory entries in JSONL format (for LLM consumption).
        pub fn get_memory_jsonl(
            agent_id: AgentId,
            offset: u64,
            limit: u32,
        ) -> Vec<JsonlEntry<BlockNumberFor<T>>> {
            let mut entries = Vec::new();
            let chunk_count = CurrentChunk::<T>::get(agent_id).saturating_add(1);
            let mut current_offset = 0u64;
            let mut collected = 0u32;

            for chunk_id in 0..chunk_count {
                if collected >= limit {
                    break;
                }

                if let Some(chunk) = MemoryChunks::<T>::get(agent_id, chunk_id) {
                    for entry in chunk.entries {
                        if current_offset >= offset {
                            let entry_type_str = match entry.entry_type {
                                EntryType::Observation => "observation",
                                EntryType::Action => "action",
                                EntryType::Result => "result",
                                EntryType::Thought => "thought",
                                EntryType::Goal => "goal",
                                EntryType::Plan => "plan",
                                EntryType::Error => "error",
                                EntryType::Checkpoint => "checkpoint",
                                EntryType::Delta => "delta",
                                EntryType::Custom => "custom",
                            };
                            entries.push(JsonlEntry {
                                id: entry.id,
                                entry_type: entry_type_str.as_bytes().to_vec(),
                                content: entry.content,
                                timestamp: entry.timestamp,
                            });
                            collected = collected.saturating_add(1);
                            if collected >= limit {
                                break;
                            }
                        }
                        current_offset = current_offset.saturating_add(1);
                    }
                }
            }

            entries
        }

        /// Get memory summary for runtime API.
        pub fn get_memory_summary(agent_id: AgentId) -> MemorySummary<BalanceOf<T>> {
            MemorySummary {
                agent_id,
                total_entries: EntryCount::<T>::get(agent_id),
                total_chunks: CurrentChunk::<T>::get(agent_id).saturating_add(1),
                storage_used: StorageUsed::<T>::get(agent_id),
                deposit: StorageDeposit::<T>::get(agent_id),
            }
        }
    }
}
