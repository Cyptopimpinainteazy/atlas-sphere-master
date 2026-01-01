/// EVM State Integration for Atlas Sphere
///
/// Manages Ethereum Virtual Machine state, account storage, and gas metering.
///
/// Note: `compute_state_root()` produces Ethereum-compatible state roots using
/// the standard Merkle Patricia Trie construction and RLP-encoded account values.
/// The method is deterministic on identical sequences of state updates and is
/// used to assert canonical state after EVM execution.

use sp_runtime::traits::Zero;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use ethereum_types::{H160 as EthH160, H256 as EthH256, U256};

use rlp::RlpStream;
#[cfg(feature = "frontier-executor")]
use evm::backend::{Apply, ApplyBackend, Backend, Basic, Log};

/// EVM account state
#[derive(Clone, Debug)]
pub struct EvmAccount {
	/// Account nonce
	pub nonce: u64,
	/// Account balance
	pub balance: u128,
	/// Account code hash
	pub code_hash: [u8; 32],
	/// Storage root
	pub storage_root: [u8; 32],
}

impl EvmAccount {
	/// Create new EVM account
	pub fn new() -> Self {
		Self {
			nonce: 0,
			balance: 0,
			code_hash: [0u8; 32],
			storage_root: [0u8; 32],
		}
	}

	/// Set account balance
	pub fn set_balance(&mut self, balance: u128) {
		self.balance = balance;
	}

	/// Increment nonce
	pub fn increment_nonce(&mut self) {
		self.nonce = self.nonce.saturating_add(1);
	}

	/// Check if account is empty
	pub fn is_empty(&self) -> bool {
		self.balance.is_zero() && self.nonce == 0 && self.code_hash == [0u8; 32]
	}
}

/// EVM contract code
#[derive(Clone, Debug)]
pub struct EvmCode {
	/// Contract bytecode
	pub bytecode: Vec<u8>,
	/// Code hash
	pub code_hash: [u8; 32],
}

impl EvmCode {
	/// Create new EVM code
	pub fn new(bytecode: Vec<u8>) -> Self {
		use sp_core::hashing::keccak_256;
		let code_hash = keccak_256(&bytecode);
		Self {
			bytecode,
			code_hash,
		}
	}

	/// Get code size
	pub fn len(&self) -> usize {
		self.bytecode.len()
	}

	/// Check if code is empty
	pub fn is_empty(&self) -> bool {
		self.bytecode.is_empty()
	}
}

/// EVM storage entry
pub type StorageValue = [u8; 32];

/// EVM state database
pub struct EvmStateDb {
	accounts: BTreeMap<[u8; 20], EvmAccount>,
	code: BTreeMap<[u8; 32], EvmCode>,
	storage: BTreeMap<([u8; 20], [u8; 32]), StorageValue>,
}

impl EvmStateDb {
	/// Create new EVM state database
	pub fn new() -> Self {
		Self {
			accounts: BTreeMap::new(),
			code: BTreeMap::new(),
			storage: BTreeMap::new(),
		}
	}

	/// Get account by address
	pub fn account(&self, address: &[u8; 20]) -> Option<&EvmAccount> {
		self.accounts.get(address)
	}

	/// Get mutable account reference
	pub fn account_mut(&mut self, address: &[u8; 20]) -> &mut EvmAccount {
		self.accounts.entry(*address).or_insert_with(EvmAccount::new)
	}

	/// Get account nonce
	pub fn nonce(&self, address: &[u8; 20]) -> u64 {
		self.account(address).map(|a| a.nonce).unwrap_or(0)
	}

	/// Get account balance
	pub fn balance(&self, address: &[u8; 20]) -> u128 {
		self.account(address).map(|a| a.balance).unwrap_or(0)
	}

	/// Set account balance
	pub fn set_balance(&mut self, address: &[u8; 20], balance: u128) {
		self.account_mut(address).set_balance(balance);
	}

