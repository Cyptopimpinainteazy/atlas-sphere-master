//! SVM (Solana Virtual Machine) utilities for the Atlas SDK.
//!
//! Provides helpers for Solana-style instruction building,
//! account meta construction, and SVM-specific payload encoding.

use crate::error::{AtlasError, Result};
use crate::utils::{encode_compact_u16, from_hex, to_hex};
use sp_core::H256;

// ============================================================================
// Pubkey / Address Handling
// ============================================================================

/// A 32-byte Solana-style public key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    /// Create a new Pubkey from bytes.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a Pubkey from slice.
    pub fn from_slice(data: &[u8]) -> Result<Self> {
        if data.len() != 32 {
            return Err(AtlasError::InvalidAddress(format!(
                "Pubkey must be 32 bytes, got {}",
                data.len()
            )));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(data);
        Ok(Self(bytes))
    }

    /// Create a Pubkey from hex string.
    pub fn from_hex(s: &str) -> Result<Self> {
        let bytes = from_hex(s)?;
        Self::from_slice(&bytes)
    }

    /// Create a Pubkey from base58 string.
    pub fn from_base58(s: &str) -> Result<Self> {
        let bytes = bs58_decode(s)?;
        Self::from_slice(&bytes)
    }

    /// Convert to H256.
    pub fn to_h256(&self) -> H256 {
        H256(self.0)
    }

    /// Get as bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        to_hex(&self.0)
    }

    /// Convert to base58 string.
    pub fn to_base58(&self) -> String {
        bs58_encode(&self.0)
    }
}

impl From<H256> for Pubkey {
    fn from(h: H256) -> Self {
        Self(h.0)
    }
}

impl From<Pubkey> for H256 {
    fn from(p: Pubkey) -> Self {
        H256(p.0)
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

// ============================================================================
// Base58 Encoding/Decoding
// ============================================================================

const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Encode bytes to base58.
pub fn bs58_encode(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }

    // Count leading zeros
    let leading_zeros = data.iter().take_while(|&&b| b == 0).count();

    // Convert to base58
    let mut result = Vec::new();
    let mut num = data.to_vec();

    while !num.is_empty() && !num.iter().all(|&b| b == 0) {
        let mut remainder = 0u32;
        let mut new_num = Vec::new();

        for &byte in &num {
            let acc = (remainder << 8) + byte as u32;
            let digit = acc / 58;
            remainder = acc % 58;

            if !new_num.is_empty() || digit > 0 {
                new_num.push(digit as u8);
            }
        }

        result.push(BASE58_ALPHABET[remainder as usize]);
        num = new_num;
    }

    // Add leading '1's for leading zeros
    for _ in 0..leading_zeros {
        result.push(b'1');
    }

    result.reverse();
    String::from_utf8(result).unwrap_or_default()
}

/// Decode base58 to bytes.
pub fn bs58_decode(s: &str) -> Result<Vec<u8>> {
    if s.is_empty() {
        return Ok(Vec::new());
    }

    // Build alphabet index map
    let mut alphabet_map = [255u8; 128];
    for (i, &c) in BASE58_ALPHABET.iter().enumerate() {
        alphabet_map[c as usize] = i as u8;
    }

    // Count leading '1's
    let leading_ones = s.chars().take_while(|&c| c == '1').count();

    // Convert from base58
    let mut num = Vec::new();

    for c in s.chars() {
        if c as usize >= 128 || alphabet_map[c as usize] == 255 {
            return Err(AtlasError::Decoding(format!(
                "Invalid base58 character: {}",
                c
            )));
        }

        let digit = alphabet_map[c as usize] as u32;

        // Multiply num by 58 and add digit
        let mut carry = digit;
        for byte in num.iter_mut().rev() {
            let acc = (*byte as u32) * 58 + carry;
            *byte = (acc % 256) as u8;
            carry = acc / 256;
        }

        while carry > 0 {
            num.insert(0, (carry % 256) as u8);
            carry /= 256;
        }
    }

    // Add leading zeros
    let mut result = vec![0u8; leading_ones];
    result.extend(num);

    Ok(result)
}

// ============================================================================
// Account Meta
// ============================================================================

/// Metadata for an account in an SVM instruction.
#[derive(Clone, Debug)]
pub struct AccountMeta {
    /// Account public key.
    pub pubkey: Pubkey,
    /// Is the account a signer?
    pub is_signer: bool,
    /// Is the account writable?
    pub is_writable: bool,
}

impl AccountMeta {
    /// Create a new read-only account meta.
    pub fn new_readonly(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: false,
        }
    }

    /// Create a new writable account meta.
    pub fn new(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: true,
        }
    }
}

// ============================================================================
// Instruction Building
// ============================================================================

/// An SVM instruction.
#[derive(Clone, Debug)]
pub struct Instruction {
    /// Program ID to invoke.
    pub program_id: Pubkey,
    /// Accounts required by the instruction.
    pub accounts: Vec<AccountMeta>,
    /// Instruction data.
    pub data: Vec<u8>,
}

impl Instruction {
    /// Create a new instruction.
    pub fn new(program_id: Pubkey, accounts: Vec<AccountMeta>, data: Vec<u8>) -> Self {
        Self {
            program_id,
            accounts,
            data,
        }
    }

    /// Serialize the instruction to bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let mut output = Vec::new();

        // Program ID
        output.extend_from_slice(&self.program_id.0);

        // Account count (compact-u16)
        output.extend_from_slice(&encode_compact_u16(self.accounts.len() as u16));

