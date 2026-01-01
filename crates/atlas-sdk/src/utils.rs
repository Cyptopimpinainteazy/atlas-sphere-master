//! Utility functions for the Atlas SDK.

use crate::error::{AtlasError, Result};
use blake2::{Blake2b, Digest};
use sp_core::H256;

// ============================================================================
// Hashing Utilities
// ============================================================================

/// Compute Blake2b-256 hash.
pub fn blake2b_256(data: &[u8]) -> H256 {
    let mut hasher = Blake2b::<blake2::digest::consts::U32>::new();
    hasher.update(data);
    let result = hasher.finalize();
    H256::from_slice(&result)
}

/// Compute the prepare root for a Comit transaction.
///
/// The prepare root is a commitment to the input payloads,
/// used to verify transaction integrity.
pub fn compute_prepare_root(evm_payload: Option<&[u8]>, svm_payload: Option<&[u8]>) -> H256 {
    let mut data = Vec::new();

    // Include EVM payload hash
    if let Some(payload) = evm_payload {
        data.extend_from_slice(b"EVM:");
        data.extend_from_slice(&blake2b_256(payload).0);
    }

    // Include SVM payload hash
    if let Some(payload) = svm_payload {
        data.extend_from_slice(b"SVM:");
        data.extend_from_slice(&blake2b_256(payload).0);
    }

    blake2b_256(&data)
}

// ============================================================================
// Hex Encoding/Decoding
// ============================================================================

/// Encode bytes to hex string with 0x prefix.
pub fn to_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Decode hex string (with or without 0x prefix) to bytes.
pub fn from_hex(s: &str) -> Result<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).map_err(|e| AtlasError::Decoding(e.to_string()))
}

/// Convert H256 to hex string.
pub fn h256_to_hex(h: &H256) -> String {
    to_hex(h.as_bytes())
}

/// Parse hex string to H256.
pub fn hex_to_h256(s: &str) -> Result<H256> {
    let bytes = from_hex(s)?;
    if bytes.len() != 32 {
        return Err(AtlasError::Decoding(format!(
            "Expected 32 bytes for H256, got {}",
            bytes.len()
        )));
    }
    Ok(H256::from_slice(&bytes))
}

// ============================================================================
// Address Utilities
// ============================================================================

/// Validate an EVM address (20 bytes).
pub fn validate_evm_address(address: &[u8]) -> Result<[u8; 20]> {
    if address.len() != 20 {
        return Err(AtlasError::InvalidAddress(format!(
            "EVM address must be 20 bytes, got {}",
            address.len()
        )));
    }
    let mut arr = [0u8; 20];
    arr.copy_from_slice(address);
    Ok(arr)
}

/// Parse EVM address from hex string.
pub fn parse_evm_address(s: &str) -> Result<[u8; 20]> {
    let bytes = from_hex(s)?;
    validate_evm_address(&bytes)
}