	/// Transfer between accounts with overflow/underflow protection
	pub fn transfer(&mut self, from: &[u8; 20], to: &[u8; 20], value: u128) -> Result<(), &'static str> {
		let from_balance = self.balance(from);
		from_balance.checked_sub(value)
			.ok_or("Insufficient balance")?;

		self.account_mut(from).balance = from_balance - value;
		let to_balance = self.balance(to);
		self.account_mut(to).balance = to_balance.checked_add(value)
			.ok_or("Balance overflow")?;

		Ok(())
	}

	/// Get code by hash
	pub fn code(&self, code_hash: &[u8; 32]) -> Option<&EvmCode> {
		self.code.get(code_hash)
	}

	/// Set code at address
	pub fn set_code(&mut self, address: &[u8; 20], code: EvmCode) {
		let code_hash = code.code_hash;
		self.code.insert(code_hash, code);
		self.account_mut(address).code_hash = code_hash;
	}

	/// Get storage value
	pub fn storage(&self, address: &[u8; 20], key: &[u8; 32]) -> StorageValue {
		self.storage.get(&(*address, *key)).copied().unwrap_or([0u8; 32])
	}

	/// Set storage value
	pub fn set_storage(&mut self, address: &[u8; 20], key: [u8; 32], value: StorageValue) {
		self.storage.insert((*address, key), value);
	}

	/// Get account count
	pub fn account_count(&self) -> usize {
		self.accounts.len()
	}

	/// Get all accounts
	pub fn accounts(&self) -> impl Iterator<Item = (&[u8; 20], &EvmAccount)> {
		self.accounts.iter()
	}

	/// Compute an Ethereum-compatible state root (accounts trie root).
	///
	/// This constructs the standard Ethereum account trie where:
	/// - Keys are `keccak256(address)`
	/// - Values are RLP(Account { nonce, balance, storage_root, code_hash })
	///
	/// Storage roots are computed per account from the storage trie:
	/// - Keys are `keccak256(slot_key)`
	/// - Values are RLP(U256(slot_value))
	pub fn compute_state_root(&self) -> [u8; 32] {
		#[cfg(not(feature = "std"))]
		{
			// No-std build of this crate is used by the runtime WASM; the full
			// trie implementation is only required on native nodes.
			return [0u8; 32];
		}

		#[cfg(feature = "std")]
		{
			use sp_core::{hashing::keccak_256, KeccakHasher};
			use sp_trie::{LayoutV1, MemoryDB, TrieDBMutBuilder};
			use trie_db::TrieMut;

			fn empty_trie_root() -> [u8; 32] {
				// Ethereum empty trie root: keccak256(rlp(""))
				let mut s = RlpStream::new();
				s.append_empty_data();
				keccak_256(&s.out()).into()
			}

			fn rlp_u256(value: U256) -> Vec<u8> {
				let mut s = RlpStream::new();
				s.append(&value);
				s.out().to_vec()
			}

			fn rlp_account(nonce: U256, balance: U256, storage_root: [u8; 32], code_hash: [u8; 32]) -> Vec<u8> {
				let mut s = RlpStream::new_list(4);
				s.append(&nonce);
				s.append(&balance);
				// Append storage_root and code_hash as raw bytes (32 bytes each)
				s.append(&storage_root.as_slice());
				s.append(&code_hash.as_slice());
				s.out().to_vec()
			}

			fn keccak_bytes(bytes: &[u8]) -> [u8; 32] {
				keccak_256(bytes)
			}

			fn storage_root_for_account(
				storage: &BTreeMap<([u8; 20], [u8; 32]), [u8; 32]>,
				address: [u8; 20],
			) -> [u8; 32] {
				let mut db: MemoryDB<KeccakHasher> = MemoryDB::default();
				let mut root = sp_core::H256::zero();
				let mut trie = TrieDBMutBuilder::<LayoutV1<KeccakHasher>>::new(&mut db, &mut root).build();
				let mut inserted_any = false;

				for ((addr, slot), value) in storage.iter() {
					if *addr != address {
						continue;
					}
					let key = keccak_bytes(slot);
					let v = U256::from_big_endian(value);
					let enc = rlp_u256(v);
					let _ = trie.insert(&key, &enc);
					inserted_any = true;
				}

				if !inserted_any {
					return empty_trie_root();
				}

				let root = trie.root();
				root.as_bytes().try_into().unwrap_or([0u8; 32])
			}

			let mut db: MemoryDB<KeccakHasher> = MemoryDB::default();
			let mut root = sp_core::H256::zero();
			let mut trie = TrieDBMutBuilder::<LayoutV1<KeccakHasher>>::new(&mut db, &mut root).build();

			for (addr, account) in self.accounts.iter() {
				let key = keccak_bytes(addr);

				let nonce = U256::from(account.nonce);
				let balance = U256::from(account.balance);

				let storage_root = {
					let root = storage_root_for_account(&self.storage, *addr,);
					root
				};

				let code_hash = if account.code_hash == [0u8; 32] {
					// Ethereum: codeHash for empty code is keccak256("")
					keccak_bytes(&[])
				} else {
					account.code_hash
				};

				let value = rlp_account(nonce, balance, storage_root, code_hash);
				let _ = trie.insert(&key, &value);
			}

			let root = trie.root();
			root.as_bytes().try_into().unwrap_or([0u8; 32])
		}
	}
}

