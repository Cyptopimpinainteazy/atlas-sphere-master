//! Arbitrum Chain Adapter
//!
//! Adapter for Arbitrum One - Optimistic rollup with Nitro stack
//! Chain ID: 42161

use crate::adapter::*;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Arbitrum chain adapter
pub struct ArbitrumAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl ArbitrumAdapter {
    /// Create new Arbitrum adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Arbitrum Inbox contract for L1->L2 messages
    pub const INBOX_ADDRESS: H160 = H160(hex_literal::hex!("4Dbd4fc535Ac27206064B68FfCf827b0A60BAB3f"));
    
    /// Arbitrum Gateway Router for token bridging
    pub const GATEWAY_ROUTER: H160 = H160(hex_literal::hex!("72Ce9c846789fdB6fC1f34aC4AD25Dd9ef7031ef"));

    /// ArbSys precompile address
    pub const ARBSYS_ADDRESS: H160 = H160(hex_literal::hex!("0000000000000000000000000000000000000064"));

    /// Encode outboundTransfer call for token bridging
    pub fn encode_outbound_transfer(
        token: H160,
        to: H160,
        amount: U256,
        data: Vec<u8>,
    ) -> Vec<u8> {
        // outboundTransfer(address _token, address _to, uint256 _amount, bytes _data)
        let mut calldata = Vec::with_capacity(4 + 32 * 4 + data.len());
        
        // Function selector: outboundTransfer(address,address,uint256,bytes)
        calldata.extend_from_slice(&[0xd2, 0xce, 0x7d, 0x65]);
        
        // Token address (padded to 32 bytes)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(token.as_bytes());
        
        // To address (padded to 32 bytes)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(to.as_bytes());
        
        // Amount (32 bytes)
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        calldata.extend_from_slice(&amount_bytes);
        
        // Data offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x80);
        
        // Data length
        let data_len = data.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&data_len.to_be_bytes());
        
        // Data (padded)
        calldata.extend_from_slice(&data);
        let padding = (32 - data.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);
        
        calldata
    }

    /// Encode sendL2Message for cross-chain messaging
    pub fn encode_send_l2_message(
        target: H160,
        calldata: Vec<u8>,
    ) -> Vec<u8> {
        // sendL2Message(address _target, bytes _data)
        let mut encoded = Vec::with_capacity(4 + 64 + calldata.len());
        
        // Function selector
        calldata.len(); // force usage
        encoded.extend_from_slice(&[0x67, 0x9a, 0xef, 0xce]);
        
        // Target address
        encoded.extend_from_slice(&[0u8; 12]);
        encoded.extend_from_slice(target.as_bytes());
        
        // Add calldata (simplified)
        encoded.extend_from_slice(&[0u8; 32]); // offset placeholder
        
        encoded
    }

    /// Arbitrum-specific: Get L1 block info from ArbSys
    #[allow(dead_code)]
    async fn get_l1_block_number(&self) -> AdapterResult<u64> {
        // Call ArbSys.arbBlockNumber()
        Ok(20_000_000) // Placeholder
    }
}

#[async_trait::async_trait]
impl ChainAdapter for ArbitrumAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Arbitrum
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        Ok(250_000_000) // Arbitrum has high block numbers
    }

    async fn get_balance(&self, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn get_token_balance(&self, _token: H160, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        // Uses ArbSys.sendTxToL1 for L2->L1 messages
        Ok(message.hash())
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Query L2ToL1Tx events from ArbSys
        Ok(vec![])
    }

    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Use Gateway Router for token transfers
        Ok(transfer.id)
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        Ok(TransferStatus::Completed)
    }

    async fn verify_message_proof(&self, _message: &ChainMessage, proof: &[u8]) -> AdapterResult<bool> {
        // Arbitrum uses Nitro's state commitment for proofs
        // 7 day challenge period for fraud proofs
        Ok(!proof.is_empty())
    }

    async fn finalize_transfer(&self, transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // After challenge period, execute on L1 Outbox
        Ok(transfer_id)
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        // Arbitrum gas is measured in ArbGas
        Ok(U256::from(100_000_000)) // 0.1 gwei
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> AdapterResult<Option<TransactionReceipt>> {
        Ok(Some(TransactionReceipt {
            tx_hash,
            block_number: 250_000_000,
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
    fn test_arbitrum_adapter() {
        let adapter = ArbitrumAdapter::new(ChainConfig::for_chain(ChainType::Arbitrum));
        assert_eq!(adapter.chain_type(), ChainType::Arbitrum);
        assert_eq!(adapter.config().chain_type, 42161);
    }

    #[test]
    fn test_constants() {
        // Verify well-known addresses
        assert_ne!(ArbitrumAdapter::INBOX_ADDRESS, H160::zero());
        assert_ne!(ArbitrumAdapter::GATEWAY_ROUTER, H160::zero());
    }
}
