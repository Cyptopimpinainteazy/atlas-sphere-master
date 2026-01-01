// crates/x3-runtime/src/payload.rs
// Comit payload construction from X3 execution results
//
// Converts X3 operations to EVM/SVM payloads for submit_comit

use crate::{X3ExecutionResult, X3Operation, X3OpType, ComitParams};
use crate::types::{Token, Route, ChainId};
use anyhow::{anyhow, Result};
use parity_scale_codec::{Encode, Decode};

/// Payload encoder for EVM operations
pub struct EvmPayloadEncoder;

impl EvmPayloadEncoder {
    /// Encode a token transfer for EVM
    pub fn encode_transfer(
        token_addr: [u8; 20],
        to: [u8; 20],
        amount: u128,
    ) -> Vec<u8> {
        // ERC20 transfer(address,uint256) = 0xa9059cbb
        let mut payload = vec![0xa9, 0x05, 0x9c, 0xbb];
        
        // Pad 'to' address to 32 bytes
        payload.extend_from_slice(&[0u8; 12]);
        payload.extend_from_slice(&to);
        
        // Amount as 32-byte big-endian
        let amount_bytes = amount.to_be_bytes();
        payload.extend_from_slice(&[0u8; 16]); // Pad to 32 bytes
        payload.extend_from_slice(&amount_bytes);
        
        payload
    }

    /// Encode a token approval for EVM
    pub fn encode_approve(
        token_addr: [u8; 20],
        spender: [u8; 20],
        amount: u128,
    ) -> Vec<u8> {
        // ERC20 approve(address,uint256) = 0x095ea7b3
        let mut payload = vec![0x09, 0x5e, 0xa7, 0xb3];
        
        payload.extend_from_slice(&[0u8; 12]);
        payload.extend_from_slice(&spender);
        
        let amount_bytes = amount.to_be_bytes();
        payload.extend_from_slice(&[0u8; 16]);
        payload.extend_from_slice(&amount_bytes);
        
        payload
    }

    /// Encode a DEX swap for EVM (Uniswap V2 style)
    pub fn encode_swap(
        router: [u8; 20],
        amount_in: u128,
        amount_out_min: u128,
        path: &[[u8; 20]],
        to: [u8; 20],
        deadline: u64,
    ) -> Vec<u8> {
        // swapExactTokensForTokens(uint,uint,address[],address,uint) = 0x38ed1739
        let mut payload = vec![0x38, 0xed, 0x17, 0x39];
        
        // amount_in
        payload.extend_from_slice(&[0u8; 16]);
        payload.extend_from_slice(&amount_in.to_be_bytes());
        
        // amount_out_min
        payload.extend_from_slice(&[0u8; 16]);
        payload.extend_from_slice(&amount_out_min.to_be_bytes());
        
        // Offset to path array (5 * 32 = 160 = 0xa0)
        payload.extend_from_slice(&[0u8; 31]);
        payload.push(0xa0);
        
        // to address
        payload.extend_from_slice(&[0u8; 12]);
        payload.extend_from_slice(&to);
        
        // deadline
        payload.extend_from_slice(&[0u8; 24]);
        payload.extend_from_slice(&deadline.to_be_bytes());
        
        // path array length
        payload.extend_from_slice(&[0u8; 31]);
        payload.push(path.len() as u8);
        
        // path elements
        for addr in path {
            payload.extend_from_slice(&[0u8; 12]);
            payload.extend_from_slice(addr);
        }
        
        payload
    }
}

/// Payload encoder for SVM operations
pub struct SvmPayloadEncoder;

impl SvmPayloadEncoder {
    /// Encode a SPL token transfer
    pub fn encode_transfer(
        mint: [u8; 32],
        source: [u8; 32],
        destination: [u8; 32],
        authority: [u8; 32],
        amount: u64,
    ) -> Vec<u8> {
        // SPL Token Transfer instruction
        // Instruction ID: 3 (Transfer)
        let mut payload = vec![3u8];
        
        // Amount (little-endian u64)
        payload.extend_from_slice(&amount.to_le_bytes());
        
        // Account indices follow in Solana transaction format
        // For X3, we encode the full addresses for runtime resolution
        payload.extend_from_slice(&source);
        payload.extend_from_slice(&destination);
        payload.extend_from_slice(&authority);
        payload.extend_from_slice(&mint);
        
        payload
    }