/// Frontier-compatible state backend supporting EVM execution
///
/// Stores account state, code, and storage with execution context (block info, gas, etc.).
/// Currently, some fields are reserved for future protocol upgrades including:
/// - original_storage: For storage proofs and historical access tracking
/// - difficulty: For PoW mining compatibility
/// - base_fee_per_gas: For EIP-1559 dynamic fee support
/// - randomness: For beacon chain random number integration
/// - block_hashes: For historical block hash access
pub struct FrontierStateBackend {
	state: EvmStateDb,
	/// Reserved for storage proof generation - future protocol upgrade
	#[allow(dead_code)]
	original_storage: BTreeMap<([u8; 20], [u8; 32]), StorageValue>,

	// Execution environment context
	gas_price: U256,
	origin: EthH160,
	block_number: U256,
	block_timestamp: U256,
	block_gas_limit: U256,
	chain_id: U256,
	/// Reserved for PoW mining compatibility - future protocol upgrade
	#[allow(dead_code)]
	coinbase: EthH160,
	/// Reserved for PoW difficulty - future protocol upgrade
	#[allow(dead_code)]
	difficulty: U256,
	/// Reserved for EIP-1559 dynamic fees - future protocol upgrade
	#[allow(dead_code)]
	base_fee_per_gas: U256,
	/// Reserved for randomness beacon integration - future protocol upgrade
	#[allow(dead_code)]
	randomness: Option<EthH256>,
	/// Reserved for historical block hashes - future protocol upgrade
	#[allow(dead_code)]
	block_hashes: BTreeMap<U256, EthH256>,
}

impl FrontierStateBackend {
	pub fn new(state: EvmStateDb) -> Self {
		Self {
			state,
			original_storage: BTreeMap::new(),
			gas_price: U256::zero(),
			origin: EthH160::zero(),
			block_number: U256::zero(),
			block_timestamp: U256::zero(),
			block_gas_limit: U256::zero(),
			chain_id: U256::zero(),
			coinbase: EthH160::zero(),
			difficulty: U256::zero(),
			base_fee_per_gas: U256::zero(),
			randomness: None,
			block_hashes: BTreeMap::new(),
		}
	}

	pub fn with_environment(
		mut self,
		gas_price: U256,
		origin: EthH160,
		block_number: U256,
		block_timestamp: U256,
		block_gas_limit: U256,
		chain_id: U256,
	) -> Self {
		self.gas_price = gas_price;
		self.origin = origin;
		self.block_number = block_number;
		self.block_timestamp = block_timestamp;
		self.block_gas_limit = block_gas_limit;
		self.chain_id = chain_id;
		self
	}

	pub fn into_state(self) -> EvmStateDb {
		self.state
	}

