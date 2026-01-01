/// Canonical Ledger Bridge for EVM Integration
///
/// This module provides the bridge between EVM state operations and the
/// Atlas Sphere Canonical Ledger (pallet-atlas-kernel storage).
///
/// Architecture:
/// ```
/// EVM Execution ─┬─> In-Memory State (EvmStateDb)
///                │         ↓
///                │   CanonicalLedgerBridge
///                │         ↓
///                └─> pallet-atlas-kernel::CanonicalLedger<T>
/// ```
///
/// The bridge performs bidirectional synchronization:
/// - READ: EVM state queries fall through to CanonicalLedger
/// - WRITE: EVM state changes are batched and committed atomically

use sp_std::vec::Vec;
use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::traits::Zero;
use ethereum_types::{H160 as EthH160, H256 as EthH256, U256};
use scale_codec::{Decode, Encode};

/// Asset ID type for canonical ledger (matches pallet-atlas-kernel::Config::AssetId)
pub type AssetId = u32;

/// Balance type for canonical ledger
pub type Balance = u128;

/// Account ID type (32 bytes for Substrate accounts)
pub type AccountId32 = [u8; 32];

/// EVM Address to Substrate Account mapping
/// Maps 20-byte EVM addresses to 32-byte Substrate accounts
#[derive(Clone, Debug, Default)]
pub struct AddressMapping {
    /// EVM address -> Substrate account mappings
    evm_to_substrate: BTreeMap<[u8; 20], AccountId32>,
    /// Substrate account -> EVM address mappings (reverse lookup)
    substrate_to_evm: BTreeMap<AccountId32, [u8; 20]>,
}

impl AddressMapping {
    /// Create new address mapping
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a bidirectional mapping between EVM and Substrate addresses
    pub fn register(&mut self, evm_addr: [u8; 20], substrate_addr: AccountId32) {
        self.evm_to_substrate.insert(evm_addr, substrate_addr);
        self.substrate_to_evm.insert(substrate_addr, evm_addr);
    }

    /// Convert EVM address to Substrate account
    /// Uses either registered mapping or deterministic derivation
    pub fn to_substrate(&self, evm_addr: &[u8; 20]) -> AccountId32 {
        if let Some(substrate) = self.evm_to_substrate.get(evm_addr) {
            return *substrate;
        }
        // Deterministic derivation: pad with zeros
        // H160 (20 bytes) -> AccountId32 (32 bytes)
        let mut substrate = [0u8; 32];
        substrate[12..32].copy_from_slice(evm_addr);
        substrate
    }

    /// Convert Substrate account to EVM address
    /// Uses either registered mapping or truncation
    pub fn to_evm(&self, substrate_addr: &AccountId32) -> [u8; 20] {
        if let Some(evm) = self.substrate_to_evm.get(substrate_addr) {
            return *evm;
        }
        // Truncation: take last 20 bytes
        let mut evm = [0u8; 20];
        evm.copy_from_slice(&substrate_addr[12..32]);
        evm
    }
}

/// Pending state change for batch commit
#[derive(Clone, Debug, Encode, Decode)]
pub struct PendingStateChange {
    /// Target account (Substrate format)
    pub account: AccountId32,
    /// Asset ID
    pub asset_id: AssetId,
    /// New balance
    pub new_balance: Balance,
    /// Previous balance (for rollback)
    pub prev_balance: Balance,
}

/// Canonical Ledger Bridge
///
/// Provides EVM-compatible state operations backed by the Canonical Ledger.
/// Supports both synchronous queries and batched writes.
#[derive(Clone, Debug, Default)]
pub struct CanonicalLedgerBridge {
    /// Address mapping registry
    pub address_mapping: AddressMapping,
    /// Pending state changes (for atomic batch commits)
    pending_changes: Vec<PendingStateChange>,
    /// Native asset ID (e.g., ATLAS token = 0)
    native_asset_id: AssetId,
    /// State cache for read optimization
    balance_cache: BTreeMap<(AccountId32, AssetId), Balance>,
    /// Whether in transaction context (changes are pending)
    in_transaction: bool,
}

impl CanonicalLedgerBridge {
    /// Create new bridge with default configuration
    pub fn new() -> Self {
        Self {
            address_mapping: AddressMapping::new(),
            pending_changes: Vec::new(),
            native_asset_id: 0, // ATLAS token
            balance_cache: BTreeMap::new(),
            in_transaction: false,
        }
    }