    /// Encode a SPL token approval (delegate)
    pub fn encode_approve(
        source: [u8; 32],
        delegate: [u8; 32],
        owner: [u8; 32],
        amount: u64,
    ) -> Vec<u8> {
        // SPL Token Approve instruction
        // Instruction ID: 4 (Approve)
        let mut payload = vec![4u8];
        
        payload.extend_from_slice(&amount.to_le_bytes());
        payload.extend_from_slice(&source);
        payload.extend_from_slice(&delegate);
        payload.extend_from_slice(&owner);
        
        payload
    }

    /// Encode a Raydium/Orca swap
    pub fn encode_swap(
        pool: [u8; 32],
        user_source: [u8; 32],
        user_destination: [u8; 32],
        amount_in: u64,
        min_amount_out: u64,
    ) -> Vec<u8> {
        // Generic AMM swap instruction
        let mut payload = vec![1u8]; // Swap instruction
        
        payload.extend_from_slice(&amount_in.to_le_bytes());
        payload.extend_from_slice(&min_amount_out.to_le_bytes());
        payload.extend_from_slice(&pool);
        payload.extend_from_slice(&user_source);
        payload.extend_from_slice(&user_destination);
        
        payload
    }
}

/// High-level Comit payload builder
pub struct ComitBuilder {
    evm_ops: Vec<EvmOp>,
    svm_ops: Vec<SvmOp>,
}

#[derive(Debug, Clone)]
pub struct EvmOp {
    pub to: [u8; 20],
    pub value: u128,
    pub data: Vec<u8>,
    pub gas_limit: u64,
}

#[derive(Debug, Clone)]
pub struct SvmOp {
    pub program: [u8; 32],
    pub data: Vec<u8>,
    pub accounts: Vec<[u8; 32]>,
}

impl ComitBuilder {
    pub fn new() -> Self {
        ComitBuilder {
            evm_ops: Vec::new(),
            svm_ops: Vec::new(),
        }
    }

    /// Add EVM token transfer
    pub fn evm_transfer(
        &mut self,
        token: [u8; 20],
        to: [u8; 20],
        amount: u128,
    ) -> &mut Self {
        let data = EvmPayloadEncoder::encode_transfer(token, to, amount);
        self.evm_ops.push(EvmOp {
            to: token,
            value: 0,
            data,
            gas_limit: 65000,
        });
        self
    }

    /// Add EVM approval
    pub fn evm_approve(
        &mut self,
        token: [u8; 20],
        spender: [u8; 20],
        amount: u128,
    ) -> &mut Self {
        let data = EvmPayloadEncoder::encode_approve(token, spender, amount);
        self.evm_ops.push(EvmOp {
            to: token,
            value: 0,
            data,
            gas_limit: 50000,
        });
        self
    }

    /// Add EVM DEX swap
    pub fn evm_swap(
        &mut self,
        router: [u8; 20],
        amount_in: u128,
        min_out: u128,
        path: &[[u8; 20]],
        to: [u8; 20],
    ) -> &mut Self {
        let deadline = u64::MAX; // Max deadline for simplicity
        let data = EvmPayloadEncoder::encode_swap(router, amount_in, min_out, path, to, deadline);
        self.evm_ops.push(EvmOp {
            to: router,
            value: 0,
            data,
            gas_limit: 300000,
        });
        self
    }

    /// Add SVM token transfer
    pub fn svm_transfer(
        &mut self,
        mint: [u8; 32],
        source: [u8; 32],
        destination: [u8; 32],
        authority: [u8; 32],
        amount: u64,
    ) -> &mut Self {
        let data = SvmPayloadEncoder::encode_transfer(mint, source, destination, authority, amount);
        // SPL Token program ID
        let spl_token = [
            0x06, 0xdd, 0xf6, 0xe1, 0xd7, 0x65, 0xa1, 0x93,
            0xd9, 0xcb, 0xe1, 0x46, 0xce, 0xeb, 0x79, 0xac,
            0x1c, 0xb4, 0x85, 0xed, 0x5f, 0x5b, 0x37, 0x91,
            0x3a, 0x8c, 0xf5, 0x85, 0x7e, 0xff, 0x00, 0xa9,
        ];
        self.svm_ops.push(SvmOp {
            program: spl_token,
            data,
            accounts: vec![source, destination, authority, mint],
        });
        self
    }

    /// Build final Comit parameters
    pub fn build(self, nonce: u64) -> Result<ComitParams> {
        // Encode EVM operations
        let evm_payload = self.encode_evm_payload()?;
        
        // Encode SVM operations
        let svm_payload = self.encode_svm_payload()?;
        
        // Validate sizes
        if evm_payload.len() > 16384 {
            return Err(anyhow!("EVM payload too large: {}", evm_payload.len()));
        }
        if svm_payload.len() > 16384 {
            return Err(anyhow!("SVM payload too large: {}", svm_payload.len()));
        }
        
        // Calculate gas limit
        let evm_gas: u64 = self.evm_ops.iter().map(|op| op.gas_limit).sum();
        let svm_gas: u64 = self.svm_ops.len() as u64 * 200000; // ~200k CU per op
        
        Ok(ComitParams {
            evm_payload,
            svm_payload,
            nonce,
            gas_limit: evm_gas + svm_gas,
        })
    }

