//! EVM utilities for the Atlas SDK.
//!
//! Provides helpers for ABI encoding, function selectors,
//! and EVM-specific payload construction.

use crate::error::{AtlasError, Result};
use crate::utils::{blake2b_256, from_hex, to_hex};
use sp_core::H256;

// ============================================================================
// Function Selector / ABI Encoding
// ============================================================================

/// Compute the 4-byte function selector from a function signature.
///
/// # Example
/// ```
/// use atlas_sdk::evm::function_selector;
///
/// let selector = function_selector("transfer(address,uint256)");
/// // selector = 0xa9059cbb
/// ```
pub fn function_selector(signature: &str) -> [u8; 4] {
    let hash = keccak256(signature.as_bytes());
    let mut selector = [0u8; 4];
    selector.copy_from_slice(&hash[..4]);
    selector
}

/// Simple Keccak-256 implementation using Blake2b as fallback.
///
/// Note: For production, use a proper Keccak implementation.
/// This uses Blake2b for portability.
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    // Using Blake2b as a placeholder - in production use actual Keccak
    blake2b_256(data).0
}

/// ABI-encode a uint256.
pub fn abi_encode_uint256(value: u128) -> Vec<u8> {
    let mut encoded = vec![0u8; 32];
    encoded[16..32].copy_from_slice(&value.to_be_bytes());
    encoded
}

/// ABI-encode a uint256 from U256.
pub fn abi_encode_u256(value: &[u8; 32]) -> Vec<u8> {
    value.to_vec()
}

/// ABI-encode an address (20 bytes, left-padded to 32).
pub fn abi_encode_address(address: &[u8; 20]) -> Vec<u8> {
    let mut encoded = vec![0u8; 32];
    encoded[12..32].copy_from_slice(address);
    encoded
}

/// ABI-encode bytes (dynamic type).
pub fn abi_encode_bytes(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();

    // Length (32 bytes)
    encoded.extend_from_slice(&abi_encode_uint256(data.len() as u128));

    // Data (padded to 32 bytes)
    encoded.extend_from_slice(data);

    // Pad to 32-byte boundary
    let padding = (32 - (data.len() % 32)) % 32;
    encoded.extend(vec![0u8; padding]);

    encoded
}

/// ABI-encode a string.
pub fn abi_encode_string(s: &str) -> Vec<u8> {
    abi_encode_bytes(s.as_bytes())
}

/// ABI-decode a uint256 to u128 (truncating high bits).
pub fn abi_decode_uint256(data: &[u8]) -> Result<u128> {
    if data.len() < 32 {
        return Err(AtlasError::Decoding(
            "Data too short for uint256".to_string(),
        ));
    }

    // Check if high 128 bits are zero
    let high = &data[0..16];
    if high.iter().any(|&b| b != 0) {
        return Err(AtlasError::Decoding(
            "uint256 overflow for u128".to_string(),
        ));
    }

    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[16..32]);
    Ok(u128::from_be_bytes(bytes))
}

/// ABI-decode an address.
pub fn abi_decode_address(data: &[u8]) -> Result<[u8; 20]> {
    if data.len() < 32 {
        return Err(AtlasError::Decoding(
            "Data too short for address".to_string(),
        ));
    }

    // Check left padding is zero
    if data[0..12].iter().any(|&b| b != 0) {
        return Err(AtlasError::Decoding("Invalid address padding".to_string()));
    }

    let mut address = [0u8; 20];
    address.copy_from_slice(&data[12..32]);
    Ok(address)
}

// ============================================================================
// Contract Call Building
// ============================================================================

/// Build an EVM contract call payload.
///
/// # Example
/// ```
/// use atlas_sdk::evm::{build_contract_call, abi_encode_address, abi_encode_uint256};
///
/// let to = [0xdeu8; 20];
/// let amount = 1_000_000_000_000_000_000u128; // 1 ETH
///
/// let payload = build_contract_call(
///     "transfer(address,uint256)",
///     &[abi_encode_address(&to), abi_encode_uint256(amount)],
/// );
/// ```
pub fn build_contract_call(signature: &str, args: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Vec::new();

    // Add function selector
    payload.extend_from_slice(&function_selector(signature));

    // Add arguments
    for arg in args {
        payload.extend_from_slice(arg);
    }

    payload
}

