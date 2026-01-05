#![cfg_attr(not(feature = "std"), no_std)]

//! # ASGA Receipts
//!
//! Canonical receipt types for atomic cross-chain swaps.
//! This crate defines the SCALE-encoded receipt formats for all supported domains:
//! - EVM (Ethereum, Polygon, etc.)
//! - SVM (Solana)
//! - BTC (Bitcoin)
//! - X3 (Atlas Sphere native)
//!
//! ## Receipt Structure
//!
//! Each receipt consists of:
//! 1. **Header**: Common fields (intent_id, domain, phase, amount, asset, timestamp, signer)
//! 2. **Payload**: Domain-specific transaction proof
//! 3. **Attestation**: Validator signature over the receipt
//!
//! ## Validation
//!
//! Receipts are validated by:
//! 1. Checking the header fields match the expected intent
//! 2. Verifying the payload matches the declared domain
//! 3. Verifying the attestation signature
//! 4. (Off-chain) Verifying the underlying transaction on the source chain

// Support allocation types in no_std
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use parity_scale_codec::MaxEncodedLen;

/// Domain enums
#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug, TypeInfo, MaxEncodedLen)]
pub enum DomainId {
    Evm = 0,
    Svm = 1,
    Btc = 2,
    X3 = 3,
}

impl DomainId {
    /// Returns the expected minimum confirmations for finality on this domain
    pub fn min_confirmations(&self) -> u32 {
        match self {
            DomainId::Evm => 12,    // ~3 minutes on Ethereum mainnet
            DomainId::Svm => 32,    // ~12 seconds on Solana
            DomainId::Btc => 6,     // ~60 minutes on Bitcoin
            DomainId::X3 => 2,      // 2 blocks on X3 (GRANDPA finality)
        }
    }
}

/// Phase enum
#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug, TypeInfo, MaxEncodedLen)]
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

impl ReceiptHeader {
    /// Create a new receipt header
    pub fn new(
        intent_id: [u8; 32],
        domain_id: DomainId,
        phase: Phase,
        amount: u128,
        asset_id: [u8; 32],
        timestamp: u64,
        signer: Vec<u8>,
    ) -> Self {
        Self {
            intent_id,
            domain_id,
            phase,
            amount,
            asset_id,
            timestamp,
            signer,
        }
    }

    /// Validate header fields
    pub fn validate(&self, expected_intent: &[u8; 32]) -> Result<(), ValidationError> {
        if self.intent_id != *expected_intent {
            return Err(ValidationError::IntentMismatch);
        }
        if self.amount == 0 {
            return Err(ValidationError::ZeroAmount);
        }
        if self.signer.is_empty() {
            return Err(ValidationError::EmptySigner);
        }
        Ok(())
    }
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

impl EvmReceipt {
    /// Validate EVM receipt fields
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.tx_hash == [0u8; 32] {
            return Err(ValidationError::EmptyTxHash);
        }
        if self.contract_address == [0u8; 20] {
            return Err(ValidationError::EmptyContractAddress);
        }
        if self.confirmations < DomainId::Evm.min_confirmations() {
            return Err(ValidationError::InsufficientConfirmations {
                got: self.confirmations,
                expected: DomainId::Evm.min_confirmations(),
            });
        }
        Ok(())
    }
}

/// SVM payload (Solana)
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct SvmReceipt {
    pub signature: [u8; 64],
    pub slot: u64,
    pub program_id: [u8; 32],
    pub escrow_pda: [u8; 32],
}

impl SvmReceipt {
    /// Validate SVM receipt fields
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.signature == [0u8; 64] {
            return Err(ValidationError::EmptySignature);
        }
        if self.program_id == [0u8; 32] {
            return Err(ValidationError::EmptyProgramId);
        }
        if self.slot == 0 {
            return Err(ValidationError::InvalidSlot);
        }
        Ok(())
    }
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