    /// Create bridge with custom native asset ID
    pub fn with_native_asset(native_asset_id: AssetId) -> Self {
        Self {
            native_asset_id,
            ..Self::new()
        }
    }

    /// Get native asset ID
    pub fn native_asset_id(&self) -> AssetId {
        self.native_asset_id
    }

    /// Begin a transaction context
    /// All subsequent changes will be batched until commit or rollback
    pub fn begin_transaction(&mut self) {
        self.in_transaction = true;
        self.pending_changes.clear();
    }

    /// Commit pending changes
    /// In production, this writes to pallet-atlas-kernel storage
    pub fn commit_transaction(&mut self) -> Result<Vec<PendingStateChange>, &'static str> {
        if !self.in_transaction {
            return Err("Not in transaction context");
        }
        
        // Apply changes to cache
        for change in &self.pending_changes {
            self.balance_cache.insert(
                (change.account, change.asset_id),
                change.new_balance,
            );
        }
        
        let changes = core::mem::take(&mut self.pending_changes);
        self.in_transaction = false;
        Ok(changes)
    }

    /// Rollback pending changes
    pub fn rollback_transaction(&mut self) {
        self.pending_changes.clear();
        self.in_transaction = false;
    }

    /// Get EVM native balance for address (reads from cache or canonical ledger)
    /// Returns balance as U256 for EVM compatibility
    pub fn get_native_balance(&self, evm_addr: &[u8; 20]) -> U256 {
        let substrate_addr = self.address_mapping.to_substrate(evm_addr);
        let balance = self.get_balance(&substrate_addr, self.native_asset_id);
        U256::from(balance)
    }

    /// Get balance for account/asset pair
    pub fn get_balance(&self, account: &AccountId32, asset_id: AssetId) -> Balance {
        // Check pending changes first (in transaction context)
        if self.in_transaction {
            for change in self.pending_changes.iter().rev() {
                if change.account == *account && change.asset_id == asset_id {
                    return change.new_balance;
                }
            }
        }
        
        // Check cache
        if let Some(&balance) = self.balance_cache.get(&(*account, asset_id)) {
            return balance;
        }
        
        // In production: Query pallet-atlas-kernel::CanonicalLedger<T>::get(account, asset_id)
        // For now: Return 0 (empty account)
        0
    }

    /// Set EVM native balance for address
    pub fn set_native_balance(&mut self, evm_addr: &[u8; 20], balance: U256) -> Result<(), &'static str> {
        let substrate_addr = self.address_mapping.to_substrate(evm_addr);
        
        // Convert U256 to u128 (will saturate if overflow)
        let balance_u128 = if balance > U256::from(u128::MAX) {
            return Err("Balance overflow");
        } else {
            balance.low_u128()
        };
        
        self.set_balance(&substrate_addr, self.native_asset_id, balance_u128)
    }

    /// Set balance for account/asset pair
    pub fn set_balance(
        &mut self,
        account: &AccountId32,
        asset_id: AssetId,
        new_balance: Balance,
    ) -> Result<(), &'static str> {
        let prev_balance = self.get_balance(account, asset_id);
        
        if self.in_transaction {
            // Batch the change
            self.pending_changes.push(PendingStateChange {
                account: *account,
                asset_id,
                new_balance,
                prev_balance,
            });
        } else {
            // Direct write to cache
            self.balance_cache.insert((*account, asset_id), new_balance);
        }
        
        Ok(())
    }

    /// Transfer native tokens between EVM addresses
    pub fn transfer(
        &mut self,
        from: &[u8; 20],
        to: &[u8; 20],
        value: U256,
    ) -> Result<(), &'static str> {
        let from_balance = self.get_native_balance(from);
        
        if from_balance < value {
            return Err("Insufficient balance");
        }
        
        let to_balance = self.get_native_balance(to);
        let new_to_balance = to_balance.checked_add(value).ok_or("Balance overflow")?;
        let new_from_balance = from_balance.checked_sub(value).ok_or("Underflow")?;
        
        self.set_native_balance(from, new_from_balance)?;
        self.set_native_balance(to, new_to_balance)?;
        
        Ok(())
    }

    /// Transfer arbitrary asset between accounts
    pub fn transfer_asset(
        &mut self,
        from: &AccountId32,
        to: &AccountId32,
        asset_id: AssetId,
        amount: Balance,
    ) -> Result<(), &'static str> {
        let from_balance = self.get_balance(from, asset_id);
        
        if from_balance < amount {
            return Err("Insufficient balance");
        }
        
        let to_balance = self.get_balance(to, asset_id);
        let new_to = to_balance.checked_add(amount).ok_or("Balance overflow")?;
        let new_from = from_balance.checked_sub(amount).ok_or("Underflow")?;
        
        self.set_balance(from, asset_id, new_from)?;
        self.set_balance(to, asset_id, new_to)?;
        
        Ok(())
    }

    /// Preload balances into cache for a set of accounts
    /// Used to optimize batch reads before EVM execution
    pub fn preload_balances(&mut self, accounts: &[AccountId32], asset_id: AssetId) {
        for account in accounts {
            // In production: Batch query from storage
            // For now: Initialize to 0 if not in cache
            self.balance_cache.entry((*account, asset_id)).or_insert(0);
        }
    }

    /// Preload EVM address balances
    pub fn preload_evm_balances(&mut self, addresses: &[[u8; 20]]) {
        for addr in addresses {
            let substrate = self.address_mapping.to_substrate(addr);
            self.balance_cache.entry((substrate, self.native_asset_id)).or_insert(0);
        }
    }

    /// Export all pending changes as state changes for pallet-atlas-kernel
    /// This is the interface point for integration with the Comit execution flow
    pub fn export_state_changes(&self) -> Vec<StateChangeExport> {
        self.pending_changes
            .iter()
            .map(|change| StateChangeExport {
                address: change.account.to_vec(),
                key: encode_asset_key(change.asset_id),
                value: encode_balance(change.new_balance),
            })
            .collect()
    }

    /// Import state from canonical ledger (for initialization)
    pub fn import_state(&mut self, entries: &[(AccountId32, AssetId, Balance)]) {
        for (account, asset_id, balance) in entries {
            self.balance_cache.insert((*account, *asset_id), *balance);
        }
    }

    /// Clear all cached state
    pub fn clear_cache(&mut self) {
        self.balance_cache.clear();
        self.pending_changes.clear();
        self.in_transaction = false;
    }
}

