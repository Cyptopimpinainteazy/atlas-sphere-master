//! # Asset Manager Pallet
//!
//! Multi-asset management for Atlas Sphere blockchain.
//! Provides cross-chain asset registry, balance tracking, and asset metadata.
//!
//! ## Overview
//!
//! The Asset Manager pallet enables:
//! - Registration and management of fungible assets (native, EVM ERC20, SVM SPL)
//! - Cross-chain asset mapping (EVM address <-> SVM mint <-> Asset ID)
//! - Balance queries and transfers across all asset types
//! - Asset metadata (name, symbol, decimals, total supply)
//! - Asset freezing and admin controls
//!
//! ## Features
//!
//! - **Asset Registry**: Central registry for all assets on the chain
//! - **Cross-Chain Mapping**: Map assets across EVM, SVM, and native representations
//! - **Multi-Asset Balances**: Track balances for any registered asset
//! - **Asset Admin**: Freeze, mint, burn controls for asset administrators
//! - **Fee Configuration**: Per-asset transfer fees and minimum balances

#![cfg_attr(not(feature = "std"), no_std)]


#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency, ExistenceRequirement},
        sp_runtime::traits::{Zero, Saturating, CheckedAdd, CheckedSub},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;
    use codec::{Encode, Decode, MaxEncodedLen};
    use scale_info::TypeInfo;

    /// Asset identifier type
    pub type AssetId = u32;

    /// Balance type for assets
    pub type AssetBalance = u128;

    /// Chain identifier for cross-chain assets
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug)]
    pub enum ChainType {
        /// Native Substrate asset
        Native,
        /// EVM-based asset (ERC20)
        Evm,
        /// SVM-based asset (SPL Token)
        Svm,
        /// Bridged from external chain
        Bridged { chain_id: u32 },
    }

    impl Default for ChainType {
        fn default() -> Self {
            ChainType::Native
        }
    }

    /// Asset status
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
    pub enum AssetStatus {
        #[default]
        Active,
        Frozen,
        Deprecated,
    }

    /// Asset metadata structure
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug)]
    #[scale_info(skip_type_params(StringLimit))]
    pub struct AssetMetadata<StringLimit: Get<u32>> {
        /// Human-readable name
        pub name: BoundedVec<u8, StringLimit>,
        /// Trading symbol
        pub symbol: BoundedVec<u8, StringLimit>,
        /// Decimal places
        pub decimals: u8,
        /// Total supply
        pub total_supply: AssetBalance,
        /// Asset status
        pub status: AssetStatus,
        /// Chain type
        pub chain_type: ChainType,
        /// Minimum balance for account existence
        pub min_balance: AssetBalance,
        /// Transfer fee in basis points (0-10000)
        pub transfer_fee_bps: u16,
    }

    /// Cross-chain asset mapping
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
    pub struct CrossChainMapping {
        /// EVM contract address (20 bytes)
        pub evm_address: Option<[u8; 20]>,
        /// SVM mint address (32 bytes)  
        pub svm_mint: Option<[u8; 32]>,
        /// External chain asset ID
        pub external_id: Option<[u8; 32]>,
    }

    /// Account asset balance with holds
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
    pub struct AccountAssetData {
        /// Free balance available for transfers
        pub free: AssetBalance,
        /// Reserved/locked balance
        pub reserved: AssetBalance,
        /// Frozen balance (cannot transfer or reserve)
        pub frozen: AssetBalance,
    }

    impl AccountAssetData {
        pub fn total(&self) -> AssetBalance {
            self.free.saturating_add(self.reserved).saturating_add(self.frozen)
        }

        pub fn transferable(&self) -> AssetBalance {
            self.free.saturating_sub(self.frozen)
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum length for asset name/symbol strings
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// Maximum number of assets that can be registered
        #[pallet::constant]
        type MaxAssets: Get<u32>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
    }

    /// Weight information trait
    pub trait WeightInfo {
        fn register_asset() -> Weight;
        fn transfer() -> Weight;
        fn mint() -> Weight;
        fn burn() -> Weight;
        fn freeze_asset() -> Weight;
        fn set_metadata() -> Weight;
        fn set_cross_chain_mapping() -> Weight;
    }

    impl WeightInfo for () {
        fn register_asset() -> Weight { Weight::from_parts(50_000_000, 0) }
        fn transfer() -> Weight { Weight::from_parts(30_000_000, 0) }
        fn mint() -> Weight { Weight::from_parts(25_000_000, 0) }
        fn burn() -> Weight { Weight::from_parts(25_000_000, 0) }
        fn freeze_asset() -> Weight { Weight::from_parts(15_000_000, 0) }
        fn set_metadata() -> Weight { Weight::from_parts(20_000_000, 0) }
        fn set_cross_chain_mapping() -> Weight { Weight::from_parts(20_000_000, 0) }
    }

    // ========================================================================
    // Storage
    // ========================================================================

    /// Next available asset ID
    #[pallet::storage]
    #[pallet::getter(fn next_asset_id)]
    pub type NextAssetId<T> = StorageValue<_, AssetId, ValueQuery>;

    /// Asset metadata by ID
    #[pallet::storage]
    #[pallet::getter(fn asset_metadata)]
    pub type Assets<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AssetId,
        AssetMetadata<T::StringLimit>,
        OptionQuery,
    >;

    /// Asset admin account
    #[pallet::storage]
    #[pallet::getter(fn asset_admin)]
    pub type AssetAdmin<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AssetId,
        T::AccountId,
        OptionQuery,
    >;

    /// Cross-chain mappings for assets
    #[pallet::storage]
    #[pallet::getter(fn cross_chain_mapping)]
    pub type CrossChainMappings<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AssetId,
        CrossChainMapping,
        ValueQuery,
    >;

    /// Account balances: (AccountId, AssetId) -> AccountAssetData
    #[pallet::storage]
    #[pallet::getter(fn account_balance)]
    pub type AccountBalances<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        AssetId,
        AccountAssetData,
        ValueQuery,
    >;

    /// EVM address to Asset ID lookup
    #[pallet::storage]
    pub type EvmToAssetId<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 20],
        AssetId,
        OptionQuery,
    >;

    /// SVM mint to Asset ID lookup
    #[pallet::storage]
    pub type SvmToAssetId<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        AssetId,
        OptionQuery,
    >;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Asset registered
        AssetRegistered {
            asset_id: AssetId,
            admin: T::AccountId,
            name: BoundedVec<u8, T::StringLimit>,
            symbol: BoundedVec<u8, T::StringLimit>,
        },
        /// Asset transferred
        Transferred {
            asset_id: AssetId,
            from: T::AccountId,
            to: T::AccountId,
            amount: AssetBalance,
        },
        /// Asset minted
        Minted {
            asset_id: AssetId,
            to: T::AccountId,
            amount: AssetBalance,
        },
        /// Asset burned
        Burned {
            asset_id: AssetId,
            from: T::AccountId,
            amount: AssetBalance,
        },
        /// Asset frozen/unfrozen
        AssetStatusChanged {
            asset_id: AssetId,
            new_status: AssetStatus,
        },
        /// Cross-chain mapping updated
        CrossChainMappingUpdated {
            asset_id: AssetId,
            evm_address: Option<[u8; 20]>,
            svm_mint: Option<[u8; 32]>,
        },
        /// Asset metadata updated
        MetadataUpdated {
            asset_id: AssetId,
        },
        /// Balance reserved
        Reserved {
            asset_id: AssetId,
            who: T::AccountId,
            amount: AssetBalance,
        },
        /// Balance unreserved
        Unreserved {
            asset_id: AssetId,
            who: T::AccountId,
            amount: AssetBalance,
        },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Asset does not exist
        AssetNotFound,
        /// Asset already exists
        AssetAlreadyExists,
        /// Not authorized to perform this action
        NotAuthorized,
        /// Insufficient balance
        InsufficientBalance,
        /// Asset is frozen
        AssetFrozen,
        /// Invalid asset metadata
        InvalidMetadata,
        /// Overflow in balance calculation
        Overflow,
        /// Underflow in balance calculation
        Underflow,
        /// Below minimum balance
        BelowMinimumBalance,
        /// Maximum assets reached
        MaxAssetsReached,
        /// EVM address already mapped
        EvmAddressAlreadyMapped,
        /// SVM mint already mapped
        SvmMintAlreadyMapped,
        /// Invalid decimals (max 18)
        InvalidDecimals,
        /// Account frozen
        AccountFrozen,
    }

    // ========================================================================
    // Extrinsics
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new asset
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_asset())]
        pub fn register_asset(
            origin: OriginFor<T>,
            name: BoundedVec<u8, T::StringLimit>,
            symbol: BoundedVec<u8, T::StringLimit>,
            decimals: u8,
            min_balance: AssetBalance,
            chain_type: ChainType,
        ) -> DispatchResult {
            let admin = ensure_signed(origin)?;

            ensure!(decimals <= 18, Error::<T>::InvalidDecimals);
            ensure!(!name.is_empty(), Error::<T>::InvalidMetadata);
            ensure!(!symbol.is_empty(), Error::<T>::InvalidMetadata);

            let asset_id = NextAssetId::<T>::get();
            ensure!(asset_id < T::MaxAssets::get(), Error::<T>::MaxAssetsReached);

            let metadata = AssetMetadata {
                name: name.clone(),
                symbol: symbol.clone(),
                decimals,
                total_supply: Zero::zero(),
                status: AssetStatus::Active,
                chain_type,
                min_balance,
                transfer_fee_bps: 0,
            };

            Assets::<T>::insert(asset_id, metadata);
            AssetAdmin::<T>::insert(asset_id, admin.clone());
            NextAssetId::<T>::put(asset_id.saturating_add(1));

            Self::deposit_event(Event::AssetRegistered {
                asset_id,
                admin,
                name,
                symbol,
            });

            Ok(())
        }

        /// Transfer asset from sender to recipient
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            asset_id: AssetId,
            to: T::AccountId,
            amount: AssetBalance,
        ) -> DispatchResult {
            let from = ensure_signed(origin)?;

            Self::do_transfer(asset_id, &from, &to, amount)?;

            Self::deposit_event(Event::Transferred {
                asset_id,
                from,
                to,
                amount,
            });

            Ok(())
        }

        /// Mint new tokens (admin only)
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::mint())]
        pub fn mint(
            origin: OriginFor<T>,
            asset_id: AssetId,
            to: T::AccountId,
            amount: AssetBalance,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let admin = AssetAdmin::<T>::get(asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(caller == admin, Error::<T>::NotAuthorized);

            Self::do_mint(asset_id, &to, amount)?;

            Self::deposit_event(Event::Minted {
                asset_id,
                to,
                amount,
            });

            Ok(())
        }

        /// Burn tokens (from sender's balance)
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::burn())]
        pub fn burn(
            origin: OriginFor<T>,
            asset_id: AssetId,
            amount: AssetBalance,
        ) -> DispatchResult {
            let from = ensure_signed(origin)?;

            Self::do_burn(asset_id, &from, amount)?;

            Self::deposit_event(Event::Burned {
                asset_id,
                from,
                amount,
            });

            Ok(())
        }

        /// Freeze or unfreeze an asset (admin only)
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::freeze_asset())]
        pub fn set_asset_status(
            origin: OriginFor<T>,
            asset_id: AssetId,
            status: AssetStatus,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let admin = AssetAdmin::<T>::get(asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(caller == admin, Error::<T>::NotAuthorized);

            Assets::<T>::try_mutate(asset_id, |maybe_metadata| -> DispatchResult {
                let metadata = maybe_metadata.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                metadata.status = status.clone();
                Ok(())
            })?;

            Self::deposit_event(Event::AssetStatusChanged {
                asset_id,
                new_status: status,
            });

            Ok(())
        }

        /// Set cross-chain mapping (admin only)
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::set_cross_chain_mapping())]
        pub fn set_cross_chain_mapping(
            origin: OriginFor<T>,
            asset_id: AssetId,
            evm_address: Option<[u8; 20]>,
            svm_mint: Option<[u8; 32]>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let admin = AssetAdmin::<T>::get(asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(caller == admin, Error::<T>::NotAuthorized);

            // Check for conflicts
            if let Some(addr) = evm_address {
                if let Some(existing) = EvmToAssetId::<T>::get(addr) {
                    ensure!(existing == asset_id, Error::<T>::EvmAddressAlreadyMapped);
                }
            }
            if let Some(mint) = svm_mint {
                if let Some(existing) = SvmToAssetId::<T>::get(mint) {
                    ensure!(existing == asset_id, Error::<T>::SvmMintAlreadyMapped);
                }
            }

            // Update mapping
            let mapping = CrossChainMapping {
                evm_address,
                svm_mint,
                external_id: None,
            };
            CrossChainMappings::<T>::insert(asset_id, mapping);

            // Update reverse lookups
            if let Some(addr) = evm_address {
                EvmToAssetId::<T>::insert(addr, asset_id);
            }
            if let Some(mint) = svm_mint {
                SvmToAssetId::<T>::insert(mint, asset_id);
            }

            Self::deposit_event(Event::CrossChainMappingUpdated {
                asset_id,
                evm_address,
                svm_mint,
            });

            Ok(())
        }

        /// Reserve balance
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn reserve(
            origin: OriginFor<T>,
            asset_id: AssetId,
            amount: AssetBalance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::do_reserve(asset_id, &who, amount)?;

            Self::deposit_event(Event::Reserved {
                asset_id,
                who,
                amount,
            });

            Ok(())
        }

        /// Unreserve balance
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn unreserve(
            origin: OriginFor<T>,
            asset_id: AssetId,
            amount: AssetBalance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::do_unreserve(asset_id, &who, amount)?;

            Self::deposit_event(Event::Unreserved {
                asset_id,
                who,
                amount,
            });

            Ok(())
        }
    }

    // ========================================================================
    // Internal Functions
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Internal transfer logic
        pub fn do_transfer(
            asset_id: AssetId,
            from: &T::AccountId,
            to: &T::AccountId,
            amount: AssetBalance,
        ) -> DispatchResult {
            let metadata = Assets::<T>::get(asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(metadata.status == AssetStatus::Active, Error::<T>::AssetFrozen);

            AccountBalances::<T>::try_mutate(from, asset_id, |data| -> DispatchResult {
                ensure!(data.transferable() >= amount, Error::<T>::InsufficientBalance);
                data.free = data.free.checked_sub(amount).ok_or(Error::<T>::Underflow)?;
                Ok(())
            })?;

            AccountBalances::<T>::try_mutate(to, asset_id, |data| -> DispatchResult {
                data.free = data.free.checked_add(amount).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;

            Ok(())
        }

        /// Internal mint logic
        pub fn do_mint(
            asset_id: AssetId,
            to: &T::AccountId,
            amount: AssetBalance,
        ) -> DispatchResult {
            Assets::<T>::try_mutate(asset_id, |maybe_metadata| -> DispatchResult {
                let metadata = maybe_metadata.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(metadata.status == AssetStatus::Active, Error::<T>::AssetFrozen);
                metadata.total_supply = metadata.total_supply.checked_add(amount)
                    .ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;

            AccountBalances::<T>::try_mutate(to, asset_id, |data| -> DispatchResult {
                data.free = data.free.checked_add(amount).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;

            Ok(())
        }

        /// Internal burn logic
        pub fn do_burn(
            asset_id: AssetId,
            from: &T::AccountId,
            amount: AssetBalance,
        ) -> DispatchResult {
            AccountBalances::<T>::try_mutate(from, asset_id, |data| -> DispatchResult {
                ensure!(data.free >= amount, Error::<T>::InsufficientBalance);
                data.free = data.free.checked_sub(amount).ok_or(Error::<T>::Underflow)?;
                Ok(())
            })?;

            Assets::<T>::try_mutate(asset_id, |maybe_metadata| -> DispatchResult {
                let metadata = maybe_metadata.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                metadata.total_supply = metadata.total_supply.checked_sub(amount)
                    .ok_or(Error::<T>::Underflow)?;
                Ok(())
            })?;

            Ok(())
        }

        /// Internal reserve logic
        pub fn do_reserve(
            asset_id: AssetId,
            who: &T::AccountId,
            amount: AssetBalance,
        ) -> DispatchResult {
            AccountBalances::<T>::try_mutate(who, asset_id, |data| -> DispatchResult {
                ensure!(data.free >= amount, Error::<T>::InsufficientBalance);
                data.free = data.free.checked_sub(amount).ok_or(Error::<T>::Underflow)?;
                data.reserved = data.reserved.checked_add(amount).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;
            Ok(())
        }

        /// Internal unreserve logic
        pub fn do_unreserve(
            asset_id: AssetId,
            who: &T::AccountId,
            amount: AssetBalance,
        ) -> DispatchResult {
            AccountBalances::<T>::try_mutate(who, asset_id, |data| -> DispatchResult {
                let actual = amount.min(data.reserved);
                data.reserved = data.reserved.checked_sub(actual).ok_or(Error::<T>::Underflow)?;
                data.free = data.free.checked_add(actual).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;
            Ok(())
        }

        /// Get asset ID from EVM address
        pub fn asset_id_from_evm(address: [u8; 20]) -> Option<AssetId> {
            EvmToAssetId::<T>::get(address)
        }

        /// Get asset ID from SVM mint
        pub fn asset_id_from_svm(mint: [u8; 32]) -> Option<AssetId> {
            SvmToAssetId::<T>::get(mint)
        }

        /// Get total balance (free + reserved + frozen)
        pub fn total_balance(who: &T::AccountId, asset_id: AssetId) -> AssetBalance {
            AccountBalances::<T>::get(who, asset_id).total()
        }

        /// Get free balance
        pub fn free_balance(who: &T::AccountId, asset_id: AssetId) -> AssetBalance {
            AccountBalances::<T>::get(who, asset_id).free
        }

        /// Get reserved balance
        pub fn reserved_balance(who: &T::AccountId, asset_id: AssetId) -> AssetBalance {
            AccountBalances::<T>::get(who, asset_id).reserved
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_ok, assert_noop, parameter_types};
    use sp_core::H256;
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test {
            System: frame_system,
            AssetManager: pallet,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const StringLimit: u32 = 64;
        pub const MaxAssets: u32 = 1000;
    }

    impl frame_system::Config for Test {
        type BaseCallFilter = frame_support::traits::Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Nonce = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Block = Block;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = frame_support::traits::ConstU32<16>;
        type RuntimeTask = ();
        type SingleBlockMigrations = ();
        type MultiBlockMigrator = ();
        type PreInherents = ();
        type PostInherents = ();
        type PostTransactions = ();
    }

    impl pallet::Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type StringLimit = StringLimit;
        type MaxAssets = MaxAssets;
        type WeightInfo = ();
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        t.into()
    }

    #[test]
    fn register_asset_works() {
        new_test_ext().execute_with(|| {
            let name: BoundedVec<u8, StringLimit> = b"Test Token".to_vec().try_into().unwrap();
            let symbol: BoundedVec<u8, StringLimit> = b"TEST".to_vec().try_into().unwrap();

            assert_ok!(AssetManager::register_asset(
                RuntimeOrigin::signed(1),
                name.clone(),
                symbol.clone(),
                18,
                1000,
                pallet::ChainType::Native,
            ));

            assert_eq!(AssetManager::next_asset_id(), 1);
            let metadata = AssetManager::asset_metadata(0).unwrap();
            assert_eq!(metadata.name, name);
            assert_eq!(metadata.symbol, symbol);
            assert_eq!(metadata.decimals, 18);
        });
    }

    #[test]
    fn mint_and_transfer_works() {
        new_test_ext().execute_with(|| {
            let name: BoundedVec<u8, StringLimit> = b"Test".to_vec().try_into().unwrap();
            let symbol: BoundedVec<u8, StringLimit> = b"TST".to_vec().try_into().unwrap();

            // Register asset
            assert_ok!(AssetManager::register_asset(
                RuntimeOrigin::signed(1),
                name,
                symbol,
                18,
                0,
                pallet::ChainType::Native,
            ));

            // Mint to account 1
            assert_ok!(AssetManager::mint(RuntimeOrigin::signed(1), 0, 1, 1000));
            assert_eq!(AssetManager::free_balance(&1, 0), 1000);

            // Transfer to account 2
            assert_ok!(AssetManager::transfer(RuntimeOrigin::signed(1), 0, 2, 400));
            assert_eq!(AssetManager::free_balance(&1, 0), 600);
            assert_eq!(AssetManager::free_balance(&2, 0), 400);
        });
    }

    #[test]
    fn insufficient_balance_fails() {
        new_test_ext().execute_with(|| {
            let name: BoundedVec<u8, StringLimit> = b"Test".to_vec().try_into().unwrap();
            let symbol: BoundedVec<u8, StringLimit> = b"TST".to_vec().try_into().unwrap();

            assert_ok!(AssetManager::register_asset(
                RuntimeOrigin::signed(1),
                name,
                symbol,
                18,
                0,
                pallet::ChainType::Native,
            ));

            assert_ok!(AssetManager::mint(RuntimeOrigin::signed(1), 0, 1, 100));

            assert_noop!(
                AssetManager::transfer(RuntimeOrigin::signed(1), 0, 2, 200),
                Error::<Test>::InsufficientBalance
            );
        });
    }

    #[test]
    fn reserve_unreserve_works() {
        new_test_ext().execute_with(|| {
            let name: BoundedVec<u8, StringLimit> = b"Test".to_vec().try_into().unwrap();
            let symbol: BoundedVec<u8, StringLimit> = b"TST".to_vec().try_into().unwrap();

            assert_ok!(AssetManager::register_asset(
                RuntimeOrigin::signed(1),
                name,
                symbol,
                18,
                0,
                pallet::ChainType::Native,
            ));

            assert_ok!(AssetManager::mint(RuntimeOrigin::signed(1), 0, 1, 1000));
            assert_ok!(AssetManager::reserve(RuntimeOrigin::signed(1), 0, 300));

            assert_eq!(AssetManager::free_balance(&1, 0), 700);
            assert_eq!(AssetManager::reserved_balance(&1, 0), 300);

            assert_ok!(AssetManager::unreserve(RuntimeOrigin::signed(1), 0, 200));
            assert_eq!(AssetManager::free_balance(&1, 0), 900);
            assert_eq!(AssetManager::reserved_balance(&1, 0), 100);
        });
    }
}
