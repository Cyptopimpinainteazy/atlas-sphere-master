//! # Flash Loan Pallet
//!
//! A DeFi primitive enabling uncollateralized loans that must be repaid within
//! a single transaction. Flash loans enable arbitrage, liquidations, and
//! collateral swaps without upfront capital.
//!
//! ## Overview
//!
//! Flash loans work by:
//! 1. Borrower requests a loan amount
//! 2. Pallet transfers funds to borrower
//! 3. Borrower executes arbitrary operations (via callback)
//! 4. Borrower repays loan + fee
//! 5. Transaction reverts if repayment fails
//!
//! ## Security Model
//!
//! - All loans must be repaid within the same transaction
//! - Reentrant calls are blocked during loan execution
//! - Fee calculation uses checked arithmetic
//! - Liquidity providers can withdraw anytime (subject to utilization)
//!
//! ## Integration with Atlas Kernel
//!
//! Flash loans can be used within Comit transactions for cross-VM arbitrage:
//! - Borrow on EVM side, execute on SVM side
//! - Atomic execution ensures repayment or full revert
//!
//! ## Example Usage
//!
//! ```ignore
//! // Request flash loan
//! FlashloanPallet::flash_loan(
//!     origin,
//!     asset_id,
//!     amount,
//!     callback_payload,
//! )?;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]


