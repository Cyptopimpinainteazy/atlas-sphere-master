//! Asset Registry and Mirroring
//!
//! Unified asset management across all connected chains.
//! Handles asset mapping, mirroring, and cross-chain token equivalence.

use crate::ChainType;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{H160, H256, U256};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

/// Unique asset identifier across all chains
pub type AssetId = H256;

/// Asset metadata
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct AssetMetadata {
    /// Asset ID
    pub id: AssetId,
    /// Asset name
    pub name: Vec<u8>,
    /// Asset symbol
    pub symbol: Vec<u8>,
    /// Decimals
    pub decimals: u8,
    /// Is native token
    pub is_native: bool,
    /// Origin chain
    pub origin_chain: u64,
    /// Total supply (across all chains)
    pub total_supply: U256,
}

impl AssetMetadata {
    /// Create metadata for a native token
    pub fn native(chain: ChainType) -> Self {
        let (name, symbol) = match chain {
            ChainType::Base => ("Ethereum", "ETH"),
            ChainType::Arbitrum => ("Ethereum", "ETH"),
            ChainType::Polygon => ("Polygon", "POL"),
            ChainType::Avalanche => ("Avalanche", "AVAX"),
            ChainType::Bnb => ("BNB", "BNB"),
            ChainType::AtlasSphere => ("Atlas", "ATLAS"),
        };

        Self {
            id: Self::compute_native_id(chain),
            name: name.as_bytes().to_vec(),
            symbol: symbol.as_bytes().to_vec(),
            decimals: 18,
            is_native: true,
            origin_chain: chain.chain_id(),
            total_supply: U256::zero(), // Dynamic
        }
    }

    /// Compute asset ID for native token
    pub fn compute_native_id(chain: ChainType) -> AssetId {
        use sp_io::hashing::keccak_256;
        let mut data = b"NATIVE:".to_vec();
        data.extend_from_slice(&chain.chain_id().to_le_bytes());
        H256::from(keccak_256(&data))
    }

    /// Compute asset ID for ERC20 token
    pub fn compute_token_id(chain: ChainType, token_address: H160) -> AssetId {
        use sp_io::hashing::keccak_256;
        let mut data = b"ERC20:".to_vec();
        data.extend_from_slice(&chain.chain_id().to_le_bytes());
        data.extend_from_slice(token_address.as_bytes());
        H256::from(keccak_256(&data))
    }
}

/// Token mapping on a specific chain
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct TokenMapping {
    /// Chain ID
    pub chain_id: u64,
    /// Token address (zero for native)
    pub address: H160,
    /// Is this the canonical/origin representation
    pub is_canonical: bool,
    /// Bridge/wrapped token contract (if mirrored)
    pub bridge_contract: Option<H160>,
}

/// Mirrored asset representation
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct MirroredAsset {
    /// Asset metadata
    pub metadata: AssetMetadata,
    /// Mappings on each chain
    pub mappings: Vec<TokenMapping>,
}

impl MirroredAsset {
    /// Create new mirrored asset from origin
    pub fn new(metadata: AssetMetadata) -> Self {
        Self {
            metadata,
            mappings: vec![],
        }
    }

    /// Add chain mapping
    pub fn add_mapping(&mut self, mapping: TokenMapping) {
        // Remove existing mapping for same chain
        self.mappings.retain(|m| m.chain_id != mapping.chain_id);
        self.mappings.push(mapping);
    }

    /// Get mapping for a specific chain
    pub fn get_mapping(&self, chain_id: u64) -> Option<&TokenMapping> {
        self.mappings.iter().find(|m| m.chain_id == chain_id)
    }

    /// Check if asset exists on chain
    pub fn exists_on_chain(&self, chain_id: u64) -> bool {
        self.get_mapping(chain_id).is_some()
    }
}

/// Unified asset registry
#[derive(Debug, Clone, Default, Encode, Decode, TypeInfo)]
pub struct AssetRegistry {
    /// All registered assets by ID
    assets: BTreeMap<AssetId, MirroredAsset>,
    /// Lookup by chain and address
    address_index: BTreeMap<(u64, H160), AssetId>,
}