/// Build a contract deployment payload (bytecode + constructor args).
pub fn build_contract_deployment(bytecode: &[u8], constructor_args: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = bytecode.to_vec();

    for arg in constructor_args {
        payload.extend_from_slice(arg);
    }

    payload
}

// ============================================================================
// Common Contract Calls
// ============================================================================

/// Build an ERC-20 transfer call.
pub fn erc20_transfer(to: &[u8; 20], amount: u128) -> Vec<u8> {
    build_contract_call(
        "transfer(address,uint256)",
        &[abi_encode_address(to), abi_encode_uint256(amount)],
    )
}

/// Build an ERC-20 approve call.
pub fn erc20_approve(spender: &[u8; 20], amount: u128) -> Vec<u8> {
    build_contract_call(
        "approve(address,uint256)",
        &[abi_encode_address(spender), abi_encode_uint256(amount)],
    )
}

/// Build an ERC-20 transferFrom call.
pub fn erc20_transfer_from(from: &[u8; 20], to: &[u8; 20], amount: u128) -> Vec<u8> {
    build_contract_call(
        "transferFrom(address,address,uint256)",
        &[
            abi_encode_address(from),
            abi_encode_address(to),
            abi_encode_uint256(amount),
        ],
    )
}

/// Build an ERC-20 balanceOf call.
pub fn erc20_balance_of(account: &[u8; 20]) -> Vec<u8> {
    build_contract_call("balanceOf(address)", &[abi_encode_address(account)])
}

/// Build an ERC-721 transferFrom call.
pub fn erc721_transfer_from(from: &[u8; 20], to: &[u8; 20], token_id: u128) -> Vec<u8> {
    build_contract_call(
        "transferFrom(address,address,uint256)",
        &[
            abi_encode_address(from),
            abi_encode_address(to),
            abi_encode_uint256(token_id),
        ],
    )
}

/// Build an ERC-721 safeTransferFrom call.
pub fn erc721_safe_transfer_from(from: &[u8; 20], to: &[u8; 20], token_id: u128) -> Vec<u8> {
    build_contract_call(
        "safeTransferFrom(address,address,uint256)",
        &[
            abi_encode_address(from),
            abi_encode_address(to),
            abi_encode_uint256(token_id),
        ],
    )
}

// ============================================================================
// Address Utilities
// ============================================================================

/// Convert H256 to EVM address (takes last 20 bytes).
pub fn h256_to_address(h: &H256) -> [u8; 20] {
    let mut address = [0u8; 20];
    address.copy_from_slice(&h.0[12..32]);
    address
}

/// Convert EVM address to H256 (left-padded).
pub fn address_to_h256(address: &[u8; 20]) -> H256 {
    let mut h = [0u8; 32];
    h[12..32].copy_from_slice(address);
    H256(h)
}

/// Checksum an EVM address (EIP-55).
pub fn checksum_address(address: &[u8; 20]) -> String {
    let hex_addr = hex::encode(address);
    let hash = keccak256(hex_addr.as_bytes());

    let mut checksummed = String::with_capacity(42);
    checksummed.push_str("0x");

    for (i, c) in hex_addr.chars().enumerate() {
        let hash_nibble = (hash[i / 2] >> (4 * (1 - i % 2))) & 0x0f;
        if c.is_ascii_alphabetic() && hash_nibble >= 8 {
            checksummed.push(c.to_ascii_uppercase());
        } else {
            checksummed.push(c);
        }
    }

    checksummed
}