/// State change export format compatible with pallet-atlas-kernel::StateChange
#[derive(Clone, Debug)]
pub struct StateChangeExport {
    pub address: Vec<u8>,
    pub key: [u8; 32],
    pub value: [u8; 32],
}

/// Encode asset ID as H256 key
fn encode_asset_key(asset_id: AssetId) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[28..32].copy_from_slice(&asset_id.to_be_bytes());
    key
}

/// Encode balance as H256 value
fn encode_balance(balance: Balance) -> [u8; 32] {
    let mut value = [0u8; 32];
    value[16..32].copy_from_slice(&balance.to_be_bytes());
    value
}

/// Decode balance from H256 value
fn decode_balance(value: &[u8; 32]) -> Balance {
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&value[16..32]);
    u128::from_be_bytes(bytes)
}

/// Runtime storage accessor trait
///
/// This trait is implemented by the runtime to provide access to
/// the CanonicalLedger storage map.
pub trait CanonicalLedgerAccess {
    /// Account ID type
    type AccountId;
    /// Asset ID type
    type AssetId;
    /// Balance type
    type Balance;

    /// Get balance from canonical ledger
    fn get_balance(account: &Self::AccountId, asset_id: &Self::AssetId) -> Self::Balance;

    /// Set balance in canonical ledger
    fn set_balance(account: &Self::AccountId, asset_id: &Self::AssetId, balance: Self::Balance);

    /// Check if asset exists
    fn asset_exists(asset_id: &Self::AssetId) -> bool;

    /// Get asset decimals
    fn get_asset_decimals(asset_id: &Self::AssetId) -> Option<u8>;
}

/// EVM Backend integration with Canonical Ledger
///
/// This struct wraps FrontierStateBackend and injects Canonical Ledger
/// reads/writes at the appropriate points in the EVM execution lifecycle.
#[cfg(feature = "frontier-executor")]
pub struct CanonicalBackend {
    /// Underlying EVM state
    inner: crate::state::FrontierStateBackend,
    /// Canonical Ledger bridge
    bridge: CanonicalLedgerBridge,
}

#[cfg(feature = "frontier-executor")]
impl CanonicalBackend {
    /// Create new canonical backend
    pub fn new(inner: crate::state::FrontierStateBackend, bridge: CanonicalLedgerBridge) -> Self {
        Self { inner, bridge }
    }

    /// Get reference to bridge
    pub fn bridge(&self) -> &CanonicalLedgerBridge {
        &self.bridge
    }

    /// Get mutable reference to bridge
    pub fn bridge_mut(&mut self) -> &mut CanonicalLedgerBridge {
        &mut self.bridge
    }

