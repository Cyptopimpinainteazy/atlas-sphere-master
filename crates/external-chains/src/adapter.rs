//! Chain Adapter Trait and Common Types
//!
//! Defines the universal adapter interface that all external chain integrations must implement.

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;
use crate::error::ExternalChainError;
use crate::ChainType;

/// Result type for adapter operations
pub type AdapterResult<T> = Result<T, ExternalChainError>;

/// Chain configuration for external adapters
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct ChainConfig {
    /// Chain type
    pub chain_type: u64, // Store as u64 for SCALE compatibility
    /// RPC endpoint URL (stored as bytes for no_std)
    pub rpc_url: Vec<u8>,
    /// WebSocket endpoint URL (optional)
    pub ws_url: Option<Vec<u8>>,
    /// Bridge contract address on the external chain
    pub bridge_contract: H160,
    /// Settlement contract address
    pub settlement_contract: H160,
    /// Required confirmations for finality
    pub confirmations: u32,
    /// Gas price multiplier (100 = 1x, 150 = 1.5x)
    pub gas_price_multiplier: u32,
    /// Max gas limit for transactions
    pub max_gas_limit: u64,
}

impl ChainConfig {
    /// Create config for a specific chain type
    pub fn for_chain(chain_type: ChainType) -> Self {
        let (bridge, settlement) = Self::default_contracts(chain_type);
        Self {
            chain_type: chain_type.chain_id(),
            rpc_url: chain_type.default_rpc().as_bytes().to_vec(),
            ws_url: None,
            bridge_contract: bridge,
            settlement_contract: settlement,
            confirmations: Self::default_confirmations(chain_type),
            gas_price_multiplier: 100,
            max_gas_limit: 500_000,
        }
    }

    fn default_contracts(chain_type: ChainType) -> (H160, H160) {
        // Placeholder contract addresses - will be deployed
        let bridge = match chain_type {
            ChainType::Base => H160::from_slice(&hex_literal::hex!("0000000000000000000000000000000000008453")),
            ChainType::Arbitrum => H160::from_slice(&hex_literal::hex!("0000000000000000000000000000000042161000")),
            ChainType::Polygon => H160::from_slice(&hex_literal::hex!("0000000000000000000000000000000000000137")),
            ChainType::Avalanche => H160::from_slice(&hex_literal::hex!("0000000000000000000000000000000000043114")),
            ChainType::Bnb => H160::from_slice(&hex_literal::hex!("0000000000000000000000000000000000000056")),
            ChainType::AtlasSphere => H160::zero(),
        };
        (bridge, bridge) // Same for now
    }

    fn default_confirmations(chain_type: ChainType) -> u32 {
        match chain_type {
            ChainType::Base => 1,      // L2, fast finality
            ChainType::Arbitrum => 1,  // L2, fast finality
            ChainType::Polygon => 128, // PoS, needs more confirmations
            ChainType::Avalanche => 1, // Avalanche consensus is instant
            ChainType::Bnb => 15,      // PoSA chain
            ChainType::AtlasSphere => 1,
        }
    }
}

/// Cross-chain message format
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct ChainMessage {
    /// Source chain ID
    pub source_chain: u64,
    /// Destination chain ID
    pub dest_chain: u64,
    /// Sender address on source chain
    pub sender: H160,
    /// Recipient address on destination chain
    pub recipient: H160,
    /// Message nonce
    pub nonce: u64,
    /// Message payload
    pub payload: Vec<u8>,
    /// Value transferred (in wei)
    pub value: U256,
    /// Gas limit for execution
    pub gas_limit: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl ChainMessage {
    /// Create a new cross-chain message
    pub fn new(
        source_chain: u64,
        dest_chain: u64,
        sender: H160,
        recipient: H160,
        payload: Vec<u8>,
        value: U256,
    ) -> Self {
        Self {
            source_chain,
            dest_chain,
            sender,
            recipient,
            nonce: 0,
            payload,
            value,
            gas_limit: 200_000,
            timestamp: 0,
        }
    }

    /// Compute message hash
    pub fn hash(&self) -> H256 {
        use sp_io::hashing::keccak_256;
        let encoded = self.encode();
        H256::from(keccak_256(&encoded))
    }
}

/// Cross-chain transfer request
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct CrossChainTransfer {
    /// Transfer ID
    pub id: H256,
    /// Source chain
    pub source_chain: u64,
    /// Destination chain
    pub dest_chain: u64,
    /// Token address on source chain (zero for native)
    pub source_token: H160,
    /// Token address on destination chain (zero for native)
    pub dest_token: H160,
    /// Sender
    pub sender: H160,
    /// Recipient
    pub recipient: H160,
    /// Amount
    pub amount: U256,
    /// Fee paid
    pub fee: U256,
    /// Transfer status
    pub status: TransferStatus,
    /// Source chain transaction hash
    pub source_tx: Option<H256>,
    /// Destination chain transaction hash
    pub dest_tx: Option<H256>,
}

/// Transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum TransferStatus {
    /// Initiated on source chain
    Initiated,
    /// Pending confirmation
    Pending,
    /// Confirmed on source chain
    Confirmed,
    /// Being processed
    Processing,
    /// Completed on destination chain
    Completed,
    /// Failed
    Failed,
    /// Refunded
    Refunded,
}

