/// Canonical Ledger Bridge for SVM Integration
///
/// This module provides the bridge between SVM (Solana Virtual Machine) state
/// operations and the Atlas Sphere Canonical Ledger (pallet-atlas-kernel storage).
///
/// Architecture:
/// ```
/// SVM Execution ─┬─> In-Memory State (StateBackend)
///                │         ↓
///                │   CanonicalLedgerBridge
///                │         ↓
///                └─> pallet-atlas-kernel::CanonicalLedger<T>
/// ```
///
/// The bridge performs bidirectional synchronization:
/// - READ: SVM account queries fall through to CanonicalLedger
/// - WRITE: SVM account changes are batched and committed atomically
///
/// Key differences from EVM bridge:
/// - SVM uses 32-byte pubkeys (Substrate-native)
/// - SVM tracks lamports instead of wei
/// - SVM has account data + owner semantics

use sp_std::vec::Vec;
use sp_std::collections::btree_map::BTreeMap;
use parity_scale_codec::{Decode, Encode};

/// Asset ID type for canonical ledger
pub type AssetId = u32;

/// Balance type for canonical ledger (lamports)
pub type Balance = u128;

/// Solana public key (32 bytes)
pub type Pubkey = [u8; 32];

/// Pending account state change for batch commit
#[derive(Clone, Debug, Encode, Decode)]
pub struct PendingAccountChange {
    /// Account public key
    pub pubkey: Pubkey,
    /// New lamport balance
    pub new_lamports: u64,
    /// Previous lamport balance (for rollback)
    pub prev_lamports: u64,
    /// New account data
    pub new_data: Vec<u8>,
    /// Previous account data (for rollback)
    pub prev_data: Vec<u8>,
    /// Account owner (program ID)
    pub owner: Pubkey,
    /// Is executable
    pub executable: bool,
}

/// Account state for SVM
#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct AccountState {
    /// Lamport balance
    pub lamports: u64,
    /// Account data
    pub data: Vec<u8>,
    /// Owner program
    pub owner: Pubkey,
    /// Is executable (program)
    pub executable: bool,
    /// Rent epoch
    pub rent_epoch: u64,
}

/// Canonical Ledger Bridge for SVM
///
/// Provides SVM-compatible account operations backed by the Canonical Ledger.
/// Supports both synchronous queries and batched writes.
#[derive(Clone, Debug, Default)]
pub struct SvmCanonicalLedgerBridge {
    /// Pending account changes (for atomic batch commits)
    pending_changes: Vec<PendingAccountChange>,
    /// SOL asset ID in canonical ledger (native token)
    native_asset_id: AssetId,
    /// Account state cache for read optimization
    account_cache: BTreeMap<Pubkey, AccountState>,
    /// Whether in transaction context
    in_transaction: bool,
}

impl SvmCanonicalLedgerBridge {
    /// Create new bridge with default configuration
    pub fn new() -> Self {
        Self {
            pending_changes: Vec::new(),
            native_asset_id: 1, // SOL token (different from ATLAS=0)
            account_cache: BTreeMap::new(),
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

    /// Get native asset ID (SOL)
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
    pub fn commit_transaction(&mut self) -> Result<Vec<PendingAccountChange>, &'static str> {
        if !self.in_transaction {
            return Err("Not in transaction context");
        }

        // Apply changes to cache
        for change in &self.pending_changes {
            let state = AccountState {
                lamports: change.new_lamports,
                data: change.new_data.clone(),
                owner: change.owner,
                executable: change.executable,
                rent_epoch: 0,
            };
            self.account_cache.insert(change.pubkey, state);
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

    /// Get account state
    pub fn get_account(&self, pubkey: &Pubkey) -> Option<AccountState> {
        // Check pending changes first (in transaction context)
        if self.in_transaction {
            for change in self.pending_changes.iter().rev() {
                if change.pubkey == *pubkey {
                    return Some(AccountState {
                        lamports: change.new_lamports,
                        data: change.new_data.clone(),
                        owner: change.owner,
                        executable: change.executable,
                        rent_epoch: 0,
                    });
                }
            }
        }

        // Check cache
        self.account_cache.get(pubkey).cloned()
    }

    /// Get account lamports
    pub fn get_lamports(&self, pubkey: &Pubkey) -> u64 {
        self.get_account(pubkey).map(|a| a.lamports).unwrap_or(0)
    }

    /// Get account data
    pub fn get_data(&self, pubkey: &Pubkey) -> Vec<u8> {
        self.get_account(pubkey).map(|a| a.data).unwrap_or_default()
    }

    /// Set account state
    pub fn set_account(&mut self, pubkey: &Pubkey, state: AccountState) -> Result<(), &'static str> {
        let prev = self.get_account(pubkey).unwrap_or_default();

        if self.in_transaction {
            self.pending_changes.push(PendingAccountChange {
                pubkey: *pubkey,
                new_lamports: state.lamports,
                prev_lamports: prev.lamports,
                new_data: state.data.clone(),
                prev_data: prev.data,
                owner: state.owner,
                executable: state.executable,
            });
        } else {
            self.account_cache.insert(*pubkey, state);
        }

        Ok(())
    }

    /// Set account lamports
    pub fn set_lamports(&mut self, pubkey: &Pubkey, lamports: u64) -> Result<(), &'static str> {
        let mut state = self.get_account(pubkey).unwrap_or_default();
        state.lamports = lamports;
        self.set_account(pubkey, state)
    }

    /// Set account data
    pub fn set_data(&mut self, pubkey: &Pubkey, data: Vec<u8>) -> Result<(), &'static str> {
        let mut state = self.get_account(pubkey).unwrap_or_default();
        state.data = data;
        self.set_account(pubkey, state)
    }