        // Account metas
        for account in &self.accounts {
            output.extend_from_slice(&account.pubkey.0);
            let flags = (account.is_writable as u8) | ((account.is_signer as u8) << 1);
            output.push(flags);
        }

        // Data length (compact-u16)
        output.extend_from_slice(&encode_compact_u16(self.data.len() as u16));

        // Instruction data
        output.extend_from_slice(&self.data);

        output
    }
}

/// Build an SVM payload from instructions.
pub fn build_svm_payload(instructions: &[Instruction]) -> Vec<u8> {
    let mut payload = Vec::new();

    // Number of instructions (compact-u16)
    payload.extend_from_slice(&encode_compact_u16(instructions.len() as u16));

    // Each instruction
    for ix in instructions {
        payload.extend_from_slice(&ix.serialize());
    }

    payload
}

// ============================================================================
// Anchor Framework Helpers
// ============================================================================

/// Compute Anchor discriminator (first 8 bytes of SHA256 hash).
///
/// Note: Using Blake2b as placeholder - for production, use SHA256.
pub fn anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    use crate::utils::blake2b_256;

    let preimage = format!("{}:{}", namespace, name);
    let hash = blake2b_256(preimage.as_bytes());

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.0[..8]);
    discriminator
}

/// Build an Anchor instruction.
pub fn build_anchor_instruction(
    program_id: Pubkey,
    instruction_name: &str,
    accounts: Vec<AccountMeta>,
    args: &[u8],
) -> Instruction {
    let discriminator = anchor_discriminator("global", instruction_name);

    let mut data = discriminator.to_vec();
    data.extend_from_slice(args);

    Instruction::new(program_id, accounts, data)
}

// ============================================================================
// Common Program IDs
// ============================================================================

/// System program ID.
pub fn system_program_id() -> Pubkey {
    // 11111111111111111111111111111111
    Pubkey([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ])
}

/// Token program ID (SPL Token).
pub fn token_program_id() -> Pubkey {
    // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
    let bytes = bs58_decode("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        .unwrap_or_else(|_| vec![0u8; 32]);
    let mut arr = [0u8; 32];
    if bytes.len() == 32 {
        arr.copy_from_slice(&bytes);
    }
    Pubkey(arr)
}

/// Associated Token Account program ID.
pub fn associated_token_program_id() -> Pubkey {
    // ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
    let bytes = bs58_decode("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
        .unwrap_or_else(|_| vec![0u8; 32]);
    let mut arr = [0u8; 32];
    if bytes.len() == 32 {
        arr.copy_from_slice(&bytes);
    }
    Pubkey(arr)
}

// ============================================================================
// SPL Token Instructions
// ============================================================================

/// Build an SPL Token transfer instruction.
pub fn spl_token_transfer(
    source: Pubkey,
    destination: Pubkey,
    authority: Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = vec![3u8]; // Transfer instruction
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction::new(
        token_program_id(),
        vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data,
    )
}

/// Build an SPL Token approve instruction.
pub fn spl_token_approve(
    source: Pubkey,
    delegate: Pubkey,
    owner: Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = vec![4u8]; // Approve instruction
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction::new(
        token_program_id(),
        vec![
            AccountMeta::new(source, false),
            AccountMeta::new_readonly(delegate, false),
            AccountMeta::new_readonly(owner, true),
        ],
        data,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubkey_creation() {
        let bytes = [0xabu8; 32];
        let pubkey = Pubkey::new(bytes);
        assert_eq!(pubkey.as_bytes(), &bytes);
    }

    #[test]
    fn test_base58_roundtrip() {
        let original = vec![1, 2, 3, 4, 5];
        let encoded = bs58_encode(&original);
        let decoded = bs58_decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_base58_leading_zeros() {
        let original = vec![0, 0, 1, 2, 3];
        let encoded = bs58_encode(&original);
        let decoded = bs58_decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_account_meta() {
        let pubkey = Pubkey::new([0x11u8; 32]);

        let readonly = AccountMeta::new_readonly(pubkey, false);
        assert!(!readonly.is_writable);
        assert!(!readonly.is_signer);

        let writable_signer = AccountMeta::new(pubkey, true);
        assert!(writable_signer.is_writable);
        assert!(writable_signer.is_signer);
    }

    #[test]
    fn test_instruction_serialize() {
        let program_id = Pubkey::new([0xaa; 32]);
        let account = AccountMeta::new(Pubkey::new([0xbb; 32]), true);
        let data = vec![1, 2, 3, 4];

        let ix = Instruction::new(program_id, vec![account], data);
        let serialized = ix.serialize();

        // Should start with program ID (32 bytes)
        assert_eq!(&serialized[0..32], &[0xaa; 32]);
    }

    #[test]
    fn test_anchor_discriminator() {
        let disc = anchor_discriminator("global", "initialize");
        assert_eq!(disc.len(), 8);
    }

    #[test]
    fn test_spl_token_transfer() {
        let source = Pubkey::new([0x11; 32]);
        let dest = Pubkey::new([0x22; 32]);
        let auth = Pubkey::new([0x33; 32]);

        let ix = spl_token_transfer(source, dest, auth, 1000);

        assert_eq!(ix.accounts.len(), 3);
        assert_eq!(ix.data[0], 3); // Transfer instruction tag
    }

    #[test]
    fn test_build_svm_payload() {
        let ix = Instruction::new(
            Pubkey::new([0xaa; 32]),
            vec![AccountMeta::new(Pubkey::new([0xbb; 32]), false)],
            vec![1, 2, 3],
        );

        let payload = build_svm_payload(&[ix]);
        assert!(!payload.is_empty());
    }
}
