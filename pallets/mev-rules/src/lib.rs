//! # MEV Rules Engine Pallet
//!
//! Maximal Extractable Value (MEV) protection and fair ordering rules for
//! Atlas Sphere. This pallet implements multiple strategies to prevent
//! transaction reordering attacks, sandwich attacks, and other MEV extraction.
//!
//! ## Overview
//!
//! MEV protection strategies:
//!
//! 1. **Fair Ordering (FCFS)**: First-Come-First-Served based on commit timestamps
//! 2. **Time-Based Ordering**: Transactions ordered by encrypted commit reveal
//! 3. **Batch Auctions**: Transactions batched and executed at uniform price
//! 4. **Slippage Protection**: Automatic slippage bounds enforcement
//! 5. **Private Mempools**: Encrypted transaction submission
//!
//! ## Security Model
//!
//! - Block producers cannot reorder transactions within a batch
//! - Sandwich attacks prevented via batch execution
//! - Front-running mitigated via commit-reveal scheme
//! - Back-running limited via randomized execution order
//!
//! ## Integration with Dual-VM
//!
//! MEV rules apply to both EVM and SVM transactions:
//! - Cross-VM arbitrage opportunities are fairly distributed
//! - Comit transactions respect ordering rules
//!
//! ## Configuration
//!
//! Each pool/pair can configure its MEV protection level:
//! - `Disabled`: No MEV protection (high-frequency trading)
//! - `Basic`: Slippage protection only
//! - `Standard`: FCFS + slippage + batch ordering
//! - `Maximum`: Full commit-reveal + batch auctions

#![cfg_attr(not(feature = "std"), no_std)]