#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_support::traits::{Currency, ExistenceRequirement, ReservableCurrency};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Saturating, Zero};
    use sp_std::vec::Vec;

    /// Maximum callback payload size (16KB)
    pub const MAX_CALLBACK_SIZE: u32 = 16_384;

    /// Flash loan fee in basis points (0.09% = 9 bps, matching Aave)
    pub const DEFAULT_FEE_BPS: u32 = 9;

    /// Fee denominator (10,000 = 100%)
    pub const FEE_DENOMINATOR: u32 = 10_000;

    /// Flash loan state during execution (reentrancy guard)
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum LoanState {
        /// No active loan
        Idle,
        /// Loan in progress, funds disbursed
        Active,
        /// Loan repayment in progress
        Repaying,
    }

    impl Default for LoanState {
        fn default() -> Self {
            Self::Idle
        }
    }

    /// Flash loan request details
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct LoanRequest<T: Config> {
        /// Borrower account
        pub borrower: T::AccountId,
        /// Asset being borrowed
        pub asset_id: T::AssetId,
        /// Amount borrowed
        pub amount: BalanceOf<T>,
        /// Fee amount
        pub fee: BalanceOf<T>,
        /// Block when loan was initiated
        pub initiated_block: BlockNumberFor<T>,
    }

    /// Pool configuration for an asset
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct PoolConfig<Balance> {
        /// Fee in basis points
        pub fee_bps: u32,
        /// Minimum loan amount
        pub min_loan: Balance,
        /// Maximum loan amount (0 = unlimited up to pool balance)
        pub max_loan: Balance,
        /// Whether pool is active
        pub active: bool,
    }

    impl<Balance: Default + Zero> Default for PoolConfig<Balance> {
        fn default() -> Self {
            Self {
                fee_bps: DEFAULT_FEE_BPS,
                min_loan: Balance::zero(),
                max_loan: Balance::zero(), // Unlimited
                active: true,
            }
        }
    }

    /// Balance type from Currency trait
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for flash loans
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Asset identifier type
        type AssetId: Parameter + Member + Ord + Default + Copy + MaxEncodedLen;

        /// Maximum number of concurrent loan requests (per block)
        #[pallet::constant]
        type MaxLoansPerBlock: Get<u32>;

        /// Weight info for benchmarking
        type WeightInfo: WeightInfo;

        /// Origin that can configure pools
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Flash loan callback executor
        /// In production, this would integrate with EVM/SVM execution
        type CallbackExecutor: FlashLoanCallback<Self>;
    }

    /// Flash loan callback trait
    /// Implementors execute the borrower's operations between loan and repayment
    pub trait FlashLoanCallback<T: Config> {
        /// Execute callback with borrowed funds
        /// Returns Ok(repaid_amount) on success, Err on failure
        fn execute_callback(
            borrower: &T::AccountId,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
            callback_data: &[u8],
        ) -> Result<BalanceOf<T>, DispatchError>;
    }

    /// Default callback implementation (no-op, for testing)
    impl<T: Config> FlashLoanCallback<T> for () {
        fn execute_callback(
            _borrower: &T::AccountId,
            _asset_id: T::AssetId,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
            _callback_data: &[u8],
        ) -> Result<BalanceOf<T>, DispatchError> {
            // Default: assume borrower repays exact amount + fee
            amount.checked_add(&fee).ok_or(DispatchError::Arithmetic(sp_runtime::ArithmeticError::Overflow))
        }
    }

    /// Pallet storage
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Pool liquidity for each asset
    #[pallet::storage]
    pub type PoolLiquidity<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, BalanceOf<T>, ValueQuery>;

    /// Pool configuration for each asset
    #[pallet::storage]
    pub type PoolConfigs<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, PoolConfig<BalanceOf<T>>, ValueQuery>;

    /// Current loan state (reentrancy guard)
    #[pallet::storage]
    pub type CurrentLoanState<T: Config> = StorageValue<_, LoanState, ValueQuery>;

    /// Active loan request (if any)
    #[pallet::storage]
    pub type ActiveLoan<T: Config> = StorageValue<_, LoanRequest<T>>;

    /// Total fees collected per asset
    #[pallet::storage]
    pub type TotalFeesCollected<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, BalanceOf<T>, ValueQuery>;

    /// Loans executed per block (rate limiting)
    #[pallet::storage]
    pub type LoansThisBlock<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Provider liquidity shares (for proportional fee distribution)
    #[pallet::storage]
    pub type ProviderShares<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    /// Total shares for each pool
    #[pallet::storage]
    pub type TotalShares<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Flash loan executed successfully
        FlashLoanExecuted {
            borrower: T::AccountId,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
        },
        /// Flash loan failed (reverted)
        FlashLoanFailed {
            borrower: T::AccountId,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
            reason: FlashLoanFailure,
        },
        /// Liquidity added to pool
        LiquidityAdded {
            provider: T::AccountId,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
            shares: BalanceOf<T>,
        },
        /// Liquidity removed from pool
        LiquidityRemoved {
            provider: T::AccountId,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
            shares: BalanceOf<T>,
        },
        /// Pool configuration updated
        PoolConfigUpdated {
            asset_id: T::AssetId,
            fee_bps: u32,
            active: bool,
        },
    }

    /// Flash loan failure reasons
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum FlashLoanFailure {
        /// Insufficient pool liquidity
        InsufficientLiquidity,
        /// Loan amount below minimum
        BelowMinimum,
        /// Loan amount above maximum
        AboveMaximum,
        /// Pool is not active
        PoolInactive,
        /// Reentrancy detected
        ReentrancyDetected,
        /// Callback execution failed
        CallbackFailed,
        /// Repayment insufficient
        InsufficientRepayment,
        /// Rate limit exceeded
        RateLimitExceeded,
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Pool does not have enough liquidity
        InsufficientLiquidity,
        /// Loan amount is below minimum
        BelowMinimum,
        /// Loan amount exceeds maximum
        AboveMaximum,
        /// Pool is not active
        PoolInactive,
        /// Reentrant flash loan detected
        ReentrancyDetected,
        /// Callback execution failed
        CallbackFailed,
        /// Repayment was insufficient
        InsufficientRepayment,
        /// Arithmetic overflow
        ArithmeticOverflow,
        /// Rate limit exceeded for this block
        RateLimitExceeded,
        /// Callback payload too large
        CallbackTooLarge,
        /// Provider has no shares to withdraw
        NoShares,
        /// Withdrawal would drain pool below minimum
        InsufficientPoolBalance,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Reset per-block loan counter
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            LoansThisBlock::<T>::put(0u32);
            Weight::from_parts(1_000, 0)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Execute a flash loan
        ///
        /// # Arguments
        /// - `asset_id`: Asset to borrow
        /// - `amount`: Amount to borrow
        /// - `callback_data`: Data passed to callback executor
        ///
        /// # Security
        /// - Reentrancy is blocked
        /// - Loan must be repaid within this call
        /// - Transaction reverts on any failure
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::flash_loan())]
        pub fn flash_loan(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
            callback_data: Vec<u8>,
        ) -> DispatchResult {
            let borrower = ensure_signed(origin)?;

            // Validate callback size
            ensure!(
                callback_data.len() <= MAX_CALLBACK_SIZE as usize,
                Error::<T>::CallbackTooLarge
            );

            // Check reentrancy
            ensure!(
                CurrentLoanState::<T>::get() == LoanState::Idle,
                Error::<T>::ReentrancyDetected
            );

            // Check rate limit
            let loans_count = LoansThisBlock::<T>::get();
            ensure!(
                loans_count < T::MaxLoansPerBlock::get(),
                Error::<T>::RateLimitExceeded
            );

            // Get pool config
            let config = PoolConfigs::<T>::get(asset_id);
            ensure!(config.active, Error::<T>::PoolInactive);

            // Validate amount bounds
            ensure!(amount >= config.min_loan, Error::<T>::BelowMinimum);
            if !config.max_loan.is_zero() {
                ensure!(amount <= config.max_loan, Error::<T>::AboveMaximum);
            }

            // Check liquidity
            let liquidity = PoolLiquidity::<T>::get(asset_id);
            ensure!(liquidity >= amount, Error::<T>::InsufficientLiquidity);

            // Calculate fee
            let fee = Self::calculate_fee(amount, config.fee_bps)?;
            let repayment_required = amount
                .checked_add(&fee)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // Set loan state (reentrancy guard)
            CurrentLoanState::<T>::put(LoanState::Active);

            // Record active loan
            let loan_request = LoanRequest {
                borrower: borrower.clone(),
                asset_id,
                amount,
                fee,
                initiated_block: <frame_system::Pallet<T>>::block_number(),
            };
            ActiveLoan::<T>::put(loan_request);

            // Increment loans counter
            LoansThisBlock::<T>::mutate(|c| *c = c.saturating_add(1));

            // Disburse funds to borrower
            // In production, this would transfer from pool reserve account
            PoolLiquidity::<T>::mutate(asset_id, |l| *l = l.saturating_sub(amount));

            // Execute callback
            CurrentLoanState::<T>::put(LoanState::Repaying);
            
            let callback_result = T::CallbackExecutor::execute_callback(
                &borrower,
                asset_id,
                amount,
                fee,
                &callback_data,
            );

            // Verify repayment
            match callback_result {
                Ok(repaid) => {
                    ensure!(repaid >= repayment_required, Error::<T>::InsufficientRepayment);

                    // Return funds to pool
                    PoolLiquidity::<T>::mutate(asset_id, |l| {
                        *l = l.saturating_add(repayment_required);
                    });

                    // Track fees
                    TotalFeesCollected::<T>::mutate(asset_id, |f| {
                        *f = f.saturating_add(fee);
                    });

                    // Clear loan state
                    CurrentLoanState::<T>::put(LoanState::Idle);
                    ActiveLoan::<T>::kill();

                    Self::deposit_event(Event::FlashLoanExecuted {
                        borrower,
                        asset_id,
                        amount,
                        fee,
                    });

                    Ok(())
                }
                Err(_) => {
                    // Revert: return funds to pool
                    PoolLiquidity::<T>::mutate(asset_id, |l| *l = l.saturating_add(amount));

                    // Clear loan state
                    CurrentLoanState::<T>::put(LoanState::Idle);
                    ActiveLoan::<T>::kill();

                    Self::deposit_event(Event::FlashLoanFailed {
                        borrower,
                        asset_id,
                        amount,
                        reason: FlashLoanFailure::CallbackFailed,
                    });

                    Err(Error::<T>::CallbackFailed.into())
                }
            }
        }

        /// Add liquidity to a flash loan pool
        ///
        /// Liquidity providers earn fees proportional to their share
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::add_liquidity())]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let provider = ensure_signed(origin)?;

            // Calculate shares (1:1 if pool is empty, proportional otherwise)
            let total_shares = TotalShares::<T>::get(asset_id);
            let total_liquidity = PoolLiquidity::<T>::get(asset_id);

            let shares = if total_shares.is_zero() || total_liquidity.is_zero() {
                amount // 1:1 for first deposit
            } else {
                // shares = amount * total_shares / total_liquidity
                amount
                    .checked_mul(&total_shares)
                    .and_then(|v| v.checked_div(&total_liquidity))
                    .ok_or(Error::<T>::ArithmeticOverflow)?
            };

            // Transfer funds from provider (in production)
            // T::Currency::transfer(&provider, &Self::pool_account(), amount, ExistenceRequirement::KeepAlive)?;

            // Update state
            PoolLiquidity::<T>::mutate(asset_id, |l| *l = l.saturating_add(amount));
            TotalShares::<T>::mutate(asset_id, |s| *s = s.saturating_add(shares));
            ProviderShares::<T>::mutate(asset_id, &provider, |s| *s = s.saturating_add(shares));

            Self::deposit_event(Event::LiquidityAdded {
                provider,
                asset_id,
                amount,
                shares,
            });

            Ok(())
        }

        /// Remove liquidity from a flash loan pool
        ///
        /// Withdraws proportional share of pool (including accrued fees)
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::remove_liquidity())]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            shares: BalanceOf<T>,
        ) -> DispatchResult {
            let provider = ensure_signed(origin)?;

            let provider_shares = ProviderShares::<T>::get(asset_id, &provider);
            ensure!(!provider_shares.is_zero(), Error::<T>::NoShares);
            ensure!(shares <= provider_shares, Error::<T>::NoShares);

            let total_shares = TotalShares::<T>::get(asset_id);
            let total_liquidity = PoolLiquidity::<T>::get(asset_id);

            // Calculate withdrawal amount
            // amount = shares * total_liquidity / total_shares
            let amount = shares
                .checked_mul(&total_liquidity)
                .and_then(|v| v.checked_div(&total_shares))
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            ensure!(!amount.is_zero(), Error::<T>::InsufficientPoolBalance);

            // Update state
            PoolLiquidity::<T>::mutate(asset_id, |l| *l = l.saturating_sub(amount));
            TotalShares::<T>::mutate(asset_id, |s| *s = s.saturating_sub(shares));
            ProviderShares::<T>::mutate(asset_id, &provider, |s| *s = s.saturating_sub(shares));

            // Transfer funds to provider (in production)
            // T::Currency::transfer(&Self::pool_account(), &provider, amount, ExistenceRequirement::AllowDeath)?;

            Self::deposit_event(Event::LiquidityRemoved {
                provider,
                asset_id,
                amount,
                shares,
            });

            Ok(())
        }

        /// Configure a flash loan pool (admin only)
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::configure_pool())]
        pub fn configure_pool(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            fee_bps: u32,
            min_loan: BalanceOf<T>,
            max_loan: BalanceOf<T>,
            active: bool,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let config = PoolConfig {
                fee_bps,
                min_loan,
                max_loan,
                active,
            };
            PoolConfigs::<T>::insert(asset_id, config);

            Self::deposit_event(Event::PoolConfigUpdated {
                asset_id,
                fee_bps,
                active,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Calculate fee for loan amount
        fn calculate_fee(amount: BalanceOf<T>, fee_bps: u32) -> Result<BalanceOf<T>, Error<T>> {
            // fee = amount * fee_bps / 10000
            let fee_bps_balance: BalanceOf<T> = fee_bps.into();
            let denominator: BalanceOf<T> = FEE_DENOMINATOR.into();

            amount
                .checked_mul(&fee_bps_balance)
                .and_then(|v| v.checked_div(&denominator))
                .ok_or(Error::<T>::ArithmeticOverflow)
        }

        /// Get current loan state (for external queries)
        pub fn loan_state() -> LoanState {
            CurrentLoanState::<T>::get()
        }

        /// Get pool info
        pub fn pool_info(asset_id: T::AssetId) -> (BalanceOf<T>, PoolConfig<BalanceOf<T>>) {
            (PoolLiquidity::<T>::get(asset_id), PoolConfigs::<T>::get(asset_id))
        }

        /// Get provider's share of pool
        pub fn provider_share(
            asset_id: T::AssetId,
            provider: &T::AccountId,
        ) -> (BalanceOf<T>, BalanceOf<T>) {
            let shares = ProviderShares::<T>::get(asset_id, provider);
            let total_shares = TotalShares::<T>::get(asset_id);
            let total_liquidity = PoolLiquidity::<T>::get(asset_id);

            if total_shares.is_zero() {
                return (shares, BalanceOf::<T>::zero());
            }

            let value = shares
                .saturating_mul(total_liquidity)
                .checked_div(&total_shares)
                .unwrap_or_else(BalanceOf::<T>::zero);

            (shares, value)
        }
    }
}

/// Weight info trait for benchmarking
pub trait WeightInfo {
    fn flash_loan() -> Weight;
    fn add_liquidity() -> Weight;
    fn remove_liquidity() -> Weight;
    fn configure_pool() -> Weight;
}

impl WeightInfo for () {
    fn flash_loan() -> Weight {
        Weight::from_parts(100_000_000, 64_000)
    }
    fn add_liquidity() -> Weight {
        Weight::from_parts(50_000_000, 32_000)
    }
    fn remove_liquidity() -> Weight {
        Weight::from_parts(50_000_000, 32_000)
    }
    fn configure_pool() -> Weight {
        Weight::from_parts(25_000_000, 16_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_noop, assert_ok, parameter_types};
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
            Flashloan: pallet,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
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
        type AccountData = pallet_balances::AccountData<u128>;
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

    parameter_types! {
        pub const ExistentialDeposit: u128 = 1;
    }

    impl pallet_balances::Config for Test {
        type Balance = u128;
        type RuntimeEvent = RuntimeEvent;
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
        type MaxLocks = ();
        type MaxReserves = ();
        type ReserveIdentifier = [u8; 8];
        type RuntimeHoldReason = ();
        type RuntimeFreezeReason = ();
        type FreezeIdentifier = ();
        type MaxFreezes = ();
    }

    parameter_types! {
        pub const MaxLoansPerBlock: u32 = 10;
    }

    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type Currency = Balances;
        type AssetId = u32;
        type MaxLoansPerBlock = MaxLoansPerBlock;
        type WeightInfo = ();
        type AdminOrigin = frame_system::EnsureRoot<u64>;
        type CallbackExecutor = ();
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }

    #[test]
    fn add_liquidity_works() {
        new_test_ext().execute_with(|| {
            let asset_id = 1u32;
            let amount = 1000u128;

            // Add liquidity
            assert_ok!(Flashloan::add_liquidity(RuntimeOrigin::signed(1), asset_id, amount));

            // Check state
            assert_eq!(PoolLiquidity::<Test>::get(asset_id), amount);
            assert_eq!(TotalShares::<Test>::get(asset_id), amount);
            assert_eq!(ProviderShares::<Test>::get(asset_id, 1), amount);
        });
    }

    #[test]
    fn remove_liquidity_works() {
        new_test_ext().execute_with(|| {
            let asset_id = 1u32;
            let amount = 1000u128;

            // Add liquidity first
            assert_ok!(Flashloan::add_liquidity(RuntimeOrigin::signed(1), asset_id, amount));

            // Remove half
            assert_ok!(Flashloan::remove_liquidity(RuntimeOrigin::signed(1), asset_id, 500));

            // Check state
            assert_eq!(PoolLiquidity::<Test>::get(asset_id), 500);
            assert_eq!(TotalShares::<Test>::get(asset_id), 500);
            assert_eq!(ProviderShares::<Test>::get(asset_id, 1), 500);
        });
    }

    #[test]
    fn configure_pool_works() {
        new_test_ext().execute_with(|| {
            let asset_id = 1u32;

            // Configure pool (root origin)
            assert_ok!(Flashloan::configure_pool(
                RuntimeOrigin::root(),
                asset_id,
                15, // 0.15% fee
                100, // min loan
                10000, // max loan
                true,
            ));

            // Check config
            let config = PoolConfigs::<Test>::get(asset_id);
            assert_eq!(config.fee_bps, 15);
            assert_eq!(config.min_loan, 100);
            assert_eq!(config.max_loan, 10000);
            assert!(config.active);
        });
    }

    #[test]
    fn flash_loan_requires_liquidity() {
        new_test_ext().execute_with(|| {
            let asset_id = 1u32;

            // Configure pool
            assert_ok!(Flashloan::configure_pool(
                RuntimeOrigin::root(),
                asset_id,
                9,
                0,
                0,
                true,
            ));

            // Try flash loan without liquidity
            assert_noop!(
                Flashloan::flash_loan(RuntimeOrigin::signed(1), asset_id, 1000, vec![]),
                Error::<Test>::InsufficientLiquidity
            );
        });
    }

    #[test]
    fn flash_loan_respects_rate_limit() {
        new_test_ext().execute_with(|| {
            let asset_id = 1u32;

            // Add liquidity
            assert_ok!(Flashloan::add_liquidity(RuntimeOrigin::signed(1), asset_id, 100_000));

            // Configure pool
            assert_ok!(Flashloan::configure_pool(
                RuntimeOrigin::root(),
                asset_id,
                9,
                0,
                0,
                true,
            ));

            // Set loans count to max
            LoansThisBlock::<Test>::put(10u32);

            // Should fail due to rate limit
            assert_noop!(
                Flashloan::flash_loan(RuntimeOrigin::signed(2), asset_id, 100, vec![]),
                Error::<Test>::RateLimitExceeded
            );
        });
    }

    #[test]
    fn fee_calculation_works() {
        new_test_ext().execute_with(|| {
            // 0.09% of 10000 = 9
            let fee = Pallet::<Test>::calculate_fee(10000u128, 9).unwrap();
            assert_eq!(fee, 9);

            // 1% of 10000 = 100
            let fee = Pallet::<Test>::calculate_fee(10000u128, 100).unwrap();
            assert_eq!(fee, 100);
        });
    }
}