/// Validate a Solana address (32 bytes).
pub fn validate_solana_address(address: &[u8]) -> Result<[u8; 32]> {
    if address.len() != 32 {
        return Err(AtlasError::InvalidAddress(format!(
            "Solana address must be 32 bytes, got {}",
            address.len()
        )));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(address);
    Ok(arr)
}

// ============================================================================
// Amount Formatting
// ============================================================================

/// Format balance with decimals.
pub fn format_balance(amount: u128, decimals: u8) -> String {
    let divisor = 10u128.pow(decimals as u32);
    let whole = amount / divisor;
    let frac = amount % divisor;

    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}

/// Parse balance string to raw amount.
pub fn parse_balance(s: &str, decimals: u8) -> Result<u128> {
    let parts: Vec<&str> = s.split('.').collect();

    let (whole_str, frac_str) = match parts.len() {
        1 => (parts[0], "0"),
        2 => (parts[0], parts[1]),
        _ => return Err(AtlasError::Decoding("Invalid balance format".to_string())),
    };

    let whole: u128 = whole_str
        .parse()
        .map_err(|_| AtlasError::Decoding("Invalid whole number".to_string()))?;

    let mut frac_padded = frac_str.to_string();
    while frac_padded.len() < decimals as usize {
        frac_padded.push('0');
    }
    frac_padded.truncate(decimals as usize);

    let frac: u128 = frac_padded
        .parse()
        .map_err(|_| AtlasError::Decoding("Invalid fractional number".to_string()))?;

    let multiplier = 10u128.pow(decimals as u32);
    Ok(whole * multiplier + frac)
}

// ============================================================================
// Compact Encoding (for SVM/Solana)
// ============================================================================

/// Encode a u16 as compact-u16 (Solana's variable-length format).
pub fn encode_compact_u16(value: u16) -> Vec<u8> {
    if value < 0x80 {
        vec![value as u8]
    } else if value < 0x4000 {
        vec![(value & 0x7f) as u8 | 0x80, ((value >> 7) & 0x7f) as u8]
    } else {
        vec![
            (value & 0x7f) as u8 | 0x80,
            ((value >> 7) & 0x7f) as u8 | 0x80,
            ((value >> 14) & 0x03) as u8,
        ]
    }
}

/// Decode compact-u16 to u16.
pub fn decode_compact_u16(data: &[u8]) -> Result<(u16, usize)> {
    if data.is_empty() {
        return Err(AtlasError::Decoding("Empty compact-u16".to_string()));
    }

    let mut value: u16 = 0;
    let mut shift = 0;
    let mut bytes_read = 0;

    for (i, &byte) in data.iter().enumerate().take(3) {
        bytes_read = i + 1;
        value |= ((byte & 0x7f) as u16) << shift;

        if byte & 0x80 == 0 {
            return Ok((value, bytes_read));
        }

        shift += 7;
    }

    Ok((value, bytes_read))
}

// ============================================================================
// Time Utilities
// ============================================================================

/// Get current Unix timestamp in seconds.
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Get current Unix timestamp in milliseconds.
pub fn current_timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake2b_256() {
        let hash = blake2b_256(b"hello");
        assert_eq!(hash.0.len(), 32);
    }

    #[test]
    fn test_hex_encoding() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let hex = to_hex(&bytes);
        assert_eq!(hex, "0xdeadbeef");

        let decoded = from_hex(&hex).unwrap();
        assert_eq!(decoded, bytes);

        // Without 0x prefix
        let decoded2 = from_hex("deadbeef").unwrap();
        assert_eq!(decoded2, bytes);
    }

    #[test]
    fn test_balance_formatting() {
        // 18 decimals (like ETH)
        assert_eq!(format_balance(1_000_000_000_000_000_000, 18), "1");
        assert_eq!(format_balance(1_500_000_000_000_000_000, 18), "1.5");
        assert_eq!(format_balance(1_234_567_890_000_000_000, 18), "1.23456789");

        // 6 decimals (like USDC)
        assert_eq!(format_balance(1_000_000, 6), "1");
        assert_eq!(format_balance(1_500_000, 6), "1.5");
    }

    #[test]
    fn test_balance_parsing() {
        assert_eq!(parse_balance("1", 18).unwrap(), 1_000_000_000_000_000_000);
        assert_eq!(parse_balance("1.5", 18).unwrap(), 1_500_000_000_000_000_000);
        assert_eq!(parse_balance("0.1", 18).unwrap(), 100_000_000_000_000_000);
    }

    #[test]
    fn test_compact_u16() {
        // Single byte
        let encoded = encode_compact_u16(100);
        assert_eq!(encoded, vec![100]);
        let (decoded, len) = decode_compact_u16(&encoded).unwrap();
        assert_eq!(decoded, 100);
        assert_eq!(len, 1);

        // Two bytes
        let encoded = encode_compact_u16(200);
        assert_eq!(encoded.len(), 2);
        let (decoded, len) = decode_compact_u16(&encoded).unwrap();
        assert_eq!(decoded, 200);
        assert_eq!(len, 2);
    }

    #[test]
    fn test_evm_address_validation() {
        let valid = [0u8; 20];
        assert!(validate_evm_address(&valid).is_ok());

        let invalid = [0u8; 19];
        assert!(validate_evm_address(&invalid).is_err());
    }

    #[test]
    fn test_prepare_root() {
        let root1 = compute_prepare_root(Some(&[0x01, 0x02]), None);
        let root2 = compute_prepare_root(Some(&[0x01, 0x02]), Some(&[0x03, 0x04]));
        let root3 = compute_prepare_root(None, Some(&[0x03, 0x04]));

        // All should be different
        assert_ne!(root1, root2);
        assert_ne!(root2, root3);
        assert_ne!(root1, root3);
    }
}