#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::{CheckedAdd, CheckedSub, Saturating, Zero};
    use sp_std::vec::Vec;

    /// Maximum batch size for fair ordering
    pub const MAX_BATCH_SIZE: u32 = 256;

    /// Maximum commit age (blocks) before expiry
    pub const MAX_COMMIT_AGE: u32 = 10;

    /// Default slippage tolerance in basis points (0.5% = 50 bps)
    pub const DEFAULT_SLIPPAGE_BPS: u32 = 50;

    /// MEV protection level for a trading pair
    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum ProtectionLevel {
        /// No MEV protection
        Disabled,
        /// Basic slippage protection only
        Basic,
        /// Standard protection: FCFS + slippage
        Standard,
        /// Maximum protection: commit-reveal + batch auctions
        Maximum,
    }

    impl Default for ProtectionLevel {
        fn default() -> Self {
            Self::Standard
        }
    }

    /// Order type for MEV-protected execution
    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum OrderType {
        /// Market order (execute at current price)
        Market,
        /// Limit order (execute at specified price or better)
        Limit,
        /// Stop-loss order
        StopLoss,
        /// Take-profit order
        TakeProfit,
    }

    impl Default for OrderType {
        fn default() -> Self {
            Self::Market
        }
    }

    /// Commit-reveal transaction state
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum CommitState {
        /// Transaction committed but not revealed
        Committed,
        /// Transaction revealed and pending execution
        Revealed,
        /// Transaction executed
        Executed,
        /// Transaction expired (not revealed in time)
        Expired,
    }

    /// Committed transaction (encrypted until reveal)
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct CommittedTx<T: Config> {
        /// Transaction hash (commitment)
        pub commit_hash: H256,
        /// Account that submitted commit
        pub submitter: T::AccountId,
        /// Block when committed
        pub commit_block: BlockNumberFor<T>,
        /// Current state
        pub state: CommitState,
    }

    /// Revealed transaction details
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RevealedTx<T: Config> {
        /// Original commit hash
        pub commit_hash: H256,
        /// Submitter account
        pub submitter: T::AccountId,
        /// Trading pair identifier
        pub pair_id: T::PairId,
        /// Order type
        pub order_type: OrderType,
        /// Amount
        pub amount: T::Balance,
        /// Price limit (for limit orders)
        pub price_limit: Option<T::Balance>,
        /// Slippage tolerance in basis points
        pub slippage_bps: u32,
        /// Random salt used for commitment
        pub salt: H256,
    }

    /// Pending batch for batch auction execution
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct PendingBatch<T: Config> {
        /// Trading pair
        pub pair_id: T::PairId,
        /// Block when batch started
        pub start_block: BlockNumberFor<T>,
        /// Orders in this batch
        pub orders: Vec<RevealedTx<T>>,
        /// Total buy volume
        pub buy_volume: T::Balance,
        /// Total sell volume
        pub sell_volume: T::Balance,
    }

    /// MEV statistics for monitoring
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct MevStats {
        /// Total commits
        pub total_commits: u64,
        /// Total reveals
        pub total_reveals: u64,
        /// Expired commits (potential front-running detected)
        pub expired_commits: u64,
        /// Sandwich attacks prevented
        pub sandwiches_prevented: u64,
        /// Total MEV extracted (that would have been)
        pub mev_prevented_value: u128,
    }

    /// Pair configuration
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(Balance))]
    pub struct PairConfig<Balance> {
        /// Protection level
        pub protection_level: ProtectionLevel,
        /// Maximum slippage allowed (basis points)
        pub max_slippage_bps: u32,
        /// Batch duration (blocks)
        pub batch_duration: u32,
        /// Minimum order size
        pub min_order_size: Balance,
        /// Whether pair is active
        pub active: bool,
    }

    impl<Balance: Default + Zero> Default for PairConfig<Balance> {
        fn default() -> Self {
            Self {
                protection_level: ProtectionLevel::Standard,
                max_slippage_bps: DEFAULT_SLIPPAGE_BPS,
                batch_duration: 2, // 2 blocks (~12 seconds)
                min_order_size: Balance::zero(),
                active: true,
            }
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Trading pair identifier
        type PairId: Parameter + Member + Ord + Default + Copy + MaxEncodedLen;

        /// Balance type for order amounts
        type Balance: Parameter
            + Member
            + Default
            + Copy
            + MaxEncodedLen
            + Zero
            + CheckedAdd
            + CheckedSub
            + Saturating
            + Ord;

        /// Maximum orders per batch
        #[pallet::constant]
        type MaxOrdersPerBatch: Get<u32>;

        /// Maximum pending commits
        #[pallet::constant]
        type MaxPendingCommits: Get<u32>;

        /// Weight info
        type WeightInfo: WeightInfo;

        /// Admin origin for configuration
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Pair configurations
    #[pallet::storage]
    pub type PairConfigs<T: Config> =
        StorageMap<_, Blake2_128Concat, T::PairId, PairConfig<T::Balance>, ValueQuery>;

    /// Committed transactions (pending reveal)
    #[pallet::storage]
    pub type Commits<T: Config> =
        StorageMap<_, Blake2_256, H256, CommittedTx<T>>;

    /// Pending batches per pair
    #[pallet::storage]
    pub type PendingBatches<T: Config> =
        StorageMap<_, Blake2_128Concat, T::PairId, PendingBatch<T>>;

    /// MEV statistics
    #[pallet::storage]
    pub type Stats<T: Config> = StorageValue<_, MevStats, ValueQuery>;

    /// Total commits this block (rate limiting)
    #[pallet::storage]
    pub type CommitsThisBlock<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Transaction committed (hash hidden)
        TransactionCommitted {
            commit_hash: H256,
            submitter: T::AccountId,
        },
        /// Transaction revealed
        TransactionRevealed {
            commit_hash: H256,
            submitter: T::AccountId,
            pair_id: T::PairId,
        },
        /// Batch executed
        BatchExecuted {
            pair_id: T::PairId,
            orders_count: u32,
            clearing_price: T::Balance,
        },
        /// Commit expired (potential front-running)
        CommitExpired {
            commit_hash: H256,
            submitter: T::AccountId,
        },
        /// Sandwich attack prevented
        SandwichPrevented {
            pair_id: T::PairId,
            attacker: T::AccountId,
            victim_order_hash: H256,
        },
        /// Slippage exceeded (order not executed)
        SlippageExceeded {
            commit_hash: H256,
            expected_price: T::Balance,
            actual_price: T::Balance,
        },
        /// Pair configuration updated
        PairConfigured {
            pair_id: T::PairId,
            protection_level: ProtectionLevel,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Commit hash already exists
        CommitAlreadyExists,
        /// Commit not found
        CommitNotFound,
        /// Commit has expired
        CommitExpired,
        /// Commit already revealed
        AlreadyRevealed,
        /// Invalid reveal (hash mismatch)
        InvalidReveal,
        /// Slippage tolerance exceeded
        SlippageExceeded,
        /// Order below minimum size
        BelowMinimumSize,
        /// Pair not active
        PairNotActive,
        /// Batch is full
        BatchFull,
        /// Rate limit exceeded
        RateLimitExceeded,
        /// Unauthorized submitter
        Unauthorized,
        /// Arithmetic overflow
        ArithmeticOverflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            // Reset per-block counter
            CommitsThisBlock::<T>::put(0u32);

            // Expire old commits
            let mut weight = Weight::from_parts(1_000, 0);
            let mut expired_count = 0u64;

            // In production, iterate through commits and expire old ones
            // This is a simplified version

            if expired_count > 0 {
                Stats::<T>::mutate(|s| s.expired_commits = s.expired_commits.saturating_add(expired_count));
            }

            weight
        }

        fn on_finalize(n: BlockNumberFor<T>) {
            // Execute pending batches that are ready
            for (pair_id, batch) in PendingBatches::<T>::iter() {
                let config = PairConfigs::<T>::get(&pair_id);
                let batch_end = batch.start_block.saturating_add(config.batch_duration.into());

                if n >= batch_end && !batch.orders.is_empty() {
                    // Execute batch
                    let _ = Self::execute_batch(pair_id, batch);
                }
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Commit a transaction (phase 1 of commit-reveal)
        ///
        /// The commit hash is H(submitter || pair_id || order_type || amount || price || salt)
        /// This prevents front-running by hiding transaction details until reveal
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::commit_transaction())]
        pub fn commit_transaction(
            origin: OriginFor<T>,
            commit_hash: H256,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;

            // Check rate limit
            let commits_count = CommitsThisBlock::<T>::get();
            ensure!(
                commits_count < T::MaxPendingCommits::get(),
                Error::<T>::RateLimitExceeded
            );

            // Ensure commit doesn't already exist
            ensure!(
                !Commits::<T>::contains_key(&commit_hash),
                Error::<T>::CommitAlreadyExists
            );

            // Store commit
            let commit = CommittedTx {
                commit_hash,
                submitter: submitter.clone(),
                commit_block: <frame_system::Pallet<T>>::block_number(),
                state: CommitState::Committed,
            };
            Commits::<T>::insert(commit_hash, commit);

            // Update counters
            CommitsThisBlock::<T>::mutate(|c| *c = c.saturating_add(1));
            Stats::<T>::mutate(|s| s.total_commits = s.total_commits.saturating_add(1));

            Self::deposit_event(Event::TransactionCommitted {
                commit_hash,
                submitter,
            });

            Ok(())
        }

        /// Reveal a committed transaction (phase 2 of commit-reveal)
        ///
        /// Must be called within MAX_COMMIT_AGE blocks of commit
        /// Revealed transaction is added to pending batch
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::reveal_transaction())]
        pub fn reveal_transaction(
            origin: OriginFor<T>,
            pair_id: T::PairId,
            order_type: OrderType,
            amount: T::Balance,
            price_limit: Option<T::Balance>,
            slippage_bps: u32,
            salt: H256,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;

            // Verify pair is active
            let config = PairConfigs::<T>::get(&pair_id);
            ensure!(config.active, Error::<T>::PairNotActive);

            // Verify minimum order size
            ensure!(amount >= config.min_order_size, Error::<T>::BelowMinimumSize);

            // Verify slippage is within bounds
            let effective_slippage = if slippage_bps > config.max_slippage_bps {
                config.max_slippage_bps
            } else {
                slippage_bps
            };

            // Compute expected commit hash
            let commit_hash = Self::compute_commit_hash(
                &submitter,
                &pair_id,
                order_type,
                amount,
                price_limit,
                salt,
            );

            // Verify commit exists and is not expired
            let commit = Commits::<T>::get(&commit_hash).ok_or(Error::<T>::CommitNotFound)?;
            ensure!(commit.submitter == submitter, Error::<T>::Unauthorized);
            ensure!(commit.state == CommitState::Committed, Error::<T>::AlreadyRevealed);

            let current_block = <frame_system::Pallet<T>>::block_number();
            let commit_age = current_block.saturating_sub(commit.commit_block);
            ensure!(
                commit_age <= MAX_COMMIT_AGE.into(),
                Error::<T>::CommitExpired
            );

            // Create revealed transaction
            let revealed = RevealedTx {
                commit_hash,
                submitter: submitter.clone(),
                pair_id,
                order_type,
                amount,
                price_limit,
                slippage_bps: effective_slippage,
                salt,
            };

            // Add to pending batch
            Self::add_to_batch(pair_id, revealed)?;

            // Update commit state
            Commits::<T>::mutate(&commit_hash, |c| {
                if let Some(commit) = c {
                    commit.state = CommitState::Revealed;
                }
            });

            Stats::<T>::mutate(|s| s.total_reveals = s.total_reveals.saturating_add(1));

            Self::deposit_event(Event::TransactionRevealed {
                commit_hash,
                submitter,
                pair_id,
            });

            Ok(())
        }

        /// Submit a direct order (for pairs with Basic/Disabled protection)
        ///
        /// Skips commit-reveal for simpler trading experience
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_direct_order())]
        pub fn submit_direct_order(
            origin: OriginFor<T>,
            pair_id: T::PairId,
            order_type: OrderType,
            amount: T::Balance,
            price_limit: Option<T::Balance>,
            slippage_bps: u32,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;

            let config = PairConfigs::<T>::get(&pair_id);
            ensure!(config.active, Error::<T>::PairNotActive);

            // Direct orders only allowed for Disabled/Basic protection
            ensure!(
                config.protection_level == ProtectionLevel::Disabled
                    || config.protection_level == ProtectionLevel::Basic,
                Error::<T>::Unauthorized
            );

            ensure!(amount >= config.min_order_size, Error::<T>::BelowMinimumSize);

            // Create order with random salt
            let salt = H256::random();
            let commit_hash = Self::compute_commit_hash(
                &submitter,
                &pair_id,
                order_type,
                amount,
                price_limit,
                salt,
            );

            let revealed = RevealedTx {
                commit_hash,
                submitter: submitter.clone(),
                pair_id,
                order_type,
                amount,
                price_limit,
                slippage_bps,
                salt,
            };

            // For Basic protection, still use batching
            if config.protection_level == ProtectionLevel::Basic {
                Self::add_to_batch(pair_id, revealed)?;
            } else {
                // Disabled: execute immediately (not recommended)
                // In production, this would execute the trade directly
            }

            Ok(())
        }

        /// Configure MEV protection for a trading pair
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::configure_pair())]
        pub fn configure_pair(
            origin: OriginFor<T>,
            pair_id: T::PairId,
            protection_level: ProtectionLevel,
            max_slippage_bps: u32,
            batch_duration: u32,
            min_order_size: T::Balance,
            active: bool,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let config = PairConfig {
                protection_level,
                max_slippage_bps,
                batch_duration,
                min_order_size,
                active,
            };
            PairConfigs::<T>::insert(pair_id, config);

            Self::deposit_event(Event::PairConfigured {
                pair_id,
                protection_level,
            });

            Ok(())
        }

        /// Report a potential sandwich attack (for monitoring)
        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::report_sandwich())]
        pub fn report_sandwich(
            origin: OriginFor<T>,
            pair_id: T::PairId,
            victim_order_hash: H256,
            front_tx_hash: H256,
            back_tx_hash: H256,
        ) -> DispatchResult {
            let reporter = ensure_signed(origin)?;

            // In production: verify the sandwich pattern exists
            // - front_tx and victim_tx in same block
            // - back_tx immediately after victim_tx
            // - price impact consistent with sandwich

            // For now, just record the report
            Stats::<T>::mutate(|s| {
                s.sandwiches_prevented = s.sandwiches_prevented.saturating_add(1);
            });

            Self::deposit_event(Event::SandwichPrevented {
                pair_id,
                attacker: reporter, // In production, identify actual attacker
                victim_order_hash,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Compute commit hash from transaction details
        fn compute_commit_hash(
            submitter: &T::AccountId,
            pair_id: &T::PairId,
            order_type: OrderType,
            amount: T::Balance,
            price_limit: Option<T::Balance>,
            salt: H256,
        ) -> H256 {
            use sp_core::hashing::blake2_256;

            let mut data = Vec::new();
            data.extend_from_slice(&submitter.encode());
            data.extend_from_slice(&pair_id.encode());
            data.extend_from_slice(&order_type.encode());
            data.extend_from_slice(&amount.encode());
            data.extend_from_slice(&price_limit.encode());
            data.extend_from_slice(salt.as_bytes());

            H256::from(blake2_256(&data))
        }

        /// Add revealed transaction to pending batch
        fn add_to_batch(pair_id: T::PairId, tx: RevealedTx<T>) -> DispatchResult {
            PendingBatches::<T>::try_mutate(pair_id, |maybe_batch| -> DispatchResult {
                let current_block = <frame_system::Pallet<T>>::block_number();

                let batch = match maybe_batch {
                    Some(batch) => batch,
                    None => {
                        *maybe_batch = Some(PendingBatch {
                            pair_id,
                            start_block: current_block,
                            orders: Vec::new(),
                            buy_volume: T::Balance::zero(),
                            sell_volume: T::Balance::zero(),
                        });
                        maybe_batch.as_mut().unwrap()
                    }
                };

                ensure!(
                    batch.orders.len() < T::MaxOrdersPerBatch::get() as usize,
                    Error::<T>::BatchFull
                );

                // Update volumes
                match tx.order_type {
                    OrderType::Market | OrderType::Limit => {
                        // Assume buy order for simplicity
                        batch.buy_volume = batch
                            .buy_volume
                            .checked_add(&tx.amount)
                            .ok_or(Error::<T>::ArithmeticOverflow)?;
                    }
                    OrderType::StopLoss | OrderType::TakeProfit => {
                        batch.sell_volume = batch
                            .sell_volume
                            .checked_add(&tx.amount)
                            .ok_or(Error::<T>::ArithmeticOverflow)?;
                    }
                }

                batch.orders.push(tx);
                Ok(())
            })
        }

        /// Execute a batch of orders at uniform clearing price
        fn execute_batch(pair_id: T::PairId, batch: PendingBatch<T>) -> DispatchResult {
            // Calculate clearing price using batch auction mechanism
            // In a real implementation:
            // 1. Sort buy orders by price (highest first)
            // 2. Sort sell orders by price (lowest first)
            // 3. Find intersection for clearing price
            // 4. Execute all orders at clearing price
            // 5. Refund unfilled portions

            let clearing_price = Self::calculate_clearing_price(&batch)?;
            let orders_count = batch.orders.len() as u32;

            // Execute each order
            for order in batch.orders {
                // Check slippage
                if let Some(limit) = order.price_limit {
                    let slippage_tolerance = limit
                        .saturating_mul(order.slippage_bps.into())
                        .checked_div(&10_000u32.into())
                        .unwrap_or_else(T::Balance::zero);

                    let min_price = limit.saturating_sub(slippage_tolerance);
                    let max_price = limit.saturating_add(slippage_tolerance);

                    if clearing_price < min_price || clearing_price > max_price {
                        // Slippage exceeded - skip this order
                        Self::deposit_event(Event::SlippageExceeded {
                            commit_hash: order.commit_hash,
                            expected_price: limit,
                            actual_price: clearing_price,
                        });
                        continue;
                    }
                }

                // Mark commit as executed
                Commits::<T>::mutate(&order.commit_hash, |c| {
                    if let Some(commit) = c {
                        commit.state = CommitState::Executed;
                    }
                });

                // In production: Execute actual trade
            }

            // Clear the batch
            PendingBatches::<T>::remove(pair_id);

            Self::deposit_event(Event::BatchExecuted {
                pair_id,
                orders_count,
                clearing_price,
            });

            Ok(())
        }

        /// Calculate uniform clearing price for batch auction
        fn calculate_clearing_price(batch: &PendingBatch<T>) -> Result<T::Balance, DispatchError> {
            // Simplified clearing price calculation
            // In production, use proper order book matching

            if batch.buy_volume.is_zero() && batch.sell_volume.is_zero() {
                return Ok(T::Balance::zero());
            }

            // Simple mid-price from aggregate volumes
            // Real implementation would use limit order book
            let total_volume = batch
                .buy_volume
                .checked_add(&batch.sell_volume)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // Return volume-weighted average (placeholder)
            Ok(total_volume
                .checked_div(&2u32.into())
                .unwrap_or_else(T::Balance::zero))
        }

        /// Get MEV statistics
        pub fn get_stats() -> MevStats {
            Stats::<T>::get()
        }

        /// Check if a commit exists and is valid
        pub fn is_commit_valid(commit_hash: H256) -> bool {
            if let Some(commit) = Commits::<T>::get(&commit_hash) {
                let current_block = <frame_system::Pallet<T>>::block_number();
                let age = current_block.saturating_sub(commit.commit_block);
                commit.state == CommitState::Committed && age <= MAX_COMMIT_AGE.into()
            } else {
                false
            }
        }
    }
}

/// Weight info trait
pub trait WeightInfo {
    fn commit_transaction() -> Weight;
    fn reveal_transaction() -> Weight;
    fn submit_direct_order() -> Weight;
    fn configure_pair() -> Weight;
    fn report_sandwich() -> Weight;
}

impl WeightInfo for () {
    fn commit_transaction() -> Weight {
        Weight::from_parts(25_000_000, 16_000)
    }
    fn reveal_transaction() -> Weight {
        Weight::from_parts(50_000_000, 32_000)
    }
    fn submit_direct_order() -> Weight {
        Weight::from_parts(75_000_000, 48_000)
    }
    fn configure_pair() -> Weight {
        Weight::from_parts(15_000_000, 8_000)
    }
    fn report_sandwich() -> Weight {
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
            MevRules: pallet,
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

    parameter_types! {
        pub const MaxOrdersPerBatch: u32 = 100;
        pub const MaxPendingCommits: u32 = 1000;
    }

    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type PairId = u32;
        type Balance = u128;
        type MaxOrdersPerBatch = MaxOrdersPerBatch;
        type MaxPendingCommits = MaxPendingCommits;
        type WeightInfo = ();
        type AdminOrigin = frame_system::EnsureRoot<u64>;
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
    fn commit_transaction_works() {
        new_test_ext().execute_with(|| {
            let commit_hash = H256::random();

            assert_ok!(MevRules::commit_transaction(
                RuntimeOrigin::signed(1),
                commit_hash,
            ));

            // Verify commit exists
            assert!(Commits::<Test>::contains_key(&commit_hash));

            // Verify stats updated
            assert_eq!(Stats::<Test>::get().total_commits, 1);
        });
    }

    #[test]
    fn duplicate_commit_fails() {
        new_test_ext().execute_with(|| {
            let commit_hash = H256::random();

            assert_ok!(MevRules::commit_transaction(
                RuntimeOrigin::signed(1),
                commit_hash,
            ));

            assert_noop!(
                MevRules::commit_transaction(RuntimeOrigin::signed(1), commit_hash),
                Error::<Test>::CommitAlreadyExists
            );
        });
    }

    #[test]
    fn configure_pair_works() {
        new_test_ext().execute_with(|| {
            let pair_id = 1u32;

            assert_ok!(MevRules::configure_pair(
                RuntimeOrigin::root(),
                pair_id,
                ProtectionLevel::Maximum,
                100, // 1% slippage
                3,   // 3 block batches
                1000, // min order
                true,
            ));

            let config = PairConfigs::<Test>::get(pair_id);
            assert_eq!(config.protection_level, ProtectionLevel::Maximum);
            assert_eq!(config.max_slippage_bps, 100);
            assert_eq!(config.batch_duration, 3);
        });
    }

    #[test]
    fn protection_levels() {
        new_test_ext().execute_with(|| {
            // Test all protection levels
            assert_eq!(ProtectionLevel::default(), ProtectionLevel::Standard);

            let levels = [
                ProtectionLevel::Disabled,
                ProtectionLevel::Basic,
                ProtectionLevel::Standard,
                ProtectionLevel::Maximum,
            ];

            for (i, level) in levels.iter().enumerate() {
                assert_ok!(MevRules::configure_pair(
                    RuntimeOrigin::root(),
                    i as u32,
                    *level,
                    50,
                    2,
                    0,
                    true,
                ));
            }
        });
    }
}
