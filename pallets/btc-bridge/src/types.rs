//! Data types for BTC bridge

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// BTC block header (80 bytes)
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub struct BtcBlockHeader {
    pub version: u32,
    pub prev_block: H256,
    pub merkle_root: H256,
    pub time: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BtcBlockHeader {
    /// Compute SHA256(SHA256(header)) = block hash
    pub fn hash(&self) -> H256 {
        // In production, use proper Bitcoin header hashing
        // For now, use blake2 as placeholder
        sp_io::hashing::blake2_256(&self.encode()).into()
    }
}

/// Merkle proof for BTC tx inclusion
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct MerkleProof {
    pub hashes: Vec<H256>,
    pub indices: Vec<u8>,
}

/// Peg-in request status
#[derive(Debug, Clone, Copy, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    Confirmed,
    Rejected,
}

/// Peg-in request information
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct PeginRequest<AccountId> {
    pub btc_tx_hash: H256,
    pub requester: AccountId,
    pub recipient: AccountId,
    pub amount: u64,
    pub requested_at: u32,
    pub status: RequestStatus,
}

/// Bridge configuration
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct BridgeConfiguration {
    pub min_pegin_amount: u64,
    pub max_pegin_amount: u64,
    pub min_pegout_amount: u64,
    pub max_pegout_amount: u64,
    pub pegin_fee_bps: u16,  // basis points (0.01%)
    pub pegout_fee_bps: u16,
}

impl Default for BridgeConfiguration {
    fn default() -> Self {
        Self {
            min_pegin_amount: 100_000,        // 0.001 BTC
            max_pegin_amount: 100_000_000_000, // 1000 BTC
            min_pegout_amount: 100_000,
            max_pegout_amount: 100_000_000_000,
            pegin_fee_bps: 50,   // 0.5%
            pegout_fee_bps: 50,
        }
    }
}