impl AssetRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        let mut registry = Self::default();
        // Register native tokens for all chains
        registry.register_native_tokens();
        registry
    }

    /// Register all native tokens
    fn register_native_tokens(&mut self) {
        let chains = [
            ChainType::Base,
            ChainType::Arbitrum,
            ChainType::Polygon,
            ChainType::Avalanche,
            ChainType::Bnb,
            ChainType::AtlasSphere,
        ];

        for chain in chains {
            let metadata = AssetMetadata::native(chain);
            let mut asset = MirroredAsset::new(metadata.clone());
            asset.add_mapping(TokenMapping {
                chain_id: chain.chain_id(),
                address: H160::zero(),
                is_canonical: true,
                bridge_contract: None,
            });
            self.assets.insert(metadata.id, asset);
            self.address_index
                .insert((chain.chain_id(), H160::zero()), metadata.id);
        }
    }

    /// Register a new ERC20 token
    pub fn register_token(
        &mut self,
        chain: ChainType,
        address: H160,
        name: Vec<u8>,
        symbol: Vec<u8>,
        decimals: u8,
    ) -> AssetId {
        let id = AssetMetadata::compute_token_id(chain, address);

        let metadata = AssetMetadata {
            id,
            name,
            symbol,
            decimals,
            is_native: false,
            origin_chain: chain.chain_id(),
            total_supply: U256::zero(),
        };

        let mut asset = MirroredAsset::new(metadata);
        asset.add_mapping(TokenMapping {
            chain_id: chain.chain_id(),
            address,
            is_canonical: true,
            bridge_contract: None,
        });

        self.assets.insert(id, asset);
        self.address_index.insert((chain.chain_id(), address), id);

        id
    }

    /// Add mirrored representation on another chain
    pub fn add_mirror(
        &mut self,
        asset_id: AssetId,
        chain: ChainType,
        wrapped_address: H160,
        bridge_contract: H160,
    ) -> Result<(), &'static str> {
        let asset = self.assets.get_mut(&asset_id).ok_or("Asset not found")?;

        asset.add_mapping(TokenMapping {
            chain_id: chain.chain_id(),
            address: wrapped_address,
            is_canonical: false,
            bridge_contract: Some(bridge_contract),
        });

        self.address_index
            .insert((chain.chain_id(), wrapped_address), asset_id);
        Ok(())
    }

    /// Get asset by ID
    pub fn get(&self, id: &AssetId) -> Option<&MirroredAsset> {
        self.assets.get(id)
    }

    /// Lookup asset by chain and address
    pub fn lookup(&self, chain_id: u64, address: H160) -> Option<&MirroredAsset> {
        self.address_index
            .get(&(chain_id, address))
            .and_then(|id| self.assets.get(id))
    }

    /// Get asset ID by chain and address
    pub fn get_asset_id(&self, chain_id: u64, address: H160) -> Option<AssetId> {
        self.address_index.get(&(chain_id, address)).copied()
    }

    /// Get all assets
    pub fn all_assets(&self) -> impl Iterator<Item = &MirroredAsset> {
        self.assets.values()
    }

    /// Count registered assets
    pub fn count(&self) -> usize {
        self.assets.len()
    }
}

/// Well-known stablecoin addresses
pub mod stablecoins {
    use sp_core::H160;

    /// USDC addresses on each chain
    pub mod usdc {
        use super::H160;

        pub const BASE: H160 = H160(hex_literal::hex!(
            "833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
        ));
        pub const ARBITRUM: H160 = H160(hex_literal::hex!(
            "af88d065e77c8cC2239327C5EDb3A432268e5831"
        ));
        pub const POLYGON: H160 = H160(hex_literal::hex!(
            "3c499c542cEF5E3811e1192ce70d8cC03d5c3359"
        ));
        pub const AVALANCHE: H160 = H160(hex_literal::hex!(
            "B97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E"
        ));
        pub const BNB: H160 = H160(hex_literal::hex!(
            "8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d"
        ));
    }

    /// USDT addresses on each chain
    pub mod usdt {
        use super::H160;

        pub const ARBITRUM: H160 = H160(hex_literal::hex!(
            "Fd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"
        ));
        pub const POLYGON: H160 = H160(hex_literal::hex!(
            "c2132D05D31c914a87C6611C10748AEb04B58e8F"
        ));
        pub const AVALANCHE: H160 = H160(hex_literal::hex!(
            "9702230A8Ea53601f5cD2dc00fDBc13d4dF4A8c7"
        ));
        pub const BNB: H160 = H160(hex_literal::hex!(
            "55d398326f99059fF775485246999027B3197955"
        ));
    }

    /// DAI addresses
    pub mod dai {
        use super::H160;

        pub const ARBITRUM: H160 = H160(hex_literal::hex!(
            "DA10009cBd5D07dd0CeCc66161FC93D7c9000da1"
        ));
        pub const POLYGON: H160 = H160(hex_literal::hex!(
            "8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"
        ));
        pub const AVALANCHE: H160 = H160(hex_literal::hex!(
            "d586E7F844cEa2F87f50152665BCbc2C279D8d70"
        ));
        pub const BNB: H160 = H160(hex_literal::hex!(
            "1AF3F329e8BE154074D8769D1FFa4eE058B1DBc3"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_registry_initialization() {
        let registry = AssetRegistry::new();
        // Should have 6 native tokens
        assert_eq!(registry.count(), 6);
    }

    #[test]
    fn test_register_token() {
        let mut registry = AssetRegistry::new();
        let id = registry.register_token(
            ChainType::Base,
            stablecoins::usdc::BASE,
            b"USD Coin".to_vec(),
            b"USDC".to_vec(),
            6,
        );

        let asset = registry.get(&id).unwrap();
        assert_eq!(asset.metadata.decimals, 6);
        assert!(!asset.metadata.is_native);
    }

    #[test]
    fn test_lookup_by_address() {
        let registry = AssetRegistry::new();

        // Lookup ETH on Base
        let asset = registry.lookup(8453, H160::zero()).unwrap();
        assert!(asset.metadata.is_native);
        assert_eq!(asset.metadata.origin_chain, 8453);
    }

    #[test]
    fn test_add_mirror() {
        let mut registry = AssetRegistry::new();

        // Register USDC on Base
        let id = registry.register_token(
            ChainType::Base,
            stablecoins::usdc::BASE,
            b"USD Coin".to_vec(),
            b"USDC".to_vec(),
            6,
        );

        // Mirror to Arbitrum
        let wrapped = H160::from_slice(&[0xAA; 20]);
        let bridge = H160::from_slice(&[0xBB; 20]);
        registry
            .add_mirror(id, ChainType::Arbitrum, wrapped, bridge)
            .unwrap();

        let asset = registry.get(&id).unwrap();
        assert!(asset.exists_on_chain(8453)); // Base
        assert!(asset.exists_on_chain(42161)); // Arbitrum
    }
}