    /// Begin transaction
    pub fn begin_transaction(&mut self) {
        self.bridge.begin_transaction();
    }

    /// Commit transaction and return state changes
    pub fn commit_transaction(&mut self) -> Result<Vec<PendingStateChange>, &'static str> {
        self.bridge.commit_transaction()
    }

    /// Rollback transaction
    pub fn rollback_transaction(&mut self) {
        self.bridge.rollback_transaction();
    }

    /// Get inner backend (for EVM execution)
    pub fn inner(&self) -> &crate::state::FrontierStateBackend {
        &self.inner
    }

    /// Get mutable inner backend
    pub fn inner_mut(&mut self) -> &mut crate::state::FrontierStateBackend {
        &mut self.inner
    }

    /// Consume and return inner backend
    pub fn into_inner(self) -> crate::state::FrontierStateBackend {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_mapping() {
        let mut mapping = AddressMapping::new();
        let evm_addr = [1u8; 20];
        let substrate_addr = [2u8; 32];

        // Test deterministic derivation
        let derived = mapping.to_substrate(&evm_addr);
        assert_eq!(&derived[12..32], &evm_addr);

        // Test registered mapping
        mapping.register(evm_addr, substrate_addr);
        assert_eq!(mapping.to_substrate(&evm_addr), substrate_addr);
        assert_eq!(mapping.to_evm(&substrate_addr), evm_addr);
    }

    #[test]
    fn test_balance_operations() {
        let mut bridge = CanonicalLedgerBridge::new();
        let evm_addr = [1u8; 20];

        // Initial balance should be 0
        assert_eq!(bridge.get_native_balance(&evm_addr), U256::zero());

        // Set balance
        bridge.set_native_balance(&evm_addr, U256::from(1000)).unwrap();
        assert_eq!(bridge.get_native_balance(&evm_addr), U256::from(1000));
    }

    #[test]
    fn test_transfer() {
        let mut bridge = CanonicalLedgerBridge::new();
        let from = [1u8; 20];
        let to = [2u8; 20];

        // Set initial balance
        bridge.set_native_balance(&from, U256::from(1000)).unwrap();

        // Transfer
        bridge.transfer(&from, &to, U256::from(300)).unwrap();

        // Verify balances
        assert_eq!(bridge.get_native_balance(&from), U256::from(700));
        assert_eq!(bridge.get_native_balance(&to), U256::from(300));
    }

    #[test]
    fn test_insufficient_balance() {
        let mut bridge = CanonicalLedgerBridge::new();
        let from = [1u8; 20];
        let to = [2u8; 20];

        bridge.set_native_balance(&from, U256::from(100)).unwrap();

        // Should fail with insufficient balance
        let result = bridge.transfer(&from, &to, U256::from(200));
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_context() {
        let mut bridge = CanonicalLedgerBridge::new();
        let addr = [1u8; 20];
        let substrate = bridge.address_mapping.to_substrate(&addr);

        // Set initial balance
        bridge.set_native_balance(&addr, U256::from(1000)).unwrap();

        // Begin transaction
        bridge.begin_transaction();

        // Make changes
        bridge.set_native_balance(&addr, U256::from(500)).unwrap();

        // Should see pending value
        assert_eq!(bridge.get_native_balance(&addr), U256::from(500));

        // Rollback
        bridge.rollback_transaction();

        // Should see original value
        assert_eq!(bridge.get_native_balance(&addr), U256::from(1000));
    }

    #[test]
    fn test_transaction_commit() {
        let mut bridge = CanonicalLedgerBridge::new();
        let addr = [1u8; 20];

        bridge.begin_transaction();
        bridge.set_native_balance(&addr, U256::from(500)).unwrap();

        let changes = bridge.commit_transaction().unwrap();

        // Should have one change
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].new_balance, 500);

        // Value should persist after commit
        assert_eq!(bridge.get_native_balance(&addr), U256::from(500));
    }

    #[test]
    fn test_export_state_changes() {
        let mut bridge = CanonicalLedgerBridge::new();
        let addr = [1u8; 20];

        bridge.begin_transaction();
        bridge.set_native_balance(&addr, U256::from(12345)).unwrap();

        let exports = bridge.export_state_changes();
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].address.len(), 32); // Substrate account size
    }

    #[test]
    fn test_encode_decode_balance() {
        let balance: Balance = 123456789012345678;
        let encoded = encode_balance(balance);
        let decoded = decode_balance(&encoded);
        assert_eq!(balance, decoded);
    }
}