impl BtcReceipt {
    /// Validate BTC receipt fields
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.txid == [0u8; 32] {
            return Err(ValidationError::EmptyTxHash);
        }
        if self.confirmations < DomainId::Btc.min_confirmations() {
            return Err(ValidationError::InsufficientConfirmations {
                got: self.confirmations,
                expected: DomainId::Btc.min_confirmations(),
            });
        }
        Ok(())
    }
}

/// X3 payload
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct X3Receipt {
    pub block_hash: [u8; 32],
    pub runtime_version: u32,
    pub arbiter_signature: [u8; 64],
}

impl X3Receipt {
    /// Validate X3 receipt fields
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.block_hash == [0u8; 32] {
            return Err(ValidationError::EmptyBlockHash);
        }
        if self.arbiter_signature == [0u8; 64] {
            return Err(ValidationError::EmptySignature);
        }
        Ok(())
    }
}

/// Attestation scheme
#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug, TypeInfo, MaxEncodedLen)]
pub enum AttestationScheme {
    Sr25519 = 0,
    Ed25519 = 1,
    Secp256k1 = 2,
}

impl AttestationScheme {
    /// Expected signature length for this scheme
    pub fn signature_len(&self) -> u32 {
        match self {
            AttestationScheme::Sr25519 => 64,
            AttestationScheme::Ed25519 => 64,
            AttestationScheme::Secp256k1 => 65, // includes recovery id
        }
    }
}

/// Attestation produced by an attester (validator) asserting the canonical receipt.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct Attestation {
    pub attester_pubkey: [u8; 32],
    pub scheme: AttestationScheme,
    pub signature: Vec<u8>, // scheme-specific signature bytes (e.g., 64 bytes for sr25519)
}

impl Attestation {
    /// Create a new attestation
    pub fn new(attester_pubkey: [u8; 32], scheme: AttestationScheme, signature: Vec<u8>) -> Self {
        Self {
            attester_pubkey,
            scheme,
            signature,
        }
    }

    /// Validate attestation format (does not verify signature)
    pub fn validate_format(&self) -> Result<(), ValidationError> {
        if self.attester_pubkey == [0u8; 32] {
            return Err(ValidationError::EmptyAttester);
        }
        if self.signature.len() as u32 != self.scheme.signature_len() {
            return Err(ValidationError::InvalidSignatureLength {
                got: self.signature.len() as u32,
                expected: self.scheme.signature_len(),
            });
        }
        Ok(())
    }
}

/// Domain-specific payload union.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub enum ReceiptPayload {
    Evm(EvmReceipt),
    Svm(SvmReceipt),
    Btc(BtcReceipt),
    X3(X3Receipt),
}

impl ReceiptPayload {
    /// Get the domain ID for this payload
    pub fn domain(&self) -> DomainId {
        match self {
            ReceiptPayload::Evm(_) => DomainId::Evm,
            ReceiptPayload::Svm(_) => DomainId::Svm,
            ReceiptPayload::Btc(_) => DomainId::Btc,
            ReceiptPayload::X3(_) => DomainId::X3,
        }
    }

    /// Validate the payload contents
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            ReceiptPayload::Evm(r) => r.validate(),
            ReceiptPayload::Svm(r) => r.validate(),
            ReceiptPayload::Btc(r) => r.validate(),
            ReceiptPayload::X3(r) => r.validate(),
        }
    }
}

/// Canonical receipt container.
///
/// This is the payload that should be SCALE-encoded and attested by validators.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct Receipt {
    pub header: ReceiptHeader,
    pub payload: ReceiptPayload,
}

impl Receipt {
    /// Create a new receipt
    pub fn new(header: ReceiptHeader, payload: ReceiptPayload) -> Self {
        Self { header, payload }
    }

    /// Validate the receipt structure
    pub fn validate(&self, expected_intent: &[u8; 32]) -> Result<(), ValidationError> {
        // Validate header
        self.header.validate(expected_intent)?;

        // Validate payload matches declared domain
        if self.header.domain_id != self.payload.domain() {
            return Err(ValidationError::DomainMismatch {
                header: self.header.domain_id,
                payload: self.payload.domain(),
            });
        }

        // Validate payload contents
        self.payload.validate()?;

        Ok(())
    }

