use asga_receipts::{Attestation, AttestationScheme, DomainId, EvmReceipt, ReceiptHeader};
use parity_scale_codec::Encode;
use secp256k1::{
    ecdsa::RecoverableSignature, ecdsa::RecoveryId, Message, Secp256k1,
};
use sp_core::{sr25519, Pair};
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError {
    WrongDomain,
    InsufficientConfirmations,
    MissingField,
    InvalidSignature,
    SignatureRecoverFailed,
    SignerMismatch,
    InvalidAttestation,
}

/// Compute Keccak256 over bytes
fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    let mut out = [0u8; 32];
    hasher.finalize(&mut out);
    out
}

/// Verify a recoverable ECDSA signature (65 bytes: r(32)|s(32)|v(1)) against the SCALE-encoded header+payload.
/// Steps:
/// 1. Compute keccak256(header.encode() || payload.encode())
/// 2. Recover pubkey from signature and compare compressed pubkey bytes to `header.signer`.
pub fn verify_evm_signature(
    header: &ReceiptHeader,
    evm: &EvmReceipt,
    signature: &[u8],
) -> Result<(), ValidationError> {
    if signature.len() != 65 {
        return Err(ValidationError::InvalidSignature);
    }

    // prepare message: SCALE-encoded header + payload
    let mut msg_bytes = header.encode();
    msg_bytes.extend(evm.encode());

    let msg_hash = keccak256(&msg_bytes);

    // secp256k1 expects a 32-byte message
    let msg =
        Message::from_slice(&msg_hash).map_err(|_| ValidationError::SignatureRecoverFailed)?;

    let mut compact = [0u8; 64];
    compact.copy_from_slice(&signature[0..64]);
    let v = signature[64];
    // normalize v: accept 27/28 or 0/1
    let recid = match v {
        27 | 28 => {
            RecoveryId::from_i32((v - 27) as i32).map_err(|_| ValidationError::InvalidSignature)?
        }
        0 | 1 => RecoveryId::from_i32(v as i32).map_err(|_| ValidationError::InvalidSignature)?,
        _ => return Err(ValidationError::InvalidSignature),
    };

    let rec_sig = RecoverableSignature::from_compact(&compact, recid)
        .map_err(|_| ValidationError::SignatureRecoverFailed)?;
    // Recover pubkey directly from the recoverable signature
    let secp = Secp256k1::new();
    let pubkey = secp.recover_ecdsa(&msg, &rec_sig)
        .map_err(|_| ValidationError::SignatureRecoverFailed)?;

    // Compare serialized pubkey to header.signer
    let serialized = pubkey.serialize();
    if header.signer.is_empty() {
        return Err(ValidationError::MissingField);
    }
    if header.signer.len() != serialized.len() || header.signer.as_slice() != serialized {
        return Err(ValidationError::SignerMismatch);
    }

    Ok(())
}

/// Verify SR25519 attestation over provided payload bytes.
pub fn verify_sr25519_attestation(
    att: &Attestation,
    payload: &[u8],
) -> Result<(), ValidationError> {
    if att.scheme != AttestationScheme::Sr25519 {
        return Err(ValidationError::InvalidAttestation);
    }

    if att.signature.len() != 64 {
        return Err(ValidationError::InvalidAttestation);
    }

    // build types
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&att.signature[..64]);
    let signature = sr25519::Signature::from_raw(sig_arr);

    let pubkey = sr25519::Public::from_raw(att.attester_pubkey);

    // verify using sp_core native verification (works in both native and wasm)
    if !sr25519::Pair::verify(&signature, payload, &pubkey) {
        return Err(ValidationError::InvalidAttestation);
    }

    Ok(())
}