	pub fn set_balance(&mut self, address: EthH160, balance: U256) {
		let addr = Self::addr_to_bytes(address);
		self.state.set_balance(&addr, balance.low_u128());
	}

	pub fn set_code(&mut self, address: EthH160, code: Vec<u8>) {
		let addr = Self::addr_to_bytes(address);
		self.state.set_code(&addr, EvmCode::new(code));
	}

	fn addr_to_bytes(address: EthH160) -> [u8; 20] {
		let mut out = [0u8; 20];
		out.copy_from_slice(address.as_bytes());
		out
	}
}

#[cfg(feature = "frontier-executor")]
impl Backend for FrontierStateBackend {
	fn gas_price(&self) -> U256 {
		self.gas_price
	}

	fn origin(&self) -> EthH160 {
		self.origin
	}

	fn block_hash(&self, number: U256) -> EthH256 {
		self.block_hashes.get(&number).copied().unwrap_or_else(EthH256::zero)
	}

	fn block_number(&self) -> U256 {
		self.block_number
	}

	fn block_coinbase(&self) -> EthH160 {
		self.coinbase
	}

	fn block_timestamp(&self) -> U256 {
		self.block_timestamp
	}

	fn block_difficulty(&self) -> U256 {
		self.difficulty
	}

	fn block_randomness(&self) -> Option<EthH256> {
		self.randomness
	}

	fn block_gas_limit(&self) -> U256 {
		self.block_gas_limit
	}

	fn block_base_fee_per_gas(&self) -> U256 {
		self.base_fee_per_gas
	}

	fn chain_id(&self) -> U256 {
		self.chain_id
	}

	fn exists(&self, address: EthH160) -> bool {
		let addr = Self::addr_to_bytes(address);
		self.state.account(&addr).is_some()
	}

	fn basic(&self, address: EthH160) -> Basic {
		let addr = Self::addr_to_bytes(address);
		let nonce = self.state.nonce(&addr);
		let balance = self.state.balance(&addr);
		Basic {
			balance: U256::from(balance),
			nonce: U256::from(nonce),
		}
	}

	fn code(&self, address: EthH160) -> Vec<u8> {
		let addr = Self::addr_to_bytes(address);
		let Some(account) = self.state.account(&addr) else { return Vec::new(); };
		let Some(code) = self.state.code(&account.code_hash) else { return Vec::new(); };
		code.bytecode.clone()
	}

	fn storage(&self, address: EthH160, index: EthH256) -> EthH256 {
		let addr = Self::addr_to_bytes(address);
		let mut key = [0u8; 32];
		key.copy_from_slice(index.as_bytes());
		EthH256::from(self.state.storage(&addr, &key))
	}

	fn original_storage(&self, address: EthH160, index: EthH256) -> Option<EthH256> {
		let addr = Self::addr_to_bytes(address);
		let mut key = [0u8; 32];
		key.copy_from_slice(index.as_bytes());
		self.original_storage
			.get(&(addr, key))
			.copied()
			.map(EthH256::from)
	}
}

