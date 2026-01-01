// crates/x3-runtime/src/types.rs
// Core type definitions for X3 runtime
//
// Maps X3 types to Rust types for interop with Substrate pallets

use parity_scale_codec::{Encode, Decode, MaxEncodedLen};
use std::fmt;

/// 32-byte address (works for both EVM and SVM)
pub type Address = [u8; 32];

/// 20-byte EVM address
pub type EvmAddress = [u8; 20];

/// 32-byte SVM address (Solana pubkey)
pub type SvmAddress = [u8; 32];

/// Asset ID (matches pallet types)
pub type AssetId = u32;

/// Balance type
pub type Balance = u128;

/// Token ID for X3 Token struct
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct TokenId(pub u32);

impl TokenId {
    pub fn new(id: u32) -> Self {
        TokenId(id)
    }
}

impl From<u32> for TokenId {
    fn from(id: u32) -> Self {
        TokenId(id)
    }
}

impl From<TokenId> for u32 {
    fn from(id: TokenId) -> Self {
        id.0
    }
}

/// X3 Token representation
#[derive(Debug, Clone, Encode, Decode)]
pub struct Token {
    /// Token ID
    pub id: TokenId,
    /// Symbol (e.g., "ATLAS", "USDC")
    pub symbol: String,
    /// Decimal places
    pub decimals: u8,
    /// EVM contract address (if bridged to EVM)
    pub evm_addr: Option<EvmAddress>,
    /// SVM mint address (if bridged to SVM)
    pub svm_mint: Option<SvmAddress>,
}

impl Token {
    pub fn native(id: u32, symbol: &str, decimals: u8) -> Self {
        Token {
            id: TokenId(id),
            symbol: symbol.to_string(),
            decimals,
            evm_addr: None,
            svm_mint: None,
        }
    }

    pub fn with_evm(mut self, addr: EvmAddress) -> Self {
        self.evm_addr = Some(addr);
        self
    }

    pub fn with_svm(mut self, mint: SvmAddress) -> Self {
        self.svm_mint = Some(mint);
        self
    }

    pub fn is_dual_vm(&self) -> bool {
        self.evm_addr.is_some() && self.svm_mint.is_some()
    }
}

/// Bridge chain identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum ChainId {
    /// Atlas L1 (native)
    Atlas = 1,
    /// EVM-compatible chain
    Evm = 2,
    /// SVM-compatible chain (Solana)
    Svm = 3,
}

impl From<u32> for ChainId {
    fn from(id: u32) -> Self {
        match id {
            1 => ChainId::Atlas,
            2 => ChainId::Evm,
            3 => ChainId::Svm,
            _ => ChainId::Atlas, // Default
        }
    }
}

impl From<ChainId> for u32 {
    fn from(id: ChainId) -> Self {
        id as u32
    }
}

/// Bridge state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum BridgeState {
    /// Message pending execution on destination
    Pending = 0,
    /// Message executed on destination
    Executed = 1,
    /// Execution failed, eligible for rollback
    Failed = 2,
    /// Committed to bridge_root, irreversible
    Finalized = 3,
}

impl fmt::Display for BridgeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BridgeState::Pending => write!(f, "Pending"),
            BridgeState::Executed => write!(f, "Executed"),
            BridgeState::Failed => write!(f, "Failed"),
            BridgeState::Finalized => write!(f, "Finalized"),
        }
    }
}

/// DEX route for multi-hop swaps
#[derive(Debug, Clone, Encode, Decode)]
pub struct Route {
    /// Token path
    pub path: Vec<Token>,
    /// Fee per hop in basis points
    pub fees: Vec<u32>,
}

impl Route {
    pub fn new() -> Self {
        Route {
            path: Vec::new(),
            fees: Vec::new(),
        }
    }

    pub fn add_hop(&mut self, token: Token, fee_bps: u32) {
        self.path.push(token);
        if self.path.len() > 1 {
            self.fees.push(fee_bps);
        }
    }