/// Basic EVM receipt validation including optional signature verification.
pub fn validate_evm_receipt(
    header: &ReceiptHeader,
    evm: &EvmReceipt,
    min_confirmations: u32,
    signature: Option<&[u8]>,
) -> Result<(), ValidationError> {
    if header.domain_id != DomainId::Evm {
        return Err(ValidationError::WrongDomain);
    }

    if evm.confirmations < min_confirmations {
        return Err(ValidationError::InsufficientConfirmations);
    }

    // Basic non-zero checks (tx hash, contract address)
    if evm.tx_hash == [0u8; 32] || evm.contract_address == [0u8; 20] {
        return Err(ValidationError::MissingField);
    }

    // Simple signer presence check: only required if an explicit signature is supplied
    if signature.is_some() && header.signer.is_empty() {
        return Err(ValidationError::MissingField);
    }

    if let Some(sig) = signature {
        verify_evm_signature(header, evm, sig)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use asga_receipts::{EvmReceipt, Phase, ReceiptHeader};
    use rand::rngs::OsRng;
    use secp256k1::{PublicKey, SecretKey};

    #[test]
    fn valid_evm_signature_roundtrip() {
        // build header and payload
        let header = ReceiptHeader {
            intent_id: [1u8; 32],
            domain_id: DomainId::Evm,
            phase: Phase::Lock,
            amount: 100u128,
            asset_id: [2u8; 32],
            timestamp: 1_700_000_000u64,
            signer: vec![0u8; 33], // to be filled
        };

        let evm = EvmReceipt {
            tx_hash: [4u8; 32],
            block_number: 1000u64,
            confirmations: 50u32,
            contract_address: [5u8; 20],
            calldata_hash: [6u8; 32],
        };

        // create keypair
        let secp = Secp256k1::new();
        let mut rng = OsRng::default();
        let sk = SecretKey::new(&mut rng);
        let pk = PublicKey::from_secret_key(&secp, &sk);
        let comp = pk.serialize();

        // patch header signer
        let mut header_signed = header.clone();
        header_signed.signer = comp.to_vec();

        // sign message
        let mut msg_bytes = header_signed.encode();
        msg_bytes.extend(evm.encode());
        let msg_hash = keccak256(&msg_bytes);
        let msg = Message::from_slice(&msg_hash).unwrap();

        let rec_sig = secp.sign_ecdsa_recoverable(&msg, &sk);
        let (recid, compact) = rec_sig.serialize_compact();
        let v = (recid.to_i32() as u8) + 27; // use 27/28
        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&compact);
        sig_bytes.push(v);

        // verify
        let res = validate_evm_receipt(&header_signed, &evm, 12u32, Some(&sig_bytes));
        assert_eq!(res, Ok(()));
    }

    #[test]
    fn sr25519_attestation_roundtrip() {
        let payload = b"hello world".to_vec();
        // create sr25519 pair
        let pair = sp_core::sr25519::Pair::from_string("//Alice", None).expect("key");
        let sig = pair.sign(&payload);
        let att = Attestation {
            attester_pubkey: pair.public().0,
            scheme: AttestationScheme::Sr25519,
            signature: sig.0.to_vec(),
        };

        let res = verify_sr25519_attestation(&att, &payload);
        assert_eq!(res, Ok(()));
    }
    #[test]
    fn signer_mismatch_fails() {
        let header = ReceiptHeader {
            intent_id: [1u8; 32],
            domain_id: DomainId::Evm,
            phase: Phase::Lock,
            amount: 100u128,
            asset_id: [2u8; 32],
            timestamp: 1_700_000_000u64,
            signer: vec![3u8; 33],
        };

        let evm = EvmReceipt {
            tx_hash: [4u8; 32],
            block_number: 1000u64,
            confirmations: 50u32,
            contract_address: [5u8; 20],
            calldata_hash: [6u8; 32],
        };

        // sign with a different key
        let secp = Secp256k1::new();
        let mut rng = OsRng::default();
        let sk = SecretKey::new(&mut rng);

        let mut msg_bytes = header.encode();
        msg_bytes.extend(evm.encode());
        let msg_hash = keccak256(&msg_bytes);
        let msg = Message::from_slice(&msg_hash).unwrap();

        let rec_sig = secp.sign_ecdsa_recoverable(&msg, &sk);
        let (recid, compact) = rec_sig.serialize_compact();
        let v = (recid.to_i32() as u8) + 27;
        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&compact);
        sig_bytes.push(v);

        let res = validate_evm_receipt(&header, &evm, 12u32, Some(&sig_bytes));
        assert_eq!(res, Err(ValidationError::SignerMismatch));
    }

    #[test]
    fn missing_signer_allowed_without_signature() {
        // If no signature is supplied, missing `header.signer` should not cause failure
        let header = ReceiptHeader {
            intent_id: [1u8; 32],
            domain_id: DomainId::Evm,
            phase: Phase::Lock,
            amount: 100u128,
            asset_id: [2u8; 32],
            timestamp: 1_700_000_000u64,
            signer: vec![0u8; 33],
        };

        let evm = EvmReceipt {
            tx_hash: [4u8; 32],
            block_number: 1000u64,
            confirmations: 50u32,
            contract_address: [5u8; 20],
            calldata_hash: [6u8; 32],
        };

        let res = validate_evm_receipt(&header, &evm, 12u32, None);
        assert_eq!(res, Ok(()));
    }
}
