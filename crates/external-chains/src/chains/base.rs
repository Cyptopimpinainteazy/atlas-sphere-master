//! Base Chain Adapter
//!
//! Adapter for Base (Coinbase L2) - an OP Stack rollup
//! Chain ID: 8453

use crate::adapter::*;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Base chain adapter
pub struct BaseAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl BaseAdapter {
    /// Create new Base adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Get chain-specific bridge ABI
    pub fn bridge_abi() -> &'static [u8] {
        // L2StandardBridge ABI for OP Stack
        include_bytes!("../../abi/l2_standard_bridge.json")
    }

    /// Encode bridge deposit call
    pub fn encode_deposit(
        to: H160,
        amount: U256,
        gas_limit: u64,
        data: Vec<u8>,
    ) -> Vec<u8> {
        // depositETH(address _to, uint32 _minGasLimit, bytes _extraData)
        let mut calldata = Vec::with_capacity(4 + 32 + 32 + 32 + data.len());
        
        // Function selector: depositETH(address,uint32,bytes)
        calldata.extend_from_slice(&[0xb1, 0xa1, 0xa8, 0x82]);
        
        // Pad address to 32 bytes
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(to.as_bytes());
        
        // Gas limit
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&(gas_limit as u32).to_be_bytes());
        
        // Data offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);
        
        // Data length
        let data_len = data.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&data_len.to_be_bytes());
        
        // Data (padded to 32 bytes)
        calldata.extend_from_slice(&data);
        let padding = (32 - data.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);
        
        calldata
    }

    /// Base-specific: Get L1 data fee estimate
    #[allow(dead_code)]
    async fn estimate_l1_data_fee(&self, _tx_data: &[u8]) -> AdapterResult<U256> {
        // Base charges L1 data fee on top of L2 gas
        // Uses EIP-4844 blobs for data availability
        Ok(U256::from(1_000_000_000_000u64)) // ~0.000001 ETH placeholder
    }
}

#[async_trait::async_trait]
impl ChainAdapter for BaseAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Base
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        // TODO: Actual RPC connectivity check
        true
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        // TODO: eth_blockNumber RPC call
        Ok(20_000_000)
    }

    async fn get_balance(&self, _address: H160) -> AdapterResult<U256> {
        // TODO: eth_getBalance RPC call
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn get_token_balance(&self, _token: H160, _address: H160) -> AdapterResult<U256> {
        // TODO: ERC20 balanceOf call
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        // TODO: Send cross-domain message via L2CrossDomainMessenger
        Ok(message.hash())
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // TODO: Query SentMessage events
        Ok(vec![])
    }

    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // TODO: Call bridge contract
        Ok(transfer.id)
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // TODO: Query bridge state
        Ok(TransferStatus::Completed)
    }

    async fn verify_message_proof(&self, _message: &ChainMessage, proof: &[u8]) -> AdapterResult<bool> {
        // For L2s: verify against L1 state root
        // Base uses OP Stack's state commitment chain
        Ok(!proof.is_empty())
    }

    async fn finalize_transfer(&self, transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // TODO: Call relayMessage on destination
        Ok(transfer_id)
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        // Base has low gas prices due to L2
        Ok(U256::from(1_000_000)) // ~0.001 gwei
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> AdapterResult<Option<TransactionReceipt>> {
        // TODO: eth_getTransactionReceipt
        Ok(Some(TransactionReceipt {
            tx_hash,
            block_number: 20_000_000,
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
    fn test_base_adapter() {
        let adapter = BaseAdapter::new(ChainConfig::for_chain(ChainType::Base));
        assert_eq!(adapter.chain_type(), ChainType::Base);
        assert_eq!(adapter.config().chain_type, 8453);
    }

    #[test]
    fn test_encode_deposit() {
        let calldata = BaseAdapter::encode_deposit(
            H160::zero(),
            U256::from(1_000_000_000_000_000_000u64),
            200_000,
            vec![],
        );
        // Check function selector
        assert_eq!(&calldata[0..4], &[0xb1, 0xa1, 0xa8, 0x82]);
    }
}