    /// Transfer lamports between accounts
    pub fn transfer_lamports(
        &mut self,
        from: &Pubkey,
        to: &Pubkey,
        amount: u64,
    ) -> Result<(), &'static str> {
        let from_lamports = self.get_lamports(from);

        if from_lamports < amount {
            return Err("Insufficient lamports");
        }

        let to_lamports = self.get_lamports(to);
        let new_to = to_lamports.checked_add(amount).ok_or("Lamports overflow")?;
        let new_from = from_lamports.checked_sub(amount).ok_or("Underflow")?;

        self.set_lamports(from, new_from)?;
        self.set_lamports(to, new_to)?;

        Ok(())
    }

    /// Convert lamports to canonical balance (128-bit for cross-VM compatibility)
    pub fn lamports_to_balance(lamports: u64) -> Balance {
        lamports as Balance
    }

    /// Convert canonical balance to lamports (truncates if overflow)
    pub fn balance_to_lamports(balance: Balance) -> u64 {
        if balance > u64::MAX as Balance {
            u64::MAX
        } else {
            balance as u64
        }
    }

    /// Export pending changes as state changes for pallet-atlas-kernel
    pub fn export_state_changes(&self) -> Vec<StateChangeExport> {
        self.pending_changes
            .iter()
            .map(|change| StateChangeExport {
                address: change.pubkey.to_vec(),
                key: encode_lamport_key(),
                value: encode_lamports(change.new_lamports),
            })
            .collect()
    }

    /// Export account data changes separately
    pub fn export_data_changes(&self) -> Vec<DataChangeExport> {
        self.pending_changes
            .iter()
            .map(|change| DataChangeExport {
                pubkey: change.pubkey,
                data: change.new_data.clone(),
                owner: change.owner,
                executable: change.executable,
            })
            .collect()
    }

    /// Import state from canonical ledger (for initialization)
    pub fn import_accounts(&mut self, accounts: &[(Pubkey, AccountState)]) {
        for (pubkey, state) in accounts {
            self.account_cache.insert(*pubkey, state.clone());
        }
    }

    /// Preload accounts into cache
    pub fn preload_accounts(&mut self, pubkeys: &[Pubkey]) {
        for pubkey in pubkeys {
            self.account_cache.entry(*pubkey).or_insert_with(AccountState::default);
        }
    }

    /// Clear all cached state
    pub fn clear_cache(&mut self) {
        self.account_cache.clear();
        self.pending_changes.clear();
        self.in_transaction = false;
    }

    /// Check if account exists (has non-zero lamports or data)
    pub fn account_exists(&self, pubkey: &Pubkey) -> bool {
        self.get_account(pubkey)
            .map(|a| a.lamports > 0 || !a.data.is_empty())
            .unwrap_or(false)
    }

    /// Get account owner
    pub fn get_owner(&self, pubkey: &Pubkey) -> Option<Pubkey> {
        self.get_account(pubkey).map(|a| a.owner)
    }

    /// Check if account is executable (is a program)
    pub fn is_executable(&self, pubkey: &Pubkey) -> bool {
        self.get_account(pubkey).map(|a| a.executable).unwrap_or(false)
    }
}

/// State change export format compatible with pallet-atlas-kernel::StateChange
#[derive(Clone, Debug)]
pub struct StateChangeExport {
    pub address: Vec<u8>,
    pub key: [u8; 32],
    pub value: [u8; 32],
}

/// Data change export for SVM-specific account data
#[derive(Clone, Debug)]
pub struct DataChangeExport {
    pub pubkey: Pubkey,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
}

/// Encode lamport balance key (slot 0 for lamports)
fn encode_lamport_key() -> [u8; 32] {
    [0u8; 32] // Slot 0 = lamports
}

/// Encode lamports as H256 value
fn encode_lamports(lamports: u64) -> [u8; 32] {
    let mut value = [0u8; 32];
    value[24..32].copy_from_slice(&lamports.to_be_bytes());
    value
}

/// Decode lamports from H256 value
pub fn decode_lamports(value: &[u8; 32]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&value[24..32]);
    u64::from_be_bytes(bytes)
}

