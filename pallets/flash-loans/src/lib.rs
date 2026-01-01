#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]


#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement},
    };
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    /// Flash loan status
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    pub enum FlashLoanStatus {
        Initiated,
        Executing,
        Repaid,
        Defaulted,
        Cancelled,
    }

    /// Flash loan details
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct FlashLoan<T: Config> {
        pub loan_id: u32,
        pub borrower: T::AccountId,
        pub asset_id: AssetId,
        pub principal: u128,
        pub fee: u128,
        pub total_repay: u128,
        pub status: FlashLoanStatus,
        pub block_initiated: BlockNumberFor<T>,
        pub block_deadline: BlockNumberFor<T>,
    }

    /// Execution receipt for flash loan operations
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct FlashLoanReceipt<T: Config> {
        pub loan_id: u32,
        pub borrower: T::AccountId,
        pub asset_id: AssetId,
        pub principal: u128,
        pub fee: u128,
        pub status: FlashLoanStatus,
        pub execution_time_ms: u32,
        pub success: bool,
    }

    pub type AssetId = u32;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum flash loan amount in base currency
        #[pallet::constant]
        type MaxFlashLoanAmount: Get<u128>;

        /// Default flash loan fee in basis points (e.g., 90 = 0.09%)
        #[pallet::constant]
        type DefaultFlashLoanFeeBps: Get<u32>;

        /// Maximum nesting level for flash loans
        #[pallet::constant]
        type MaxNestingLevel: Get<u32>;

        /// Blocks for which flash loan must be repaid
        #[pallet::constant]
        type FlashLoanDeadlineBlocks: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// All active flash loans
    #[pallet::storage]
    #[pallet::getter(fn flash_loan)]
    pub type FlashLoans<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        FlashLoan<T>,
        OptionQuery,
    >;

    /// Borrower to active loan ID mapping
    #[pallet::storage]
    #[pallet::getter(fn borrower_loans)]
    pub type BorrowerLoans<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Vec<u32>,
        ValueQuery,
    >;

    /// Asset liquidity pools
    #[pallet::storage]
    #[pallet::getter(fn asset_liquidity)]
    pub type AssetLiquidity<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AssetId,
        u128,
        ValueQuery,
    >;

    /// Borrower flash loan limits
    #[pallet::storage]
    #[pallet::getter(fn borrower_limit)]
    pub type BorrowerLimits<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        u128,
        ValueQuery,
    >;

    /// Flash loan counter for generating unique IDs
    #[pallet::storage]
    #[pallet::getter(fn loan_counter)]
    pub type LoanCounter<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Total statistics
    #[pallet::storage]
    #[pallet::getter(fn total_loans_processed)]
    pub type TotalLoansProcessed<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_fees_collected)]
    pub type TotalFeesCollected<T: Config> = StorageValue<_, u128, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn default_count)]
    pub type DefaultCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Flash loan fee in basis points (governance controlled)
    #[pallet::storage]
    #[pallet::getter(fn flash_loan_fee_bps)]
    pub type FlashLoanFeeBps<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Flash loan initiated
        FlashLoanInitiated {
            loan_id: u32,
            borrower: T::AccountId,
            asset_id: AssetId,
            amount: u128,
        },
        /// Flash loan repaid successfully
        FlashLoanRepaid {
            loan_id: u32,
            borrower: T::AccountId,
            asset_id: AssetId,
            fee: u128,
        },
        /// Flash loan defaulted
        FlashLoanDefaulted {
            loan_id: u32,
            borrower: T::AccountId,
            asset_id: AssetId,
        },
        /// Multi-asset flash loan initiated
        MultiFlashLoanInitiated {
            loan_id: u32,
            borrower: T::AccountId,
            asset_count: u32,
            total_value: u128,
        },
        /// Fee rate updated via governance
        FeeRateUpdated {
            old_fee_bps: u32,
            new_fee_bps: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset not found or invalid
        InvalidAsset,
        /// Insufficient liquidity for flash loan
        InsufficientLiquidity,
        /// Loan limit exceeded
        LoanLimitExceeded,
        /// Invalid loan amount
        InvalidAmount,
        /// Loan not found
        LoanNotFound,
        /// Repayment amount mismatch
        RepaymentMismatch,
        /// Insufficient balance for repayment
        InsufficientBalance,
        /// Flash loan deadline exceeded
        DeadlineExceeded,
        /// Maximum nesting level exceeded
        MaxNestingLevelExceeded,
        /// Borrower has outstanding loans
        OutstandingLoans,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // Initialize flash loan fee if not set
            if FlashLoanFeeBps::<T>::get() == 0 {
                FlashLoanFeeBps::<T>::put(T::DefaultFlashLoanFeeBps::get());
            }
            Weight::zero()
        }

        fn on_finalize(_n: BlockNumberFor<T>) {
            // Check for defaulted loans at end of block
            // Any loans not repaid by deadline are marked defaulted
            let current_block = frame_system::Pallet::<T>::block_number();
            let mut to_default = Vec::new();

            FlashLoans::<T>::iter().for_each(|(loan_id, mut loan)| {
                if loan.block_deadline <= current_block && loan.status == FlashLoanStatus::Executing {
                    loan.status = FlashLoanStatus::Defaulted;
                    FlashLoans::<T>::insert(loan_id, &loan);
                    to_default.push(loan_id);
                    
                    let default_count = DefaultCount::<T>::get();
                    DefaultCount::<T>::put(default_count.saturating_add(1));

                    Self::deposit_event(Event::FlashLoanDefaulted {
                        loan_id,
                        borrower: loan.borrower,
                        asset_id: loan.asset_id,
                    });
                }
            });
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Request a flash loan
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn request_flash_loan(
            origin: OriginFor<T>,
            asset_id: AssetId,
            amount: u128,
        ) -> DispatchResult {
            let borrower = ensure_signed(origin)?;

            // Validation
            ensure!(amount > 0, Error::<T>::InvalidAmount);
            ensure!(amount <= T::MaxFlashLoanAmount::get(), Error::<T>::InvalidAmount);

            // Check liquidity
            let liquidity = AssetLiquidity::<T>::get(asset_id);
            ensure!(liquidity >= amount, Error::<T>::InsufficientLiquidity);

            // Check borrower limit
            let borrower_limit = BorrowerLimits::<T>::get(&borrower);
            ensure!(amount <= borrower_limit, Error::<T>::LoanLimitExceeded);

            // Check active loans count
            let active_loans = BorrowerLoans::<T>::get(&borrower).len() as u32;
            ensure!(active_loans < T::MaxNestingLevel::get(), Error::<T>::MaxNestingLevelExceeded);

            // Generate loan ID
            let loan_id = Self::generate_loan_id();

            // Calculate fee
            let fee_bps = FlashLoanFeeBps::<T>::get();
            let fee = (amount.saturating_mul(fee_bps as u128)) / 100_000u128;
            let total_repay = amount.saturating_add(fee);

            // Create loan record
            let current_block = frame_system::Pallet::<T>::block_number();
            let deadline = current_block.saturating_add(T::FlashLoanDeadlineBlocks::get());

            let loan = FlashLoan {
                loan_id,
                borrower: borrower.clone(),
                asset_id,
                principal: amount,
                fee,
                total_repay,
                status: FlashLoanStatus::Executing,
                block_initiated: current_block,
                block_deadline: deadline,
            };

            // Store loan
            FlashLoans::<T>::insert(loan_id, &loan);

            // Update borrower loans list
            let mut borrower_loans = BorrowerLoans::<T>::get(&borrower);
            borrower_loans.push(loan_id);
            BorrowerLoans::<T>::insert(&borrower, borrower_loans);

            // Update liquidity (simulate transfer)
            let new_liquidity = liquidity.saturating_sub(amount);
            AssetLiquidity::<T>::insert(asset_id, new_liquidity);

            Self::deposit_event(Event::FlashLoanInitiated {
                loan_id,
                borrower,
                asset_id,
                amount,
            });

            Ok(())
        }

        /// Repay a flash loan
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn repay_flash_loan(
            origin: OriginFor<T>,
            loan_id: u32,
            asset_id: AssetId,
        ) -> DispatchResult {
            let borrower = ensure_signed(origin)?;

            // Get loan
            let mut loan = Self::flash_loan(loan_id).ok_or(Error::<T>::LoanNotFound)?;
            ensure!(loan.borrower == borrower, Error::<T>::LoanNotFound);
            ensure!(loan.asset_id == asset_id, Error::<T>::InvalidAsset);

            // Check status
            ensure!(loan.status == FlashLoanStatus::Executing, Error::<T>::LoanNotFound);

            // Check deadline
            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(current_block <= loan.block_deadline, Error::<T>::DeadlineExceeded);

            // Mark as repaid
            loan.status = FlashLoanStatus::Repaid;
            FlashLoans::<T>::insert(loan_id, &loan);

            // Update liquidity (repayment + fee)
            let current_liquidity = AssetLiquidity::<T>::get(asset_id);
            let new_liquidity = current_liquidity.saturating_add(loan.total_repay);
            AssetLiquidity::<T>::insert(asset_id, new_liquidity);

            // Update fees collected
            let total_fees = TotalFeesCollected::<T>::get();
            TotalFeesCollected::<T>::put(total_fees.saturating_add(loan.fee));

            // Update borrower loans
            let mut borrower_loans = BorrowerLoans::<T>::get(&borrower);
            borrower_loans.retain(|&id| id != loan_id);
            BorrowerLoans::<T>::insert(&borrower, borrower_loans);

            Self::deposit_event(Event::FlashLoanRepaid {
                loan_id,
                borrower,
                asset_id,
                fee: loan.fee,
            });

            Ok(())
        }

        /// Set flash loan fee (governance call)
        #[pallet::call_index(2)]
        #[pallet::weight(1_000)]
        pub fn set_flash_loan_fee(
            origin: OriginFor<T>,
            new_fee_bps: u32,
        ) -> DispatchResult {
            // In production, this would require governance origin
            let _root = ensure_root(origin)?;

            let old_fee = FlashLoanFeeBps::<T>::get();
            FlashLoanFeeBps::<T>::put(new_fee_bps);

            Self::deposit_event(Event::FeeRateUpdated {
                old_fee_bps: old_fee,
                new_fee_bps,
            });

            Ok(())
        }

        /// Set borrower flash loan limit
        #[pallet::call_index(3)]
        #[pallet::weight(1_000)]
        pub fn set_borrower_limit(
            origin: OriginFor<T>,
            borrower: T::AccountId,
            limit: u128,
        ) -> DispatchResult {
            let _root = ensure_root(origin)?;
            BorrowerLimits::<T>::insert(borrower, limit);
            Ok(())
        }

        /// Initialize asset liquidity pool
        #[pallet::call_index(4)]
        #[pallet::weight(1_000)]
        pub fn initialize_asset_liquidity(
            origin: OriginFor<T>,
            asset_id: AssetId,
            liquidity: u128,
        ) -> DispatchResult {
            let _root = ensure_root(origin)?;
            AssetLiquidity::<T>::insert(asset_id, liquidity);
            Ok(())
        }
    }

    // Internal functions
    impl<T: Config> Pallet<T> {
        fn generate_loan_id() -> u32 {
            let counter = LoanCounter::<T>::get();
            let new_counter = counter.saturating_add(1);
            LoanCounter::<T>::put(new_counter);

            let total = TotalLoansProcessed::<T>::get();
            TotalLoansProcessed::<T>::put(total.saturating_add(1));

            new_counter
        }

        pub fn get_borrower_active_loans(borrower: &T::AccountId) -> u32 {
            BorrowerLoans::<T>::get(borrower).len() as u32
        }

        pub fn get_flash_loan_fee(amount: u128) -> u128 {
            let fee_bps = FlashLoanFeeBps::<T>::get();
            (amount.saturating_mul(fee_bps as u128)) / 100_000u128
        }

        pub fn get_stats() -> (u32, u32, u128, u32) {
            (
                TotalLoansProcessed::<T>::get(),
                LoanCounter::<T>::get(),
                TotalFeesCollected::<T>::get(),
                DefaultCount::<T>::get(),
            )
        }
    }
}
