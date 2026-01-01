//! Types for the Agent Memory pallet.

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Maximum entries per chunk (matches runtime config)
pub const MAX_ENTRIES_PER_CHUNK: u32 = 100;

/// Type of memory entry.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum EntryType {
    /// Observation from environment.
    #[default]
    Observation,
    /// Action taken by agent.
    Action,
    /// Result of an action.
    Result,
    /// Thought or reasoning step.
    Thought,
    /// Goal or objective.
    Goal,
    /// Plan or strategy.
    Plan,
    /// Error or exception.
    Error,
    /// State checkpoint.
    Checkpoint,
    /// Delta from previous state.
    Delta,
    /// Custom type.
    Custom,
}

/// A single memory entry.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct MemoryEntry<BlockNumber> {
    /// Entry ID (sequential within agent).
    pub id: u64,
    /// Type of entry.
    pub entry_type: EntryType,
    /// Content (typically JSON).
    pub content: BoundedVec<u8, ConstU32<4096>>,
    /// Optional metadata.
    pub metadata: Option<BoundedVec<u8, ConstU32<256>>>,
    /// Block when created.
    pub timestamp: BlockNumber,
    /// Expiration block (None = never expires).
    pub ttl: Option<BlockNumber>,
}

/// A chunk of memory entries (bounded to prevent storage attacks).
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct MemoryChunk<BlockNumber> {
    /// Chunk ID.
    pub id: u32,
    /// Entries in this chunk (bounded by MAX_ENTRIES_PER_CHUNK).
    pub entries: BoundedVec<MemoryEntry<BlockNumber>, ConstU32<MAX_ENTRIES_PER_CHUNK>>,
    /// Block when created.
    pub created_at: BlockNumber,
    /// Whether chunk is finalized (no more entries).
    pub finalized: bool,
    /// Hash for integrity verification.
    pub hash: Option<sp_core::H256>,
}

impl<BlockNumber: Default> Default for MemoryChunk<BlockNumber> {
    fn default() -> Self {
        Self {
            id: 0,
            entries: BoundedVec::default(),
            created_at: BlockNumber::default(),
            finalized: false,
            hash: None,
        }
    }
}

/// Memory access permissions.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub struct MemoryAccess<AccountId> {
    /// Whether anyone can read.
    pub can_public_read: bool,
    /// Specific accounts allowed to read.
    pub allowed_readers: BoundedVec<AccountId, ConstU32<100>>,
    /// Specific accounts allowed to write.
    pub allowed_writers: BoundedVec<AccountId, ConstU32<10>>,
}

impl<AccountId> Default for MemoryAccess<AccountId> {
    fn default() -> Self {
        Self {
            can_public_read: false,
            allowed_readers: BoundedVec::default(),
            allowed_writers: BoundedVec::default(),
        }
    }
}

/// JSONL-friendly entry format for LLM consumption.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct JsonlEntry<BlockNumber> {
    /// Entry ID.
    pub id: u64,
    /// Entry type as string.
    pub entry_type: sp_std::vec::Vec<u8>,
    /// Content.
    pub content: BoundedVec<u8, ConstU32<4096>>,
    /// Timestamp.
    pub timestamp: BlockNumber,
}

/// Memory summary for API responses.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MemorySummary<Balance> {
    /// Agent ID.
    pub agent_id: u32,
    /// Total entries.
    pub total_entries: u64,
    /// Total chunks.
    pub total_chunks: u32,
    /// Storage used in bytes.
    pub storage_used: u64,
    /// Current deposit.
    pub deposit: Balance,
}

/// Delta entry for compressed updates.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct DeltaEntry<BlockNumber> {
    /// Reference entry ID.
    pub base_id: u64,
    /// Fields that changed.
    pub changes: BoundedVec<FieldChange, ConstU32<32>>,
    /// Timestamp.
    pub timestamp: BlockNumber,
}

/// A single field change in a delta.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub struct FieldChange {
    /// Field path (e.g., "state.balance").
    pub path: BoundedVec<u8, ConstU32<64>>,
    /// Operation type.
    pub operation: ChangeOperation,
    /// New value (for set/add operations).
    pub value: BoundedVec<u8, ConstU32<256>>,
}

/// Type of change operation.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub enum ChangeOperation {
    /// Set field to value.
    Set,
    /// Add to numeric field.
    Add,
    /// Subtract from numeric field.
    Subtract,
    /// Append to array.
    Append,
    /// Remove field.
    Remove,
}

/// Memory statistics.
#[derive(Clone, Default, Encode, Decode, TypeInfo, Debug)]
pub struct MemoryStats {
    /// Total entries across all agents.
    pub total_entries: u64,
    /// Total storage used.
    pub total_storage: u64,
    /// Active agents with memory.
    pub active_agents: u32,
}