    pub fn hop_count(&self) -> usize {
        if self.path.is_empty() {
            0
        } else {
            self.path.len() - 1
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.path.is_empty() && self.fees.len() == self.hop_count()
    }

    /// Compute canonical route hash
    pub fn hash(&self) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(self.encode());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

impl Default for Route {
    fn default() -> Self {
        Self::new()
    }
}

/// Vault configuration
#[derive(Debug, Clone, Encode, Decode)]
pub struct VaultConfig {
    /// Underlying token
    pub underlying: Token,
    /// Vault share token
    pub share_token: Token,
    /// Minimum deposit
    pub min_deposit: Balance,
    /// Maximum deposit (0 = unlimited)
    pub max_deposit: Balance,
    /// Withdrawal delay in blocks
    pub withdrawal_delay: u32,
}

/// Flashloan parameters
#[derive(Debug, Clone, Encode, Decode)]
pub struct FlashloanParams {
    /// Token to borrow
    pub token: Token,
    /// Amount to borrow
    pub amount: Balance,
    /// Fee in basis points
    pub fee_bps: u32,
    /// Callback function name
    pub callback: String,
    /// Callback data
    pub callback_data: Vec<u8>,
}

impl FlashloanParams {
    /// Calculate fee amount
    pub fn fee_amount(&self) -> Balance {
        self.amount * self.fee_bps as u128 / 10000
    }

    /// Calculate total repayment
    pub fn repayment_amount(&self) -> Balance {
        self.amount + self.fee_amount()
    }
}

/// ZK proof wrapper
#[derive(Debug, Clone, Encode, Decode)]
pub struct ZkProof {
    /// Proof bytes
    pub data: Vec<u8>,
    /// Proof type (groth16, plonk, stark)
    pub proof_type: ProofType,
    /// Public inputs
    pub public_inputs: Vec<u128>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum ProofType {
    Groth16 = 0,
    Plonk = 1,
    Stark = 2,
}

impl ZkProof {
    pub fn new(data: Vec<u8>, proof_type: ProofType) -> Self {
        ZkProof {
            data,
            proof_type,
            public_inputs: Vec::new(),
        }
    }

    pub fn with_inputs(mut self, inputs: Vec<u128>) -> Self {
        self.public_inputs = inputs;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let token = Token::native(1, "ATLAS", 18);
        
        assert_eq!(token.id.0, 1);
        assert_eq!(token.symbol, "ATLAS");
        assert_eq!(token.decimals, 18);
        assert!(token.evm_addr.is_none());
        assert!(token.svm_mint.is_none());
        assert!(!token.is_dual_vm());
    }

    #[test]
    fn test_dual_vm_token() {
        let token = Token::native(1, "ATLAS", 18)
            .with_evm([1u8; 20])
            .with_svm([2u8; 32]);
        
        assert!(token.is_dual_vm());
        assert_eq!(token.evm_addr, Some([1u8; 20]));
        assert_eq!(token.svm_mint, Some([2u8; 32]));
    }

    #[test]
    fn test_route() {
        let mut route = Route::new();
        
        route.add_hop(Token::native(1, "A", 18), 0);
        route.add_hop(Token::native(2, "B", 18), 30);  // 0.3%
        route.add_hop(Token::native(3, "C", 18), 30);
        
        assert_eq!(route.hop_count(), 2);
        assert!(route.is_valid());
        assert_eq!(route.path.len(), 3);
        assert_eq!(route.fees.len(), 2);
    }

    #[test]
    fn test_route_hash() {
        let mut route1 = Route::new();
        route1.add_hop(Token::native(1, "A", 18), 0);
        route1.add_hop(Token::native(2, "B", 18), 30);
        
        let mut route2 = Route::new();
        route2.add_hop(Token::native(1, "A", 18), 0);
        route2.add_hop(Token::native(2, "B", 18), 30);
        
        // Same routes should have same hash
        assert_eq!(route1.hash(), route2.hash());
        
        // Different routes should have different hash
        let mut route3 = Route::new();
        route3.add_hop(Token::native(1, "A", 18), 0);
        route3.add_hop(Token::native(3, "C", 18), 30);
        
        assert_ne!(route1.hash(), route3.hash());
    }

    #[test]
    fn test_flashloan_params() {
        let params = FlashloanParams {
            token: Token::native(1, "USDC", 6),
            amount: 1_000_000, // 1 USDC
            fee_bps: 9, // 0.09%
            callback: "on_flashloan".to_string(),
            callback_data: vec![],
        };
        
        assert_eq!(params.fee_amount(), 90); // 0.09% of 1M
        assert_eq!(params.repayment_amount(), 1_000_090);
    }

    #[test]
    fn test_chain_id_conversion() {
        assert_eq!(ChainId::from(1), ChainId::Atlas);
        assert_eq!(ChainId::from(2), ChainId::Evm);
        assert_eq!(ChainId::from(3), ChainId::Svm);
        assert_eq!(ChainId::from(99), ChainId::Atlas); // Default
        
        assert_eq!(u32::from(ChainId::Atlas), 1);
        assert_eq!(u32::from(ChainId::Evm), 2);
        assert_eq!(u32::from(ChainId::Svm), 3);
    }

    #[test]
    fn test_bridge_state_display() {
        assert_eq!(format!("{}", BridgeState::Pending), "Pending");
        assert_eq!(format!("{}", BridgeState::Executed), "Executed");
        assert_eq!(format!("{}", BridgeState::Failed), "Failed");
        assert_eq!(format!("{}", BridgeState::Finalized), "Finalized");
    }

    #[test]
    fn test_zk_proof() {
        let proof = ZkProof::new(vec![0x01, 0x02, 0x03], ProofType::Groth16)
            .with_inputs(vec![42, 100]);
        
        assert_eq!(proof.proof_type, ProofType::Groth16);
        assert_eq!(proof.public_inputs, vec![42, 100]);
    }
}
