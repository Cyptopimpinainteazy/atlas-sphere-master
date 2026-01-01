//! Runtime API for Agent Memory pallet.
//!
//! Provides offchain access to agent memory chunks.

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Memory entry for API response.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryEntryResponse {
    /// Entry ID.
    pub id: u64,
    /// Entry type (as string).
    pub entry_type: Vec<u8>,
    /// Content (JSON bytes).
    pub content: Vec<u8>,
    /// Metadata.
    pub metadata: Option<Vec<u8>>,
    /// Block timestamp.
    pub timestamp: u64,
    /// TTL (blocks until expiration).
    pub ttl: Option<u64>,
}

/// Memory chunk response for API.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryChunkResponse {
    /// Chunk index.
    pub chunk_index: u32,
    /// Agent ID.
    pub agent_id: u32,
    /// Whether chunk is finalized.
    pub finalized: bool,
    /// Number of entries.
    pub entry_count: u32,
    /// Entries in this chunk.
    pub entries: Vec<MemoryEntryResponse>,
    /// Start block.
    pub start_block: u64,
    /// End block (if finalized).
    pub end_block: Option<u64>,
}

/// JSONL-formatted memory dump.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct MemoryJsonlResponse {
    /// JSONL lines (each line is a JSON object).
    pub lines: Vec<Vec<u8>>,
    /// Total entries.
    pub total: u32,
    /// Whether there are more entries.
    pub has_more: bool,
    /// Cursor for pagination.
    pub cursor: Option<u64>,
}

sp_api::decl_runtime_apis! {
    /// Agent Memory Runtime API.
    pub trait AgentMemoryApi {
        /// Get a memory chunk by agent and index.
        fn get_memory_chunk(agent_id: u32, chunk_index: u32) -> Option<MemoryChunkResponse>;

        /// Get latest N entries for an agent.
        fn get_latest_entries(agent_id: u32, count: u32) -> Vec<MemoryEntryResponse>;

        /// Get entries as JSONL format (LLM-friendly).
        fn get_memory_jsonl(agent_id: u32, from_id: u64, limit: u32) -> MemoryJsonlResponse;

        /// Get total chunk count for agent.
        fn get_chunk_count(agent_id: u32) -> u32;

        /// Get entry count for agent.
        fn get_entry_count(agent_id: u32) -> u64;
    }
}