    fn encode_evm_payload(&self) -> Result<Vec<u8>> {
        // Simple encoding: [num_ops][op1_len][op1_data]...
        let mut payload = Vec::new();
        
        payload.push(self.evm_ops.len() as u8);
        
        for op in &self.evm_ops {
            // Operation format: [to:20][value:16][gas:8][data_len:4][data:...]
            payload.extend_from_slice(&op.to);
            payload.extend_from_slice(&op.value.to_le_bytes());
            payload.extend_from_slice(&op.gas_limit.to_le_bytes());
            payload.extend_from_slice(&(op.data.len() as u32).to_le_bytes());
            payload.extend_from_slice(&op.data);
        }
        
        Ok(payload)
    }

    fn encode_svm_payload(&self) -> Result<Vec<u8>> {
        // Simple encoding for SVM
        let mut payload = Vec::new();
        
        payload.push(self.svm_ops.len() as u8);
        
        for op in &self.svm_ops {
            // Operation format: [program:32][num_accounts:1][accounts...][data_len:4][data:...]
            payload.extend_from_slice(&op.program);
            payload.push(op.accounts.len() as u8);
            for acc in &op.accounts {
                payload.extend_from_slice(acc);
            }
            payload.extend_from_slice(&(op.data.len() as u32).to_le_bytes());
            payload.extend_from_slice(&op.data);
        }
        
        Ok(payload)
    }
}

impl Default for ComitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_transfer_encoding() {
        let token = [0x11u8; 20];
        let to = [0x22u8; 20];
        let amount = 1000u128;
        
        let payload = EvmPayloadEncoder::encode_transfer(token, to, amount);
        
        // Should start with transfer selector
        assert_eq!(&payload[0..4], &[0xa9, 0x05, 0x9c, 0xbb]);
        
        // Should be 4 + 32 + 32 = 68 bytes
        assert_eq!(payload.len(), 68);
    }

    #[test]
    fn test_evm_approve_encoding() {
        let token = [0x11u8; 20];
        let spender = [0x22u8; 20];
        let amount = u128::MAX;
        
        let payload = EvmPayloadEncoder::encode_approve(token, spender, amount);
        
        // Should start with approve selector
        assert_eq!(&payload[0..4], &[0x09, 0x5e, 0xa7, 0xb3]);
        
        assert_eq!(payload.len(), 68);
    }

    #[test]
    fn test_svm_transfer_encoding() {
        let mint = [1u8; 32];
        let source = [2u8; 32];
        let dest = [3u8; 32];
        let authority = [4u8; 32];
        let amount = 1000u64;
        
        let payload = SvmPayloadEncoder::encode_transfer(mint, source, dest, authority, amount);
        
        // First byte is instruction ID
        assert_eq!(payload[0], 3);
        
        // Should be 1 + 8 + 4*32 = 137 bytes
        assert_eq!(payload.len(), 137);
    }

    #[test]
    fn test_comit_builder_evm_only() {
        let mut builder = ComitBuilder::new();
        
        builder.evm_transfer(
            [0x11u8; 20],
            [0x22u8; 20],
            1000,
        );
        
        let params = builder.build(1).unwrap();
        
        assert!(!params.evm_payload.is_empty());
        assert_eq!(params.svm_payload, vec![0]); // Just count byte
        assert_eq!(params.nonce, 1);
    }

    #[test]
    fn test_comit_builder_dual_vm() {
        let mut builder = ComitBuilder::new();
        
        builder
            .evm_transfer([0x11u8; 20], [0x22u8; 20], 1000)
            .svm_transfer([1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], 500);
        
        let params = builder.build(42).unwrap();
        
        assert!(!params.evm_payload.is_empty());
        assert!(!params.svm_payload.is_empty());
        assert_eq!(params.nonce, 42);
        assert!(params.gas_limit > 0);
    }

    #[test]
    fn test_payload_size_validation() {
        let mut builder = ComitBuilder::new();
        
        // Add many operations (should still be under limit)
        for i in 0..10 {
            builder.evm_transfer([i as u8; 20], [0x22u8; 20], 1000);
        }
        
        let params = builder.build(1).unwrap();
        assert!(params.evm_payload.len() < 16384);
    }
}