    /// Get the canonical bytes for signing
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.encode()
    }
}

/// Attested receipt submitted to X3.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo)]
pub struct AttestedReceipt {
    pub receipt: Receipt,
    pub attestation: Attestation,
}

impl AttestedReceipt {
    /// Create a new attested receipt
    pub fn new(receipt: Receipt, attestation: Attestation) -> Self {
        Self { receipt, attestation }
    }

    /// Validate the attested receipt structure (does not verify cryptographic signature)
    pub fn validate(&self, expected_intent: &[u8; 32]) -> Result<(), ValidationError> {
        self.receipt.validate(expected_intent)?;
        self.attestation.validate_format()?;
        Ok(())
    }
}

/// Validation error types
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub enum ValidationError {
    /// Intent ID doesn't match
    IntentMismatch,
    /// Amount is zero
    ZeroAmount,
    /// Signer field is empty
    EmptySigner,
    /// Transaction hash is empty
    EmptyTxHash,
    /// Contract address is empty
    EmptyContractAddress,
    /// Insufficient confirmations for finality
    InsufficientConfirmations { got: u32, expected: u32 },
    /// Signature is empty
    EmptySignature,
    /// Program ID is empty
    EmptyProgramId,
    /// Slot is invalid
    InvalidSlot,
    /// Block hash is empty
    EmptyBlockHash,
    /// Attester pubkey is empty
    EmptyAttester,
    /// Invalid signature length (use u32 for SCALE compatibility)
    InvalidSignatureLength { got: u32, expected: u32 },
    /// Domain mismatch between header and payload
    DomainMismatch { header: DomainId, payload: DomainId },
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

    #[test]
    fn receipt_encodes_and_decodes() {
        let receipt = Receipt {
            header: ReceiptHeader {
                intent_id: [9u8; 32],
                domain_id: DomainId::Evm,
                phase: Phase::Lock,
                amount: 123u128,
                asset_id: [1u8; 32],
                timestamp: 7u64,
                signer: vec![2u8; 33],
            },
            payload: ReceiptPayload::Evm(EvmReceipt {
                tx_hash: [3u8; 32],
                block_number: 1u64,
                confirmations: 12u32,
                contract_address: [4u8; 20],
                calldata_hash: [5u8; 32],
            }),
        };

        let enc = receipt.encode();
        let dec = Receipt::decode(&mut &enc[..]).expect("decode receipt");
        assert_eq!(receipt, dec);
    }

    #[test]
    fn test_receipt_validation_happy_path() {
        let intent_id = [42u8; 32];
        let receipt = Receipt {
            header: ReceiptHeader {
                intent_id,
                domain_id: DomainId::Evm,
                phase: Phase::Lock,
                amount: 1000u128,
                asset_id: [1u8; 32],
                timestamp: 1700000000u64,
                signer: vec![2u8; 33],
            },
            payload: ReceiptPayload::Evm(EvmReceipt {
                tx_hash: [3u8; 32],
                block_number: 1000u64,
                confirmations: 15u32,
                contract_address: [4u8; 20],
                calldata_hash: [5u8; 32],
            }),
        };

        assert!(receipt.validate(&intent_id).is_ok());
    }

    #[test]
    fn test_receipt_validation_intent_mismatch() {
        let intent_id = [42u8; 32];
        let wrong_intent = [99u8; 32];
        let receipt = Receipt {
            header: ReceiptHeader {
                intent_id: wrong_intent,
                domain_id: DomainId::Evm,
                phase: Phase::Lock,
                amount: 1000u128,
                asset_id: [1u8; 32],
                timestamp: 1700000000u64,
                signer: vec![2u8; 33],
            },
            payload: ReceiptPayload::Evm(EvmReceipt {
                tx_hash: [3u8; 32],
                block_number: 1000u64,
                confirmations: 15u32,
                contract_address: [4u8; 20],
                calldata_hash: [5u8; 32],
            }),
        };

        assert_eq!(receipt.validate(&intent_id), Err(ValidationError::IntentMismatch));
    }

    #[test]
    fn test_receipt_validation_domain_mismatch() {
        let intent_id = [42u8; 32];
        let receipt = Receipt {
            header: ReceiptHeader {
                intent_id,
                domain_id: DomainId::Svm, // Header says SVM
                phase: Phase::Lock,
                amount: 1000u128,
                asset_id: [1u8; 32],
                timestamp: 1700000000u64,
                signer: vec![2u8; 33],
            },
            payload: ReceiptPayload::Evm(EvmReceipt { // But payload is EVM
                tx_hash: [3u8; 32],
                block_number: 1000u64,
                confirmations: 15u32,
                contract_address: [4u8; 20],
                calldata_hash: [5u8; 32],
            }),
        };

        assert!(matches!(receipt.validate(&intent_id), Err(ValidationError::DomainMismatch { .. })));
    }

    #[test]
    fn test_evm_insufficient_confirmations() {
        let evm = EvmReceipt {
            tx_hash: [3u8; 32],
            block_number: 1000u64,
            confirmations: 5u32, // Less than 12 required
            contract_address: [4u8; 20],
            calldata_hash: [5u8; 32],
        };

        assert!(matches!(evm.validate(), Err(ValidationError::InsufficientConfirmations { .. })));
    }

    #[test]
    fn test_attestation_validation() {
        let valid_attestation = Attestation {
            attester_pubkey: [1u8; 32],
            scheme: AttestationScheme::Sr25519,
            signature: vec![0u8; 64],
        };
        assert!(valid_attestation.validate_format().is_ok());

        let invalid_attestation = Attestation {
            attester_pubkey: [1u8; 32],
            scheme: AttestationScheme::Sr25519,
            signature: vec![0u8; 32], // Wrong length
        };
        assert!(matches!(invalid_attestation.validate_format(), Err(ValidationError::InvalidSignatureLength { .. })));
    }

    #[test]
    fn test_domain_min_confirmations() {
        assert_eq!(DomainId::Evm.min_confirmations(), 12);
        assert_eq!(DomainId::Svm.min_confirmations(), 32);
        assert_eq!(DomainId::Btc.min_confirmations(), 6);
        assert_eq!(DomainId::X3.min_confirmations(), 2);
    }

    #[test]
    fn test_svm_receipt_validation() {
        let valid = SvmReceipt {
            signature: [1u8; 64],
            slot: 12345u64,
            program_id: [2u8; 32],
            escrow_pda: [3u8; 32],
        };
        assert!(valid.validate().is_ok());

        let invalid = SvmReceipt {
            signature: [0u8; 64], // Empty
            slot: 12345u64,
            program_id: [2u8; 32],
            escrow_pda: [3u8; 32],
        };
        assert_eq!(invalid.validate(), Err(ValidationError::EmptySignature));
    }

    #[test]
    fn test_btc_receipt_validation() {
        let valid = BtcReceipt {
            txid: [1u8; 32],
            vout: 0,
            script_hash: [2u8; 20],
            confirmations: 6,
            locktime: 0,
        };
        assert!(valid.validate().is_ok());

        let insufficient_conf = BtcReceipt {
            txid: [1u8; 32],
            vout: 0,
            script_hash: [2u8; 20],
            confirmations: 3, // Less than 6
            locktime: 0,
        };
        assert!(matches!(insufficient_conf.validate(), Err(ValidationError::InsufficientConfirmations { .. })));
    }

    #[test]
    fn test_x3_receipt_validation() {
        let valid = X3Receipt {
            block_hash: [1u8; 32],
            runtime_version: 100,
            arbiter_signature: [2u8; 64],
        };
        assert!(valid.validate().is_ok());

        let invalid = X3Receipt {
            block_hash: [0u8; 32], // Empty
            runtime_version: 100,
            arbiter_signature: [2u8; 64],
        };
        assert_eq!(invalid.validate(), Err(ValidationError::EmptyBlockHash));
    }
}
