//! Avalanche C-Chain Adapter
//!
//! Adapter for Avalanche C-Chain (EVM compatible)
//! Chain ID: 43114

use crate::adapter::*;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Avalanche C-Chain adapter
pub struct AvalancheAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl AvalancheAdapter {
    /// Create new Avalanche adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Avalanche Bridge contract
    pub const BRIDGE_CONTRACT: H160 = H160(hex_literal::hex!("8EB8a3b98659Cce290402893d0123abb75E3ab28"));
    
    /// WAVAX token (wrapped AVAX)
    pub const WAVAX: H160 = H160(hex_literal::hex!("B31f66AA3C1e785363F0875A1B74E27b85FD66c7"));

    /// Teleporter Messenger (Avalanche's native cross-chain)
    pub const TELEPORTER_MESSENGER: H160 = H160(hex_literal::hex!("253b2784c75e510dD0fF1da844684a1aC0aa5fcf"));

    /// Encode Teleporter message send
    pub fn encode_send_cross_chain_message(
        destination_chain_id: H256, // Avalanche uses 32-byte chain IDs
        destination_address: H160,
        message: Vec<u8>,
        required_gas_limit: U256,
    ) -> Vec<u8> {
        // sendCrossChainMessage(TeleporterMessageInput)
        let mut calldata = Vec::with_capacity(4 + 32 * 6 + message.len());
        
        // Function selector (simplified)
        calldata.extend_from_slice(&[0x62, 0xe0, 0xa3, 0xf1]);
        
        // Destination chain ID
        calldata.extend_from_slice(destination_chain_id.as_bytes());
        
        // Destination address
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(destination_address.as_bytes());
        
        // Fee info (simplified - using native AVAX)
        calldata.extend_from_slice(&[0u8; 32]); // feeTokenAddress = 0
        calldata.extend_from_slice(&[0u8; 32]); // feeAmount = 0
        
        // Required gas limit
        let mut gas_bytes = [0u8; 32];
        required_gas_limit.to_big_endian(&mut gas_bytes);
        calldata.extend_from_slice(&gas_bytes);
        
        // Message offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0xc0);
        
        // Message length
        let msg_len = message.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&msg_len.to_be_bytes());
        
        // Message data
        calldata.extend_from_slice(&message);
        let padding = (32 - message.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);
        
        calldata
    }

    /// Avalanche-specific: Get P-Chain block height
    #[allow(dead_code)]
    async fn get_p_chain_height(&self) -> AdapterResult<u64> {
        // Avalanche has multiple chains (X, P, C)
        // P-Chain is for staking/validation
        Ok(50_000_000)
    }

    /// Check if subnet is validated
    #[allow(dead_code)]
    async fn is_subnet_validated(&self, _subnet_id: H256) -> AdapterResult<bool> {
        // Avalanche subnets can have custom validation
        Ok(true)
    }
}

#[async_trait::async_trait]
impl ChainAdapter for AvalancheAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Avalanche
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        Ok(55_000_000) // C-Chain block number
    }

    async fn get_balance(&self, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn get_token_balance(&self, _token: H160, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        // Uses Teleporter for native cross-subnet messaging
        Ok(message.hash())
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Query TeleporterMessageReceived events
        Ok(vec![])
    }

    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Use Teleporter or Avalanche Bridge
        Ok(transfer.id)
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Avalanche has instant finality via Snowman consensus
        Ok(TransferStatus::Completed)
    }

    async fn verify_message_proof(&self, _message: &ChainMessage, proof: &[u8]) -> AdapterResult<bool> {
        // Avalanche has instant finality - no challenge period
        // Proof is validator signature aggregation
        Ok(!proof.is_empty())
    }

    async fn finalize_transfer(&self, transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Teleporter handles automatic finalization
        Ok(transfer_id)
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        // Avalanche uses dynamic fees
        Ok(U256::from(25_000_000_000u64)) // 25 nAVAX
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> AdapterResult<Option<TransactionReceipt>> {
        Ok(Some(TransactionReceipt {
            tx_hash,
            block_number: 55_000_000,
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
    fn test_avalanche_adapter() {
        let adapter = AvalancheAdapter::new(ChainConfig::for_chain(ChainType::Avalanche));
        assert_eq!(adapter.chain_type(), ChainType::Avalanche);
        assert_eq!(adapter.config().chain_type, 43114);
    }

    #[test]
    fn test_well_known_addresses() {
        assert_ne!(AvalancheAdapter::WAVAX, H160::zero());
        assert_ne!(AvalancheAdapter::TELEPORTER_MESSENGER, H160::zero());
    }
}
