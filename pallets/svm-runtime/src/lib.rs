//! # SVM Pallet
//!
//! A Substrate pallet providing Solana Virtual Machine (SVM) functionality for Atlas Sphere.
//!
//! ## Overview
//!
//! This pallet implements a Solana-compatible account model with:
//! - Account creation and management (lamports, owner, executable flag)
//! - Program deployment and upgrades
//! - BPF program execution via solana-rbpf
//! - Rent collection and account lifecycle management
//!
//! ## Account Model
//!
//! SVM accounts follow the Solana model:
//! - Each account has a unique 32-byte pubkey
//! - Accounts store lamports (native token balance)
//! - Accounts have an owner program that controls mutations
//! - Accounts can be marked as executable (programs)
//! - Account data is stored separately for efficient access
//!
//! ## Programs
//!
//! Programs are special accounts marked as executable:
//! - Programs are deployed via BPF Loader
//! - Programs can be upgraded by their upgrade authority
//! - Program execution is metered by compute units

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    /// Maximum account data size (10 MB like Solana)
    pub const MAX_ACCOUNT_DATA_SIZE: u32 = 10 * 1024 * 1024;

    /// Maximum program size (same as Solana's BPF loader limit)
    pub const MAX_PROGRAM_SIZE: u32 = 10 * 1024 * 1024;

    /// Maximum accounts per instruction
    pub const MAX_ACCOUNTS_PER_INSTRUCTION: u32 = 64;

    /// Maximum instruction data size
    pub const MAX_INSTRUCTION_DATA_SIZE: u32 = 1232;

    /// Rent: lamports per byte-year (Solana-compatible)
    pub const LAMPORTS_PER_BYTE_YEAR: u64 = 3480;

    /// Two years of rent exemption (Solana standard)
    pub const RENT_EXEMPTION_YEARS: u64 = 2;

    /// Well-known program IDs
    pub mod program_ids {
        /// System Program ID (all zeros)
        pub const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

        /// BPF Loader Program ID
        pub const BPF_LOADER_PROGRAM_ID: [u8; 32] = {
            let mut arr = [0u8; 32];
            arr[0] = 2;
            arr
        };

        /// BPF Loader Upgradeable Program ID
        pub const BPF_LOADER_UPGRADEABLE_PROGRAM_ID: [u8; 32] = {
            let mut arr = [0u8; 32];
            arr[0] = 2;
            arr[31] = 1;
            arr
        };

        /// Token Program ID
        pub const TOKEN_PROGRAM_ID: [u8; 32] = {
            let mut arr = [0u8; 32];
            arr[0] = 6;
            arr
        };
    }

    /// SVM Account Info - metadata for an SVM account
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    #[scale_info(skip_type_params(T))]
    pub struct SvmAccountInfo<T: Config> {
        /// Balance in lamports
        pub lamports: u64,
        /// Program that owns this account
        pub owner: [u8; 32],
        /// Whether this account is a program
        pub executable: bool,
        /// Rent epoch for rent collection
        pub rent_epoch: u64,
        /// Length of account data (actual data stored separately)
        pub data_len: u32,
        /// Block number when account was created
        pub created_at: BlockNumberFor<T>,
    }

    /// Program Info - metadata for deployed programs
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    #[scale_info(skip_type_params(T))]
    pub struct ProgramInfo<T: Config> {
        /// Authority that can upgrade this program (None = immutable)
        pub upgrade_authority: Option<[u8; 32]>,
        /// Block when program was last deployed/upgraded
        pub last_deploy_block: BlockNumberFor<T>,
        /// Size of program bytecode
        pub bytecode_len: u32,
        /// Whether program is frozen (no more upgrades)
        pub is_frozen: bool,
    }

    /// Account metadata for instruction execution
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
    pub struct AccountMeta {
        /// Account pubkey
        pub pubkey: [u8; 32],
        /// Whether this account must sign the transaction
        pub is_signer: bool,
        /// Whether this account's data can be modified
        pub is_writable: bool,
    }

    /// Instruction to execute against a program
    #[derive(Clone, Encode, Decode, TypeInfo, Debug, PartialEq, Eq)]
    pub struct Instruction {
        /// Program ID to invoke
        pub program_id: [u8; 32],
        /// Accounts involved in the instruction
        pub accounts: Vec<AccountMeta>,
        /// Opaque instruction data
        pub data: Vec<u8>,
    }

    /// Result of program execution
    #[derive(Clone, Encode, Decode, TypeInfo, Debug, PartialEq, Eq, Default)]
    pub struct ExecutionResult {
        /// Whether execution succeeded
        pub success: bool,
        /// Return data from program
        pub return_data: Vec<u8>,
        /// Compute units consumed
        pub compute_units_used: u64,
        /// Log messages from program
        pub logs: Vec<Vec<u8>>,
    }

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for native token operations
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Maximum size of account data
        #[pallet::constant]
        type MaxAccountDataSize: Get<u32>;

        /// Maximum size of program bytecode
        #[pallet::constant]
        type MaxProgramSize: Get<u32>;

        /// Maximum compute units per instruction
        #[pallet::constant]
        type MaxComputeUnits: Get<u64>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
    }

    /// Account storage: pubkey -> account info
    #[pallet::storage]
    #[pallet::getter(fn accounts)]
    pub type Accounts<T: Config> =
        StorageMap<_, Blake2_128Concat, [u8; 32], SvmAccountInfo<T>, OptionQuery>;

    /// Account data storage: pubkey -> data bytes
    #[pallet::storage]
    #[pallet::getter(fn account_data)]
    pub type AccountData<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        BoundedVec<u8, T::MaxAccountDataSize>,
        OptionQuery,
    >;

    /// Program metadata: program_id -> program info
    #[pallet::storage]
    #[pallet::getter(fn programs)]
    pub type Programs<T: Config> =
        StorageMap<_, Blake2_128Concat, [u8; 32], ProgramInfo<T>, OptionQuery>;

    /// Program bytecode: program_id -> bytecode
    #[pallet::storage]
    #[pallet::getter(fn program_data)]
    pub type ProgramData<T: Config> =
        StorageMap<_, Blake2_128Concat, [u8; 32], BoundedVec<u8, T::MaxProgramSize>, OptionQuery>;

    /// Current slot (block number for SVM purposes)
    #[pallet::storage]
    #[pallet::getter(fn current_slot)]
    pub type CurrentSlot<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total lamports in circulation
    #[pallet::storage]
    #[pallet::getter(fn total_lamports)]
    pub type TotalLamports<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Account created
        AccountCreated {
            pubkey: [u8; 32],
            owner: [u8; 32],
            lamports: u64,
        },
        /// Account closed
        AccountClosed {
            pubkey: [u8; 32],
            lamports_recovered: u64,
        },
        /// Lamports transferred
        Transfer {
            from: [u8; 32],
            to: [u8; 32],
            amount: u64,
        },
        /// Program deployed
        ProgramDeployed {
            program_id: [u8; 32],
            authority: Option<[u8; 32]>,
            bytecode_len: u32,
        },
        /// Program upgraded
        ProgramUpgraded {
            program_id: [u8; 32],
            new_bytecode_len: u32,
        },
        /// Program frozen (no more upgrades)
        ProgramFrozen { program_id: [u8; 32] },
        /// Instruction executed
        InstructionExecuted {
            program_id: [u8; 32],
            success: bool,
            compute_units: u64,
        },
        /// Account data allocated
        AccountAllocated { pubkey: [u8; 32], new_size: u32 },
        /// Account ownership assigned
        AccountAssigned {
            pubkey: [u8; 32],
            new_owner: [u8; 32],
        },
        /// Native tokens funded to SVM account
        AccountFunded {
            substrate_account: T::AccountId,
            svm_pubkey: [u8; 32],
            lamports: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Account already exists
        AccountAlreadyExists,
        /// Account not found
        AccountNotFound,
        /// Insufficient lamports for operation
        InsufficientLamports,
        /// Account is not writable
        AccountNotWritable,
        /// Account is not a signer
        AccountNotSigner,
        /// Invalid account owner
        InvalidOwner,
        /// Program not found
        ProgramNotFound,
        /// Program is not executable
        ProgramNotExecutable,
        /// Program is frozen and cannot be upgraded
        ProgramFrozen,
        /// Invalid upgrade authority
        InvalidUpgradeAuthority,
        /// Account data too large
        AccountDataTooLarge,
        /// Program bytecode too large
        ProgramTooLarge,
        /// Instruction data too large
        InstructionDataTooLarge,
        /// Too many accounts in instruction
        TooManyAccounts,
        /// Compute budget exceeded
        ComputeBudgetExceeded,
        /// Program execution failed
        ExecutionFailed,
        /// Invalid program ID
        InvalidProgramId,
        /// Account is executable and cannot be closed
        CannotCloseExecutable,
        /// Insufficient rent
        InsufficientRent,
        /// Arithmetic overflow
        ArithmeticOverflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            CurrentSlot::<T>::mutate(|slot| *slot = slot.saturating_add(1));
            T::DbWeight::get().reads_writes(1, 1)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new SVM account
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_account())]
        pub fn create_account(
            origin: OriginFor<T>,
            pubkey: [u8; 32],
            lamports: u64,
            space: u32,
            owner: [u8; 32],
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            ensure!(
                !Accounts::<T>::contains_key(&pubkey),
                Error::<T>::AccountAlreadyExists
            );

            ensure!(
                space <= T::MaxAccountDataSize::get(),
                Error::<T>::AccountDataTooLarge
            );

            let min_balance = Self::rent_exempt_balance(space);
            ensure!(lamports >= min_balance, Error::<T>::InsufficientRent);

            let account_info = SvmAccountInfo {
                lamports,
                owner,
                executable: false,
                rent_epoch: CurrentSlot::<T>::get(),
                data_len: space,
                created_at: frame_system::Pallet::<T>::block_number(),
            };

            Accounts::<T>::insert(&pubkey, account_info);

            if space > 0 {
                let data = BoundedVec::try_from(sp_std::vec![0u8; space as usize])
                    .map_err(|_| Error::<T>::AccountDataTooLarge)?;
                AccountData::<T>::insert(&pubkey, data);
            }

            TotalLamports::<T>::mutate(|total| *total = total.saturating_add(lamports));

            Self::deposit_event(Event::AccountCreated {
                pubkey,
                owner,
                lamports,
            });

            Ok(())
        }

        /// Deploy a new BPF program
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::deploy_program(bytecode.len() as u32))]
        pub fn deploy_program(
            origin: OriginFor<T>,
            program_id: [u8; 32],
            bytecode: Vec<u8>,
            upgrade_authority: Option<[u8; 32]>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            ensure!(
                !Programs::<T>::contains_key(&program_id),
                Error::<T>::AccountAlreadyExists
            );

            let bytecode_len = bytecode.len() as u32;
            ensure!(
                bytecode_len <= T::MaxProgramSize::get(),
                Error::<T>::ProgramTooLarge
            );

            let program_info = ProgramInfo {
                upgrade_authority,
                last_deploy_block: frame_system::Pallet::<T>::block_number(),
                bytecode_len,
                is_frozen: false,
            };

            Programs::<T>::insert(&program_id, program_info);

            let bounded_bytecode =
                BoundedVec::try_from(bytecode).map_err(|_| Error::<T>::ProgramTooLarge)?;
            ProgramData::<T>::insert(&program_id, bounded_bytecode);

            let lamports = Self::rent_exempt_balance(bytecode_len);
            let account_info = SvmAccountInfo {
                lamports,
                owner: program_ids::BPF_LOADER_PROGRAM_ID,
                executable: true,
                rent_epoch: CurrentSlot::<T>::get(),
                data_len: bytecode_len,
                created_at: frame_system::Pallet::<T>::block_number(),
            };
            Accounts::<T>::insert(&program_id, account_info);

            TotalLamports::<T>::mutate(|total| *total = total.saturating_add(lamports));

            Self::deposit_event(Event::ProgramDeployed {
                program_id,
                authority: upgrade_authority,
                bytecode_len,
            });

            Ok(())
        }

        /// Transfer lamports between accounts
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            from: [u8; 32],
            to: [u8; 32],
            amount: u64,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            Accounts::<T>::try_mutate(&from, |maybe_account| -> DispatchResult {
                let account = maybe_account.as_mut().ok_or(Error::<T>::AccountNotFound)?;
                account.lamports = account
                    .lamports
                    .checked_sub(amount)
                    .ok_or(Error::<T>::InsufficientLamports)?;
                Ok(())
            })?;

            if Accounts::<T>::contains_key(&to) {
                Accounts::<T>::try_mutate(&to, |maybe_account| -> DispatchResult {
                    let account = maybe_account.as_mut().ok_or(Error::<T>::AccountNotFound)?;
                    account.lamports = account
                        .lamports
                        .checked_add(amount)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    Ok(())
                })?;
            } else {
                let account_info = SvmAccountInfo {
                    lamports: amount,
                    owner: program_ids::SYSTEM_PROGRAM_ID,
                    executable: false,
                    rent_epoch: CurrentSlot::<T>::get(),
                    data_len: 0,
                    created_at: frame_system::Pallet::<T>::block_number(),
                };
                Accounts::<T>::insert(&to, account_info);
            }

            Self::deposit_event(Event::Transfer { from, to, amount });

            Ok(())
        }

        /// Close an account and recover lamports
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::close_account())]
        pub fn close_account(
            origin: OriginFor<T>,
            pubkey: [u8; 32],
            recipient: [u8; 32],
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let account = Accounts::<T>::get(&pubkey).ok_or(Error::<T>::AccountNotFound)?;
            ensure!(!account.executable, Error::<T>::CannotCloseExecutable);

            let lamports = account.lamports;

            Accounts::<T>::remove(&pubkey);
            AccountData::<T>::remove(&pubkey);

            if lamports > 0 {
                if Accounts::<T>::contains_key(&recipient) {
                    Accounts::<T>::try_mutate(&recipient, |maybe_account| -> DispatchResult {
                        let account = maybe_account.as_mut().ok_or(Error::<T>::AccountNotFound)?;
                        account.lamports = account
                            .lamports
                            .checked_add(lamports)
                            .ok_or(Error::<T>::ArithmeticOverflow)?;
                        Ok(())
                    })?;
                } else {
                    let account_info = SvmAccountInfo {
                        lamports,
                        owner: program_ids::SYSTEM_PROGRAM_ID,
                        executable: false,
                        rent_epoch: CurrentSlot::<T>::get(),
                        data_len: 0,
                        created_at: frame_system::Pallet::<T>::block_number(),
                    };
                    Accounts::<T>::insert(&recipient, account_info);
                }
            }

            Self::deposit_event(Event::AccountClosed {
                pubkey,
                lamports_recovered: lamports,
            });

            Ok(())
        }

        /// Fund an SVM account by burning native tokens
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::fund_account())]
        pub fn fund_account(
            origin: OriginFor<T>,
            svm_pubkey: [u8; 32],
            amount: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            use sp_runtime::traits::UniqueSaturatedInto;
            let burn_amount: <T::Currency as Currency<T::AccountId>>::Balance =
                amount.unique_saturated_into();
            let _imbalance = T::Currency::withdraw(
                &who,
                burn_amount,
                frame_support::traits::WithdrawReasons::TRANSFER,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            if Accounts::<T>::contains_key(&svm_pubkey) {
                Accounts::<T>::try_mutate(&svm_pubkey, |maybe_account| -> DispatchResult {
                    let account = maybe_account.as_mut().ok_or(Error::<T>::AccountNotFound)?;
                    account.lamports = account
                        .lamports
                        .checked_add(amount)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    Ok(())
                })?;
            } else {
                let account_info = SvmAccountInfo {
                    lamports: amount,
                    owner: program_ids::SYSTEM_PROGRAM_ID,
                    executable: false,
                    rent_epoch: CurrentSlot::<T>::get(),
                    data_len: 0,
                    created_at: frame_system::Pallet::<T>::block_number(),
                };
                Accounts::<T>::insert(&svm_pubkey, account_info);
            }

            TotalLamports::<T>::mutate(|total| *total = total.saturating_add(amount));

            Self::deposit_event(Event::AccountFunded {
                substrate_account: who,
                svm_pubkey,
                lamports: amount,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Calculate rent-exempt balance for a given data size
        pub fn rent_exempt_balance(data_len: u32) -> u64 {
            const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;
            let total_bytes = ACCOUNT_STORAGE_OVERHEAD + data_len as u64;
            total_bytes
                .saturating_mul(LAMPORTS_PER_BYTE_YEAR)
                .saturating_mul(RENT_EXEMPTION_YEARS)
        }

        /// Get account info and data
        pub fn get_account(pubkey: &[u8; 32]) -> Option<(SvmAccountInfo<T>, Vec<u8>)> {
            let info = Accounts::<T>::get(pubkey)?;
            let data = AccountData::<T>::get(pubkey)
                .map(|d| d.into_inner())
                .unwrap_or_default();
            Some((info, data))
        }

        /// Check if account exists
        pub fn account_exists(pubkey: &[u8; 32]) -> bool {
            Accounts::<T>::contains_key(pubkey)
        }

        /// Get program bytecode
        pub fn get_program_bytecode(program_id: &[u8; 32]) -> Option<Vec<u8>> {
            ProgramData::<T>::get(program_id).map(|d| d.into_inner())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{
        assert_noop, assert_ok, parameter_types,
        traits::{ConstU32, ConstU64},
    };
    use sp_core::H256;
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test {
            System: frame_system,
            Balances: pallet_balances,
            Svm: pallet,
        }
    );

    parameter_types! {
        pub const ExistentialDeposit: u128 = 1;
    }

    impl frame_system::Config for Test {
        type BaseCallFilter = frame_support::traits::Everything;
        type Block = Block;
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = ConstU64<250>;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<u128>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = ConstU32<16>;
        type Nonce = u32;
    }

    impl pallet_balances::Config for Test {
        type Balance = u128;
        type DustRemoval = ();
        type RuntimeEvent = RuntimeEvent;
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type MaxLocks = ();
        type MaxReserves = ();
        type MaxHolds = ConstU32<0>;
        type MaxFreezes = ConstU32<0>;
        type ReserveIdentifier = [u8; 8];
        type FreezeIdentifier = ();
        type WeightInfo = ();
        type RuntimeHoldReason = ();
    }

    impl pallet::Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type Currency = Balances;
        type MaxAccountDataSize = ConstU32<{ 10 * 1024 * 1024 }>;
        type MaxProgramSize = ConstU32<{ 10 * 1024 * 1024 }>;
        type MaxComputeUnits = ConstU64<1_400_000>;
        type WeightInfo = ();
    }

    pub fn new_test_ext() -> sp_io::TestExternalities {
        frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .into()
    }

    #[test]
    fn create_account_works() {
        new_test_ext().execute_with(|| {
            let pubkey = [1u8; 32];
            // Rent exempt balance for 100 bytes: (128 + 100) * 3480 * 2 = 1,586,880 lamports
            let lamports = 2_000_000u64;
            let space = 100u32;
            let owner = pallet::program_ids::SYSTEM_PROGRAM_ID;

            assert_ok!(Svm::create_account(
                RuntimeOrigin::signed(1),
                pubkey,
                lamports,
                space,
                owner
            ));

            assert!(pallet::Pallet::<Test>::account_exists(&pubkey));

            let account = pallet::Accounts::<Test>::get(&pubkey).unwrap();
            assert_eq!(account.lamports, lamports);
            assert_eq!(account.owner, owner);
            assert!(!account.executable);
            assert_eq!(account.data_len, space);
        });
    }

    #[test]
    fn create_account_fails_if_exists() {
        new_test_ext().execute_with(|| {
            let pubkey = [1u8; 32];
            let owner = pallet::program_ids::SYSTEM_PROGRAM_ID;

            assert_ok!(Svm::create_account(
                RuntimeOrigin::signed(1),
                pubkey,
                2_000_000,
                100,
                owner
            ));

            assert_noop!(
                Svm::create_account(RuntimeOrigin::signed(1), pubkey, 2_000_000, 100, owner),
                pallet::Error::<Test>::AccountAlreadyExists
            );
        });
    }

    #[test]
    fn transfer_works() {
        new_test_ext().execute_with(|| {
            let from_pubkey = [1u8; 32];
            let to_pubkey = [2u8; 32];
            let owner = pallet::program_ids::SYSTEM_PROGRAM_ID;

            assert_ok!(Svm::create_account(
                RuntimeOrigin::signed(1),
                from_pubkey,
                1_000_000,
                0,
                owner
            ));

            assert_ok!(Svm::transfer(
                RuntimeOrigin::signed(1),
                from_pubkey,
                to_pubkey,
                500_000
            ));

            let from_account = pallet::Accounts::<Test>::get(&from_pubkey).unwrap();
            assert_eq!(from_account.lamports, 500_000);

            let to_account = pallet::Accounts::<Test>::get(&to_pubkey).unwrap();
            assert_eq!(to_account.lamports, 500_000);
        });
    }

    #[test]
    fn transfer_fails_insufficient_lamports() {
        new_test_ext().execute_with(|| {
            let from_pubkey = [1u8; 32];
            let to_pubkey = [2u8; 32];
            let owner = pallet::program_ids::SYSTEM_PROGRAM_ID;
            // Rent exempt for 0 bytes: 128 * 3480 * 2 = 890,880 lamports
            // Use 1_000_000 to be rent-exempt
            assert_ok!(Svm::create_account(
                RuntimeOrigin::signed(1),
                from_pubkey,
                1_000_000,
                0,
                owner
            ));

            assert_noop!(
                Svm::transfer(RuntimeOrigin::signed(1), from_pubkey, to_pubkey, 2_000_000),
                pallet::Error::<Test>::InsufficientLamports
            );
        });
    }

    #[test]
    fn deploy_program_works() {
        new_test_ext().execute_with(|| {
            let program_id = [42u8; 32];
            let bytecode = vec![0u8; 1024];
            let authority = [99u8; 32];

            assert_ok!(Svm::deploy_program(
                RuntimeOrigin::signed(1),
                program_id,
                bytecode.clone(),
                Some(authority)
            ));

            let program = pallet::Programs::<Test>::get(&program_id).unwrap();
            assert_eq!(program.upgrade_authority, Some(authority));
            assert_eq!(program.bytecode_len, 1024);
            assert!(!program.is_frozen);

            let account = pallet::Accounts::<Test>::get(&program_id).unwrap();
            assert!(account.executable);
        });
    }

    #[test]
    fn close_account_works() {
        new_test_ext().execute_with(|| {
            let pubkey = [1u8; 32];
            let recipient = [2u8; 32];
            let owner = pallet::program_ids::SYSTEM_PROGRAM_ID;

            assert_ok!(Svm::create_account(
                RuntimeOrigin::signed(1),
                pubkey,
                1_000_000,
                0,
                owner
            ));

            assert_ok!(Svm::close_account(
                RuntimeOrigin::signed(1),
                pubkey,
                recipient
            ));

            assert!(!pallet::Pallet::<Test>::account_exists(&pubkey));

            let recipient_account = pallet::Accounts::<Test>::get(&recipient).unwrap();
            assert_eq!(recipient_account.lamports, 1_000_000);
        });
    }

    #[test]
    fn rent_exempt_calculation_works() {
        let exemption = pallet::Pallet::<Test>::rent_exempt_balance(0);
        assert_eq!(exemption, 128 * 3480 * 2);

        let exemption_100 = pallet::Pallet::<Test>::rent_exempt_balance(100);
        assert_eq!(exemption_100, (128 + 100) * 3480 * 2);
    }
}