/// Runtime storage accessor trait for SVM
///
/// This trait is implemented by the runtime to provide access to
/// the CanonicalLedger storage map for SVM accounts.
pub trait SvmCanonicalLedgerAccess {
    /// Get account lamports from canonical ledger
    fn get_lamports(pubkey: &Pubkey) -> u64;

    /// Set account lamports in canonical ledger
    fn set_lamports(pubkey: &Pubkey, lamports: u64);

    /// Get account data from canonical ledger
    fn get_account_data(pubkey: &Pubkey) -> Vec<u8>;

    /// Set account data in canonical ledger
    fn set_account_data(pubkey: &Pubkey, data: Vec<u8>);

    /// Check if account exists
    fn account_exists(pubkey: &Pubkey) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_set_lamports() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let pubkey = [1u8; 32];

        // Initial lamports should be 0
        assert_eq!(bridge.get_lamports(&pubkey), 0);

        // Set lamports
        bridge.set_lamports(&pubkey, 1000).unwrap();
        assert_eq!(bridge.get_lamports(&pubkey), 1000);
    }

    #[test]
    fn test_transfer_lamports() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let from = [1u8; 32];
        let to = [2u8; 32];

        // Set initial balance
        bridge.set_lamports(&from, 1000).unwrap();

        // Transfer
        bridge.transfer_lamports(&from, &to, 300).unwrap();

        // Verify balances
        assert_eq!(bridge.get_lamports(&from), 700);
        assert_eq!(bridge.get_lamports(&to), 300);
    }

    #[test]
    fn test_insufficient_lamports() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let from = [1u8; 32];
        let to = [2u8; 32];

        bridge.set_lamports(&from, 100).unwrap();

        // Should fail with insufficient lamports
        let result = bridge.transfer_lamports(&from, &to, 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_commit() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let pubkey = [1u8; 32];

        bridge.begin_transaction();
        bridge.set_lamports(&pubkey, 500).unwrap();

        let changes = bridge.commit_transaction().unwrap();

        // Should have one change
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].new_lamports, 500);

        // Value should persist after commit
        assert_eq!(bridge.get_lamports(&pubkey), 500);
    }

    #[test]
    fn test_transaction_rollback() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let pubkey = [1u8; 32];

        // Set initial value
        bridge.set_lamports(&pubkey, 1000).unwrap();

        // Begin transaction
        bridge.begin_transaction();

        // Make changes
        bridge.set_lamports(&pubkey, 500).unwrap();

        // Should see pending value
        assert_eq!(bridge.get_lamports(&pubkey), 500);

        // Rollback
        bridge.rollback_transaction();

        // Should see original value
        assert_eq!(bridge.get_lamports(&pubkey), 1000);
    }

    #[test]
    fn test_account_data() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let pubkey = [1u8; 32];
        let data = vec![1, 2, 3, 4, 5];

        bridge.set_data(&pubkey, data.clone()).unwrap();
        assert_eq!(bridge.get_data(&pubkey), data);
    }

    #[test]
    fn test_full_account_state() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let pubkey = [1u8; 32];
        let owner = [2u8; 32];

        let state = AccountState {
            lamports: 1000,
            data: vec![1, 2, 3],
            owner,
            executable: true,
            rent_epoch: 0,
        };

        bridge.set_account(&pubkey, state.clone()).unwrap();

        let retrieved = bridge.get_account(&pubkey).unwrap();
        assert_eq!(retrieved.lamports, 1000);
        assert_eq!(retrieved.data, vec![1, 2, 3]);
        assert_eq!(retrieved.owner, owner);
        assert!(retrieved.executable);
    }

    #[test]
    fn test_export_state_changes() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let pubkey = [1u8; 32];

        bridge.begin_transaction();
        bridge.set_lamports(&pubkey, 12345).unwrap();

        let exports = bridge.export_state_changes();
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].address, pubkey.to_vec());
    }

    #[test]
    fn test_encode_decode_lamports() {
        let lamports: u64 = 123456789012345;
        let encoded = encode_lamports(lamports);
        let decoded = decode_lamports(&encoded);
        assert_eq!(lamports, decoded);
    }

    #[test]
    fn test_account_exists() {
        let mut bridge = SvmCanonicalLedgerBridge::new();
        let pubkey = [1u8; 32];

        // Should not exist initially
        assert!(!bridge.account_exists(&pubkey));

        // Set lamports, should exist
        bridge.set_lamports(&pubkey, 100).unwrap();
        assert!(bridge.account_exists(&pubkey));
    }

    #[test]
    fn test_balance_conversion() {
        // u64 max should convert cleanly
        let lamports = u64::MAX;
        let balance = SvmCanonicalLedgerBridge::lamports_to_balance(lamports);
        assert_eq!(balance, lamports as Balance);

        // Balance larger than u64::MAX should saturate
        let large_balance: Balance = u128::MAX;
        let converted = SvmCanonicalLedgerBridge::balance_to_lamports(large_balance);
        assert_eq!(converted, u64::MAX);
    }
}
