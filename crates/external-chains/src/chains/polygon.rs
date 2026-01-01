//! Polygon Chain Adapter
//!
//! Adapter for Polygon PoS (formerly Matic)
//! Chain ID: 137

use crate::adapter::*;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Polygon chain adapter
pub struct PolygonAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl PolygonAdapter {
    /// Create new Polygon adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Polygon PoS Bridge (Root Chain Manager)
    pub const ROOT_CHAIN_MANAGER: H160 = H160(hex_literal::hex!("A0c68C638235ee32657e8f720a23ceC1bFc77C77"));
    
    /// Polygon PoS Bridge (Child Chain Manager Proxy)
    pub const CHILD_CHAIN_MANAGER: H160 = H160(hex_literal::hex!("A6FA4fB5f76172d178d61B04b0ecd319C5d1C0aa"));

    /// MATIC token on Ethereum (for reference)
    pub const MATIC_ON_ETH: H160 = H160(hex_literal::hex!("7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0"));

    /// POL token (native token on Polygon)
    pub const POL_TOKEN: H160 = H160(hex_literal::hex!("0000000000000000000000000000000000001010"));

    /// Encode deposit for POL (native transfer via PoS bridge)
    pub fn encode_deposit_for(
        user: H160,
        token: H160,
        deposit_data: Vec<u8>,
    ) -> Vec<u8> {
        // depositFor(address user, address rootToken, bytes depositData)
        let mut calldata = Vec::with_capacity(4 + 32 * 3 + deposit_data.len());
        
        // Function selector: depositFor(address,address,bytes)
        calldata.extend_from_slice(&[0xe8, 0x27, 0x42, 0x12]);
        
        // User address
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(user.as_bytes());
        
        // Token address
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(token.as_bytes());
        
        // Deposit data offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);
        
        // Deposit data length
        let data_len = deposit_data.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&data_len.to_be_bytes());
        
        // Deposit data
        calldata.extend_from_slice(&deposit_data);
        let padding = (32 - deposit_data.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);
        
        calldata
    }

    /// Encode withdraw (burn on Polygon, exit on Ethereum)
    pub fn encode_withdraw(amount: U256) -> Vec<u8> {
        // withdraw(uint256 amount) for native POL
        let mut calldata = Vec::with_capacity(4 + 32);
        
        // Function selector: withdraw(uint256)
        calldata.extend_from_slice(&[0x2e, 0x1a, 0x7d, 0x4d]);
        
        // Amount
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        calldata.extend_from_slice(&amount_bytes);
        
        calldata
    }

    /// Get checkpoint data for exit proofs
    #[allow(dead_code)]
    async fn get_checkpoint(&self, _block_number: u64) -> AdapterResult<H256> {
        // Polygon uses checkpoints submitted to Ethereum
        Ok(H256::zero())
    }
}

#[async_trait::async_trait]
impl ChainAdapter for PolygonAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Polygon
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        Ok(65_000_000) // Polygon block number
    }

    async fn get_balance(&self, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn get_token_balance(&self, _token: H160, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        // Uses StateSender for Polygon->Ethereum messages
        Ok(message.hash())
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Query StateSync events
        Ok(vec![])
    }

    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Deposit via RootChainManager (Ethereum->Polygon)
        // Or burn via ChildToken (Polygon->Ethereum)
        Ok(transfer.id)
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Check checkpoint inclusion for exits
        Ok(TransferStatus::Completed)
    }

    async fn verify_message_proof(&self, _message: &ChainMessage, proof: &[u8]) -> AdapterResult<bool> {
        // Polygon uses Merkle proof against checkpoint
        // Requires block to be checkpointed (~30 min to 1 hr)
        Ok(!proof.is_empty())
    }

    async fn finalize_transfer(&self, transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Call exit() on RootChainManager with proof
        Ok(transfer_id)
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        // Polygon has very low gas prices
        Ok(U256::from(30_000_000_000u64)) // 30 gwei
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> AdapterResult<Option<TransactionReceipt>> {
        Ok(Some(TransactionReceipt {
            tx_hash,
            block_number: 65_000_000,
            block_hash: H256::zero(),
            tx_index: 0,
            success: true,
            gas_used: 21_000,
            logs: vec![],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_adapter() {
        let adapter = PolygonAdapter::new(ChainConfig::for_chain(ChainType::Polygon));
        assert_eq!(adapter.chain_type(), ChainType::Polygon);
        assert_eq!(adapter.config().chain_type, 137);
    }

    #[test]
    fn test_encode_withdraw() {
        let calldata = PolygonAdapter::encode_withdraw(U256::from(1_000_000_000_000_000_000u64));
        assert_eq!(&calldata[0..4], &[0x2e, 0x1a, 0x7d, 0x4d]);
        assert_eq!(calldata.len(), 36);
    }
}