/// Universal chain adapter trait
/// 
/// This trait defines the interface that all external chain adapters must implement.
/// The Atlas Kernel uses this interface to communicate with external chains.
#[async_trait::async_trait]
pub trait ChainAdapter: Send + Sync {
    /// Get the chain type this adapter handles
    fn chain_type(&self) -> ChainType;

    /// Get chain configuration
    fn config(&self) -> &ChainConfig;

    /// Check if the adapter is connected
    async fn is_connected(&self) -> bool;

    /// Get current block number
    async fn get_block_number(&self) -> AdapterResult<u64>;

    /// Get native token balance
    async fn get_balance(&self, address: H160) -> AdapterResult<U256>;

    /// Get ERC20 token balance
    async fn get_token_balance(&self, token: H160, address: H160) -> AdapterResult<U256>;

    /// Send a cross-chain message
    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256>;

    /// Receive and process pending messages
    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>>;

    /// Initiate cross-chain transfer
    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256>;

    /// Check transfer status
    async fn check_transfer_status(&self, transfer_id: H256) -> AdapterResult<TransferStatus>;

    /// Verify message proof from source chain
    async fn verify_message_proof(&self, message: &ChainMessage, proof: &[u8]) -> AdapterResult<bool>;

    /// Finalize transfer on destination
    async fn finalize_transfer(&self, transfer_id: H256, proof: Vec<u8>) -> AdapterResult<H256>;

    /// Get gas price estimate
    async fn estimate_gas_price(&self) -> AdapterResult<U256>;

    /// Get transaction receipt
    async fn get_transaction_receipt(&self, tx_hash: H256) -> AdapterResult<Option<TransactionReceipt>>;
}

/// Transaction receipt from external chain
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct TransactionReceipt {
    /// Transaction hash
    pub tx_hash: H256,
    /// Block number
    pub block_number: u64,
    /// Block hash
    pub block_hash: H256,
    /// Transaction index in block
    pub tx_index: u32,
    /// Success status
    pub success: bool,
    /// Gas used
    pub gas_used: u64,
    /// Logs
    pub logs: Vec<LogEntry>,
}

/// Log entry from transaction
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct LogEntry {
    /// Contract address
    pub address: H160,
    /// Topics
    pub topics: Vec<H256>,
    /// Data
    pub data: Vec<u8>,
}

/// Mock adapter for testing
pub struct MockChainAdapter {
    chain_type: ChainType,
    config: ChainConfig,
    connected: bool,
    block_number: u64,
}

impl MockChainAdapter {
    pub fn new(chain_type: ChainType) -> Self {
        Self {
            chain_type,
            config: ChainConfig::for_chain(chain_type),
            connected: true,
            block_number: 1_000_000,
        }
    }
}

#[async_trait::async_trait]
impl ChainAdapter for MockChainAdapter {
    fn chain_type(&self) -> ChainType {
        self.chain_type
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        Ok(self.block_number)
    }

    async fn get_balance(&self, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64)) // 1 ETH
    }

    async fn get_token_balance(&self, _token: H160, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        Ok(message.hash())
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        Ok(vec![])
    }

    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256> {
        Ok(transfer.id)
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        Ok(TransferStatus::Completed)
    }

    async fn verify_message_proof(&self, _message: &ChainMessage, _proof: &[u8]) -> AdapterResult<bool> {
        Ok(true)
    }

    async fn finalize_transfer(&self, transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        Ok(transfer_id)
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        Ok(U256::from(20_000_000_000u64)) // 20 gwei
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> AdapterResult<Option<TransactionReceipt>> {
        Ok(Some(TransactionReceipt {
            tx_hash,
            block_number: self.block_number,
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

    #[tokio::test]
    async fn test_mock_adapter() {
        let adapter = MockChainAdapter::new(ChainType::Base);
        assert_eq!(adapter.chain_type(), ChainType::Base);
        assert!(adapter.is_connected().await);
        
        let block = adapter.get_block_number().await.unwrap();
        assert_eq!(block, 1_000_000);
    }

    #[test]
    fn test_chain_message_hash() {
        let msg = ChainMessage::new(
            8453, // Base
            42,   // Atlas Sphere
            H160::zero(),
            H160::zero(),
            vec![1, 2, 3],
            U256::zero(),
        );
        let hash = msg.hash();
        assert_ne!(hash, H256::zero());
    }
}