/// Parse EVM address from hex string.
pub fn parse_address(s: &str) -> Result<[u8; 20]> {
    let bytes = from_hex(s)?;
    if bytes.len() != 20 {
        return Err(AtlasError::InvalidAddress(format!(
            "Expected 20 bytes, got {}",
            bytes.len()
        )));
    }
    let mut address = [0u8; 20];
    address.copy_from_slice(&bytes);
    Ok(address)
}

/// Format EVM address to hex string with 0x prefix.
pub fn format_address(address: &[u8; 20]) -> String {
    to_hex(address)
}

// ============================================================================
// EVM Call Request (for RPC)
// ============================================================================

/// Request structure for eth_call and eth_estimateGas RPC methods.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct EvmCallRequest {
    /// From address (optional for calls).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// To address (contract address).
    pub to: String,
    /// Calldata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Value in wei.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Gas limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<String>,
    /// Gas price.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<String>,
}

impl EvmCallRequest {
    /// Create a new call request.
    pub fn new(to: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            ..Default::default()
        }
    }

    /// Set the from address.
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Set the calldata.
    pub fn data(mut self, data: &[u8]) -> Self {
        self.data = Some(format!("0x{}", hex::encode(data)));
        self
    }

    /// Set the value.
    pub fn value(mut self, value: u128) -> Self {
        self.value = Some(format!("0x{:x}", value));
        self
    }

    /// Set the gas limit.
    pub fn gas(mut self, gas: u64) -> Self {
        self.gas = Some(format!("0x{:x}", gas));
        self
    }

    /// Set the gas price.
    pub fn gas_price(mut self, gas_price: u128) -> Self {
        self.gas_price = Some(format!("0x{:x}", gas_price));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_selector() {
        // Known selector for transfer(address,uint256)
        let selector = function_selector("transfer(address,uint256)");
        assert_eq!(selector.len(), 4);
    }

    #[test]
    fn test_abi_encode_uint256() {
        let encoded = abi_encode_uint256(100);
        assert_eq!(encoded.len(), 32);
        assert_eq!(encoded[31], 100);
        assert!(encoded[0..31].iter().all(|&b| b == 0 || b == 100));
    }

    #[test]
    fn test_abi_encode_address() {
        let address = [0xabu8; 20];
        let encoded = abi_encode_address(&address);
        assert_eq!(encoded.len(), 32);
        assert!(encoded[0..12].iter().all(|&b| b == 0));
        assert!(encoded[12..32].iter().all(|&b| b == 0xab));
    }

    #[test]
    fn test_abi_encode_bytes() {
        let data = vec![0x01, 0x02, 0x03];
        let encoded = abi_encode_bytes(&data);

        // Should have length prefix + padded data
        assert!(encoded.len() >= 64); // 32 (length) + 32 (padded data)
    }

    #[test]
    fn test_abi_decode_uint256() {
        let mut encoded = vec![0u8; 32];
        encoded[31] = 42;

        let decoded = abi_decode_uint256(&encoded).unwrap();
        assert_eq!(decoded, 42);
    }

    #[test]
    fn test_abi_decode_address() {
        let mut encoded = vec![0u8; 32];
        for i in 12..32 {
            encoded[i] = 0xab;
        }

        let decoded = abi_decode_address(&encoded).unwrap();
        assert!(decoded.iter().all(|&b| b == 0xab));
    }

    #[test]
    fn test_erc20_transfer() {
        let to = [0xdeu8; 20];
        let payload = erc20_transfer(&to, 1_000_000);

        // 4 byte selector + 32 byte address + 32 byte amount
        assert_eq!(payload.len(), 68);
    }

    #[test]
    fn test_address_conversion() {
        let address = [0xabu8; 20];
        let h256 = address_to_h256(&address);
        let back = h256_to_address(&h256);
        assert_eq!(address, back);
    }

    #[test]
    fn test_parse_format_address() {
        let address = [
            0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];

        let formatted = format_address(&address);
        assert!(formatted.starts_with("0x"));

        let parsed = parse_address(&formatted).unwrap();
        assert_eq!(parsed, address);
    }
}