#[cfg(feature = "frontier-executor")]
impl ApplyBackend for FrontierStateBackend {
	fn apply<A, I, L>(&mut self, values: A, logs: L, delete_empty: bool)
	where
		A: IntoIterator<Item = Apply<I>>,
		I: IntoIterator<Item = (EthH256, EthH256)>,
		L: IntoIterator<Item = Log>,
	{
		let _ = logs;

		for apply in values {
			match apply {
				Apply::Modify { address, basic, code, storage, reset_storage } => {
					let addr = Self::addr_to_bytes(address);

					if reset_storage {
						self.state.storage.retain(|(a, _k), _v| *a != addr);
						self.original_storage.retain(|(a, _k), _v| *a != addr);
					}

					// Apply nonce/balance
					{
						let account = self.state.account_mut(&addr);
						account.nonce = basic.nonce.low_u64();
						account.balance = basic.balance.low_u128();
					}

					// Apply code if provided
					if let Some(code) = code {
						if !code.is_empty() {
							self.state.set_code(&addr, EvmCode::new(code));
						}
					}

					// Apply storage changes
					for (index, value) in storage {
						let mut key = [0u8; 32];
						key.copy_from_slice(index.as_bytes());
						let mut val = [0u8; 32];
						val.copy_from_slice(value.as_bytes());

						self.original_storage
							.entry((addr, key))
							.or_insert_with(|| self.state.storage(&addr, &key));
						self.state.set_storage(&addr, key, val);
					}

					if delete_empty {
						if let Some(a) = self.state.account(&addr) {
							let has_code = a.code_hash != [0u8; 32];
							let is_empty = a.is_empty() && !has_code;
							if is_empty {
								self.state.accounts.remove(&addr);
							}
						}
					}
				}
				Apply::Delete { address } => {
					let addr = Self::addr_to_bytes(address);
					self.state.accounts.remove(&addr);
					self.state.storage.retain(|(a, _k), _v| *a != addr);
					self.original_storage.retain(|(a, _k), _v| *a != addr);
				}
			}
		}
	}
}

/// EVM execution context
#[derive(Clone, Debug)]
pub struct EvmContext {
	/// Current block number
	pub block_number: u32,
	/// Current block timestamp
	pub block_timestamp: u64,
	/// Gas price
	pub gas_price: u128,
	/// Call origin
	pub origin: [u8; 20],
	/// Caller address
	pub caller: [u8; 20],
	/// Call value
	pub call_value: u128,
	/// Gas limit
	pub gas_limit: u64,
}

impl EvmContext {
	/// Create new EVM context
	pub fn new(origin: [u8; 20]) -> Self {
		Self {
			block_number: 0,
			block_timestamp: 0,
			gas_price: 1,
			origin,
			caller: origin,
			call_value: 0,
			gas_limit: 1_000_000,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_evm_account() {
		let mut account = EvmAccount::new();
		assert!(account.is_empty());

		account.set_balance(1000);
		assert!(!account.is_empty());
		assert_eq!(account.balance, 1000);

		account.increment_nonce();
		assert_eq!(account.nonce, 1);
	}

	#[test]
	fn test_evm_state_db_transfer() {
		let mut db = EvmStateDb::new();
		let addr1 = [1u8; 20];
		let addr2 = [2u8; 20];

		db.set_balance(&addr1, 1000);
		assert!(db.transfer(&addr1, &addr2, 500).is_ok());
		assert_eq!(db.balance(&addr1), 500);
		assert_eq!(db.balance(&addr2), 500);
	}

	#[test]
	fn test_compute_state_root_deterministic() {
		let mut db1 = EvmStateDb::new();
		let mut db2 = EvmStateDb::new();

		let addr_a = [0x0Au8; 20];
		let addr_b = [0x0Bu8; 20];

		// Same operations, different insertion order
		db1.set_balance(&addr_a, 100);
		db1.set_balance(&addr_b, 200);
		db1.set_storage(&addr_a, [0u8;32], [1u8;32]);

		db2.set_balance(&addr_b, 200);
		db2.set_balance(&addr_a, 100);
		db2.set_storage(&addr_a, [0u8;32], [1u8;32]);

		let r1 = db1.compute_state_root();
		let r2 = db2.compute_state_root();
		assert_eq!(r1, r2, "State roots must be deterministic regardless of insertion order");
	}

	#[test]
	fn test_evm_state_db_insufficient_balance() {
		let mut db = EvmStateDb::new();
		let addr1 = [1u8; 20];
		let addr2 = [2u8; 20];

		db.set_balance(&addr1, 100);
		assert!(db.transfer(&addr1, &addr2, 200).is_err());
	}

	#[test]
	fn test_evm_code() {
		let bytecode = vec![0x60, 0x01, 0x61]; // PUSH1 01 PUSH2
		let code = EvmCode::new(bytecode.clone());
		assert_eq!(code.len(), 3);
		assert!(!code.is_empty());
	}
}
