#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

/// Domain enums
#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug, TypeInfo)]
pub enum DomainId {
    Evm = 0,
    Svm = 1,
    Btc = 2,
    X3 = 3,
}

/// Phase enum
#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug, TypeInfo)]
pub enum Phase {
    Lock = 0,
    Exec = 1,
    Final = 2,
}

/// Common receipt header
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct ReceiptHeader {
    pub intent_id: [u8; 32],
    pub domain_id: DomainId,
    pub phase: Phase,
    pub amount: u128,
    pub asset_id: [u8; 32],
    pub timestamp: u64,
    pub signer: Vec<u8>, // flexible signer bytes (e.g., secp256k1 compressed or sr25519 pubkey bytes)
}

/// EVM payload
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct EvmReceipt {
    pub tx_hash: [u8; 32],
    pub block_number: u64,
    pub confirmations: u32,
    pub contract_address: [u8; 20],
    pub calldata_hash: [u8; 32],
}

/// SVM payload (Solana)
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct SvmReceipt {
    pub signature: [u8; 64],
    pub slot: u64,
    pub program_id: [u8; 32],
    pub escrow_pda: [u8; 32],
}

/// BTC payload
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct BtcReceipt {
    pub txid: [u8; 32],
    pub vout: u32,
    pub script_hash: [u8; 20],
    pub confirmations: u32,
    pub locktime: u32,
}

/// X3 payload
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct X3Receipt {
    pub block_hash: [u8; 32],
    pub runtime_version: u32,
    pub arbiter_signature: [u8; 64],
}

/// Attestation scheme
#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug, TypeInfo)]
pub enum AttestationScheme {
    Sr25519 = 0,
}

/// Attestation produced by an attester (validator) asserting the canonical receipt.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct Attestation {
    pub attester_pubkey: [u8; 32],
    pub scheme: AttestationScheme,
    pub signature: Vec<u8>, // scheme-specific signature bytes (e.g., 64 bytes for sr25519)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::FromHex;

    #[test]
    fn header_encode_decode() {
        let header = ReceiptHeader {
            intent_id: [0u8; 32],
            domain_id: DomainId::Evm,
            phase: Phase::Lock,
            amount: 1_000_000u128,
            asset_id: [1u8; 32],
            timestamp: 1_700_000_000u64,
            signer: vec![2u8; 33],
        };

        let enc = header.encode();
        let dec = ReceiptHeader::decode(&mut &enc[..]).expect("decode header");
        assert_eq!(header, dec);
    }

    #[test]
    fn evm_payload_encode_decode() {
        let evm = EvmReceipt {
            tx_hash: [3u8; 32],
            block_number: 12345u64,
            confirmations: 12u32,
            contract_address: [4u8; 20],
            calldata_hash: [5u8; 32],
        };
        let enc = evm.encode();
        let dec = EvmReceipt::decode(&mut &enc[..]).expect("decode evm");
        assert_eq!(evm, dec);
    }

    #[test]
    fn canonical_encoding_is_deterministic() {
        // same struct encodes the same bytes
        let a = ReceiptHeader {
            intent_id: [7u8; 32],
            domain_id: DomainId::Btc,
            phase: Phase::Exec,
            amount: 42u128,
            asset_id: [9u8; 32],
            timestamp: 2u64,
            signer: vec![11u8; 33],
        };
        let b = ReceiptHeader { ..a.clone() };
        assert_eq!(a.encode(), b.encode());
    }
}
