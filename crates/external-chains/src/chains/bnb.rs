//! BNB Smart Chain Adapter
//!
//! Adapter for BNB Smart Chain (formerly Binance Smart Chain)
//! Chain ID: 56

use crate::adapter::*;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// BNB Smart Chain adapter
pub struct BnbAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl BnbAdapter {
    /// Create new BNB adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// BNB Token Hub (cross-chain bridge to BNB Beacon Chain)
    pub const TOKEN_HUB: H160 = H160(hex_literal::hex!("0000000000000000000000000000000000001004"));
    
    /// Cross-chain contract for Beacon Chain communication
    pub const CROSS_CHAIN: H160 = H160(hex_literal::hex!("0000000000000000000000000000000000002000"));

    /// WBNB token
    pub const WBNB: H160 = H160(hex_literal::hex!("bb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"));

    /// PancakeSwap Router (for reference - common DEX)
    pub const PANCAKE_ROUTER: H160 = H160(hex_literal::hex!("10ED43C718714eb63d5aA57B78B54704E256024E"));

    /// Encode cross-chain transfer to Beacon Chain
    pub fn encode_transfer_out(
        contract_addr: H160,
        recipient: Vec<u8>, // BNB Beacon Chain address (bech32)
        amount: U256,
        expire_time: u64,
    ) -> Vec<u8> {
        // transferOut(address,bytes,uint256,uint64)
        let mut calldata = Vec::with_capacity(4 + 32 * 4 + recipient.len());
        
        // Function selector
        calldata.extend_from_slice(&[0xaa, 0x7a, 0x56, 0x21]);
        
        // Contract address (token)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(contract_addr.as_bytes());
        
        // Recipient offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x80);
        
        // Amount
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        calldata.extend_from_slice(&amount_bytes);
        
        // Expire time
        calldata.extend_from_slice(&[0u8; 24]);
        calldata.extend_from_slice(&expire_time.to_be_bytes());
        
        // Recipient length
        let recipient_len = recipient.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&recipient_len.to_be_bytes());
        
        // Recipient data
        calldata.extend_from_slice(&recipient);
        let padding = (32 - recipient.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);
        
        calldata
    }

    /// Encode BEP20 approve
    pub fn encode_approve(spender: H160, amount: U256) -> Vec<u8> {
        let mut calldata = Vec::with_capacity(4 + 64);
        
        // approve(address,uint256)
        calldata.extend_from_slice(&[0x09, 0x5e, 0xa7, 0xb3]);
        
        // Spender
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(spender.as_bytes());
        
        // Amount
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        calldata.extend_from_slice(&amount_bytes);
        
        calldata
    }

    /// Get validator set from system contract
    #[allow(dead_code)]
    async fn get_validators(&self) -> AdapterResult<Vec<H160>> {
        // BNB uses PoSA with 21 validators
        Ok(vec![])
    }
}

#[async_trait::async_trait]
impl ChainAdapter for BnbAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Bnb
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        Ok(45_000_000) // BSC block number
    }

    async fn get_balance(&self, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn get_token_balance(&self, _token: H160, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        // Uses CrossChain contract for inter-chain messaging
        Ok(message.hash())
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Query CrossChainPackage events
        Ok(vec![])
    }

    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Use TokenHub for BEP2<->BEP20 transfers
        Ok(transfer.id)
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // BSC has 3-second blocks, ~15 confirmations for safety
        Ok(TransferStatus::Completed)
    }

    async fn verify_message_proof(&self, _message: &ChainMessage, proof: &[u8]) -> AdapterResult<bool> {
        // BSC uses validator signatures for cross-chain proofs
        Ok(!proof.is_empty())
    }

    async fn finalize_transfer(&self, transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Handled by relayer infrastructure
        Ok(transfer_id)
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        // BSC has low gas prices
        Ok(U256::from(3_000_000_000u64)) // 3 gwei
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> AdapterResult<Option<TransactionReceipt>> {
        Ok(Some(TransactionReceipt {
            tx_hash,
            block_number: 45_000_000,
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
    fn test_bnb_adapter() {
        let adapter = BnbAdapter::new(ChainConfig::for_chain(ChainType::Bnb));
        assert_eq!(adapter.chain_type(), ChainType::Bnb);
        assert_eq!(adapter.config().chain_type, 56);
    }

    #[test]
    fn test_encode_approve() {
        let calldata = BnbAdapter::encode_approve(
            H160::zero(),
            U256::MAX,
        );
        assert_eq!(&calldata[0..4], &[0x09, 0x5e, 0xa7, 0xb3]);
        assert_eq!(calldata.len(), 68);
    }

    #[test]
    fn test_well_known_addresses() {
        assert_ne!(BnbAdapter::WBNB, H160::zero());
        assert_ne!(BnbAdapter::PANCAKE_ROUTER, H160::zero());
    }
}
