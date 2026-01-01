//! # X3Chain Treasury Pallet
//!
//! A comprehensive treasury management system supporting:
//! - Multi-sig spending approvals
//! - Spending tracks with different approval thresholds
//! - Recurring payments with configurable intervals
//! - Yield strategies that can be delegated to AI agents
//! - Emergency pause system
//!
//! ## Overview
//!
//! The treasury pallet manages protocol funds with multiple security layers.
//! Spending proposals require multi-sig approval based on the spending track.
//! AI agents can be delegated to execute yield strategies within defined limits.
//!
//! ## Spending Tracks
//!
//! - Small: Low threshold, fast approval (e.g., bug bounties)
//! - Medium: Standard threshold (e.g., grants)
//! - Large: High threshold, longer voting (e.g., major initiatives)
//! - Critical: Maximum security, council-only (e.g., protocol upgrades)

#![cfg_attr(not(feature = "std"), no_std)]


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod types;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, ReservableCurrency},
        Blake2_128Concat, PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AccountIdConversion, Saturating, Zero},
        Percent,
    };
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type for treasury operations.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Origin that can approve small spending proposals.
        type SmallSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can approve medium spending proposals.
        type MediumSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can approve large spending proposals.
        type LargeSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can approve critical spending (council).
        type CriticalSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can pause treasury operations.
        type PauseOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can configure yield strategies.
        type YieldConfigOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The treasury's pallet ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Maximum signers for multi-sig.
        #[pallet::constant]
        type MaxSigners: Get<u32>;

        /// Maximum pending proposals.
        #[pallet::constant]
        type MaxProposals: Get<u32>;

        /// Maximum recurring payments.
        #[pallet::constant]
        type MaxRecurringPayments: Get<u32>;

        /// Maximum yield strategies.
        #[pallet::constant]
        type MaxYieldStrategies: Get<u32>;

        /// Small spend threshold.
        #[pallet::constant]
        type SmallSpendLimit: Get<BalanceOf<Self>>;

        /// Medium spend threshold.
        #[pallet::constant]
        type MediumSpendLimit: Get<BalanceOf<Self>>;

        /// Large spend threshold.
        #[pallet::constant]
        type LargeSpendLimit: Get<BalanceOf<Self>>;

        /// Proposal bond percentage.
        #[pallet::constant]
        type ProposalBond: Get<Percent>;

        /// Minimum proposal bond.
        #[pallet::constant]
        type ProposalBondMinimum: Get<BalanceOf<Self>>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    // ========================================================================
    // Storage Items
    // ========================================================================

    /// Counter for spending proposal IDs.
    #[pallet::storage]
    #[pallet::getter(fn proposal_count)]
    pub type ProposalCount<T> = StorageValue<_, u32, ValueQuery>;

    /// All spending proposals.
    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        SpendingProposal<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Multi-sig signers for spending.
    #[pallet::storage]
    #[pallet::getter(fn signers)]
    pub type Signers<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxSigners>, ValueQuery>;

    /// Approvals for each proposal.
    #[pallet::storage]
    #[pallet::getter(fn approvals)]
    pub type Approvals<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, BoundedVec<T::AccountId, T::MaxSigners>, ValueQuery>;

    /// Recurring payment schedules.
    #[pallet::storage]
    #[pallet::getter(fn recurring_payments)]
    pub type RecurringPayments<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        RecurringPayment<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Counter for recurring payment IDs.
    #[pallet::storage]
    #[pallet::getter(fn recurring_payment_count)]
    pub type RecurringPaymentCount<T> = StorageValue<_, u32, ValueQuery>;

    /// Active yield strategies.
    #[pallet::storage]
    #[pallet::getter(fn yield_strategies)]
    pub type YieldStrategies<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        YieldStrategy<T::AccountId, BalanceOf<T>>,
        OptionQuery,
    >;

    /// Counter for yield strategy IDs.
    #[pallet::storage]
    #[pallet::getter(fn yield_strategy_count)]
    pub type YieldStrategyCount<T> = StorageValue<_, u32, ValueQuery>;

    /// Whether treasury is paused.
    #[pallet::storage]
    #[pallet::getter(fn is_paused)]
    pub type IsPaused<T> = StorageValue<_, bool, ValueQuery>;

    /// Emergency pause info.
    #[pallet::storage]
    #[pallet::getter(fn pause_info)]
    pub type PauseInfo<T: Config> =
        StorageValue<_, EmergencyPause<T::AccountId, BlockNumberFor<T>>, OptionQuery>;

    /// Treasury statistics.
    #[pallet::storage]
    #[pallet::getter(fn stats)]
    pub type Stats<T: Config> = StorageValue<_, TreasuryStats<BalanceOf<T>>, ValueQuery>;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A spending proposal was submitted.
        ProposalSubmitted {
            proposal_id: u32,
            proposer: T::AccountId,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
            track: SpendingTrack,
        },
        /// A proposal was approved by a signer.
        ProposalApproved {
            proposal_id: u32,
            signer: T::AccountId,
            approvals: u32,
            threshold: u32,
        },
        /// A proposal was executed.
        ProposalExecuted {
            proposal_id: u32,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// A proposal was rejected.
        ProposalRejected { proposal_id: u32 },
        /// A recurring payment was created.
        RecurringPaymentCreated {
            payment_id: u32,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
            interval: BlockNumberFor<T>,
        },
        /// A recurring payment was executed.
        RecurringPaymentExecuted {
            payment_id: u32,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// A recurring payment was cancelled.
        RecurringPaymentCancelled { payment_id: u32 },
        /// A yield strategy was registered.
        YieldStrategyRegistered {
            strategy_id: u32,
            agent: T::AccountId,
            max_allocation: BalanceOf<T>,
        },
        /// A yield strategy was executed.
        YieldStrategyExecuted {
            strategy_id: u32,
            amount: BalanceOf<T>,
            profit: BalanceOf<T>,
        },
        /// A yield strategy was deactivated.
        YieldStrategyDeactivated { strategy_id: u32 },
        /// Treasury was paused.
        TreasuryPaused {
            by: T::AccountId,
            reason: BoundedVec<u8, ConstU32<256>>,
        },
        /// Treasury was unpaused.
        TreasuryUnpaused { by: T::AccountId },
        /// Signers were updated.
        SignersUpdated { signers: Vec<T::AccountId> },
        /// Funds deposited to treasury.
        Deposited {
            from: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Proposal not found.
        ProposalNotFound,
        /// Too many proposals.
        TooManyProposals,
        /// Insufficient treasury balance.
        InsufficientBalance,
        /// Not a valid signer.
        NotSigner,
        /// Already approved this proposal.
        AlreadyApproved,
        /// Treasury is paused.
        TreasuryPaused,
        /// Treasury is not paused.
        TreasuryNotPaused,
        /// Invalid spending track.
        InvalidTrack,
        /// Amount exceeds track limit.
        AmountExceedsLimit,
        /// Recurring payment not found.
        RecurringPaymentNotFound,
        /// Too many recurring payments.
        TooManyRecurringPayments,
        /// Payment not due yet.
        PaymentNotDue,
        /// Yield strategy not found.
        YieldStrategyNotFound,
        /// Too many yield strategies.
        TooManyYieldStrategies,
        /// Strategy allocation exceeded.
        AllocationExceeded,
        /// Invalid yield return.
        InvalidYieldReturn,
        /// Not the strategy agent.
        NotStrategyAgent,
        /// Strategy is not active.
        StrategyNotActive,
        /// Too many signers.
        TooManySigners,
        /// Threshold not reached.
        ThresholdNotReached,
        /// Zero amount not allowed.
        ZeroAmount,
        /// Proposal already executed.
        AlreadyExecuted,
        /// Arithmetic overflow.
        ArithmeticOverflow,
    }

    // ========================================================================
    // Genesis Config
    // ========================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub initial_signers: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let signers: BoundedVec<T::AccountId, T::MaxSigners> =
                self.initial_signers.clone().try_into().unwrap_or_else(|_| {
                    // This should never happen in practice as initial_signers is validated
                    // during chain spec creation, but we provide a safe fallback
                    BoundedVec::truncate_from(vec![])
                });
            Signers::<T>::put(signers);
        }
    }

    // ========================================================================
    // Hooks
    // ========================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let mut weight = Weight::zero();

            if !IsPaused::<T>::get() {
                // Process recurring payments
                weight = weight.saturating_add(Self::process_recurring_payments(n));
            }

            weight
        }
    }

    // ========================================================================
    // Extrinsics
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a spending proposal.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_proposal())]
        pub fn submit_proposal(
            origin: OriginFor<T>,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
            description: BoundedVec<u8, ConstU32<1024>>,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);

            // Determine spending track
            let track = Self::determine_track(amount);

            // Reserve proposal bond
            let bond = Self::calculate_bond(amount);
            T::Currency::reserve(&proposer, bond)?;

            let proposal_id = ProposalCount::<T>::get();
            ensure!(
                proposal_id < T::MaxProposals::get(),
                Error::<T>::TooManyProposals
            );

            let current_block = frame_system::Pallet::<T>::block_number();

            let proposal = SpendingProposal {
                id: proposal_id,
                proposer: proposer.clone(),
                beneficiary: beneficiary.clone(),
                amount,
                bond,
                description,
                track,
                status: ProposalStatus::Pending,
                submitted_at: current_block,
                executed_at: None,
            };

            Proposals::<T>::insert(proposal_id, proposal);
            ProposalCount::<T>::put(proposal_id.saturating_add(1));

            Self::deposit_event(Event::ProposalSubmitted {
                proposal_id,
                proposer,
                beneficiary,
                amount,
                track,
            });

            Ok(())
        }

        /// Approve a spending proposal (multi-sig).
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::approve_proposal())]
        pub fn approve_proposal(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);

            // Verify signer
            let signers = Signers::<T>::get();
            ensure!(signers.contains(&signer), Error::<T>::NotSigner);

            let proposal = Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(
                proposal.status == ProposalStatus::Pending,
                Error::<T>::AlreadyExecuted
            );

            // Check not already approved
            let mut approvals = Approvals::<T>::get(proposal_id);
            ensure!(!approvals.contains(&signer), Error::<T>::AlreadyApproved);

            // Add approval
            approvals
                .try_push(signer.clone())
                .map_err(|_| Error::<T>::TooManySigners)?;

            let approval_count = approvals.len() as u32;
            let threshold = Self::get_threshold(&proposal.track);

            Approvals::<T>::insert(proposal_id, approvals);

            Self::deposit_event(Event::ProposalApproved {
                proposal_id,
                signer,
                approvals: approval_count,
                threshold,
            });

            // Auto-execute if threshold reached
            if approval_count >= threshold {
                Self::do_execute_proposal(proposal_id)?;
            }

            Ok(())
        }

        /// Execute an approved proposal.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::execute_proposal())]
        pub fn execute_proposal(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
            ensure_signed(origin)?;
            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);

            Self::do_execute_proposal(proposal_id)
        }

        /// Reject a proposal.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::reject_proposal())]
        pub fn reject_proposal(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
            // Requires appropriate origin based on track
            let proposal = Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            match proposal.track {
                SpendingTrack::Small => {
                    T::SmallSpendOrigin::ensure_origin(origin)?;
                }
                SpendingTrack::Medium => {
                    T::MediumSpendOrigin::ensure_origin(origin)?;
                }
                SpendingTrack::Large => {
                    T::LargeSpendOrigin::ensure_origin(origin)?;
                }
                SpendingTrack::Critical => {
                    T::CriticalSpendOrigin::ensure_origin(origin)?;
                }
            }

            // Slash bond (ignore imbalance - slashed funds go to pot)
            let _ = T::Currency::slash_reserved(&proposal.proposer, proposal.bond);

            // Remove proposal
            Proposals::<T>::remove(proposal_id);
            Approvals::<T>::remove(proposal_id);

            Self::deposit_event(Event::ProposalRejected { proposal_id });

            Ok(())
        }

        /// Create a recurring payment.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::create_recurring_payment())]
        pub fn create_recurring_payment(
            origin: OriginFor<T>,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
            interval: BlockNumberFor<T>,
            total_payments: Option<u32>,
            description: BoundedVec<u8, ConstU32<256>>,
        ) -> DispatchResult {
            T::MediumSpendOrigin::ensure_origin(origin)?;
            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);

            let payment_id = RecurringPaymentCount::<T>::get();
            ensure!(
                payment_id < T::MaxRecurringPayments::get(),
                Error::<T>::TooManyRecurringPayments
            );

            let current_block = frame_system::Pallet::<T>::block_number();

            let payment = RecurringPayment {
                id: payment_id,
                beneficiary: beneficiary.clone(),
                amount,
                interval,
                next_payment: current_block.saturating_add(interval),
                payments_made: 0,
                total_payments,
                description,
                active: true,
            };

            RecurringPayments::<T>::insert(payment_id, payment);
            RecurringPaymentCount::<T>::put(payment_id.saturating_add(1));

            Self::deposit_event(Event::RecurringPaymentCreated {
                payment_id,
                beneficiary,
                amount,
                interval,
            });

            Ok(())
        }

        /// Cancel a recurring payment.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::cancel_recurring_payment())]
        pub fn cancel_recurring_payment(origin: OriginFor<T>, payment_id: u32) -> DispatchResult {
            T::MediumSpendOrigin::ensure_origin(origin)?;

            RecurringPayments::<T>::try_mutate(payment_id, |maybe_payment| -> DispatchResult {
                let payment = maybe_payment
                    .as_mut()
                    .ok_or(Error::<T>::RecurringPaymentNotFound)?;
                payment.active = false;
                Ok(())
            })?;

            Self::deposit_event(Event::RecurringPaymentCancelled { payment_id });

            Ok(())
        }

        /// Register a yield strategy (delegated to AI agent).
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::register_yield_strategy())]
        pub fn register_yield_strategy(
            origin: OriginFor<T>,
            agent: T::AccountId,
            max_allocation: BalanceOf<T>,
            min_expected_return: Percent,
            risk_level: RiskLevel,
            description: BoundedVec<u8, ConstU32<256>>,
        ) -> DispatchResult {
            T::YieldConfigOrigin::ensure_origin(origin)?;
            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);

            let strategy_id = YieldStrategyCount::<T>::get();
            ensure!(
                strategy_id < T::MaxYieldStrategies::get(),
                Error::<T>::TooManyYieldStrategies
            );

            let strategy = YieldStrategy {
                id: strategy_id,
                agent: agent.clone(),
                max_allocation,
                current_allocation: Zero::zero(),
                min_expected_return,
                total_profit: Zero::zero(),
                total_loss: Zero::zero(),
                executions: 0,
                risk_level,
                description,
                active: true,
            };

            YieldStrategies::<T>::insert(strategy_id, strategy);
            YieldStrategyCount::<T>::put(strategy_id.saturating_add(1));

            Self::deposit_event(Event::YieldStrategyRegistered {
                strategy_id,
                agent,
                max_allocation,
            });

            Ok(())
        }

        /// Execute a yield strategy (called by AI agent).
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::execute_yield_strategy())]
        pub fn execute_yield_strategy(
            origin: OriginFor<T>,
            strategy_id: u32,
            amount: BalanceOf<T>,
            expected_return: BalanceOf<T>,
        ) -> DispatchResult {
            let agent = ensure_signed(origin)?;
            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);

            YieldStrategies::<T>::try_mutate(strategy_id, |maybe_strategy| -> DispatchResult {
                let strategy = maybe_strategy
                    .as_mut()
                    .ok_or(Error::<T>::YieldStrategyNotFound)?;

                ensure!(strategy.active, Error::<T>::StrategyNotActive);
                ensure!(strategy.agent == agent, Error::<T>::NotStrategyAgent);

                let new_allocation = strategy.current_allocation.saturating_add(amount);
                ensure!(
                    new_allocation <= strategy.max_allocation,
                    Error::<T>::AllocationExceeded
                );

                // Verify treasury has funds
                let treasury_account = Self::account_id();
                let treasury_balance = T::Currency::free_balance(&treasury_account);
                ensure!(treasury_balance >= amount, Error::<T>::InsufficientBalance);

                // Transfer to agent for strategy execution
                T::Currency::transfer(
                    &treasury_account,
                    &agent,
                    amount,
                    ExistenceRequirement::KeepAlive,
                )?;

                strategy.current_allocation = new_allocation;
                strategy.executions = strategy.executions.saturating_add(1);

                Ok(())
            })?;

            Self::deposit_event(Event::YieldStrategyExecuted {
                strategy_id,
                amount,
                profit: expected_return,
            });

            Ok(())
        }

        /// Report yield strategy returns (called by AI agent).
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::report_yield_return())]
        pub fn report_yield_return(
            origin: OriginFor<T>,
            strategy_id: u32,
            returned_amount: BalanceOf<T>,
            original_amount: BalanceOf<T>,
        ) -> DispatchResult {
            let agent = ensure_signed(origin)?;

            YieldStrategies::<T>::try_mutate(strategy_id, |maybe_strategy| -> DispatchResult {
                let strategy = maybe_strategy
                    .as_mut()
                    .ok_or(Error::<T>::YieldStrategyNotFound)?;

                ensure!(strategy.agent == agent, Error::<T>::NotStrategyAgent);

                // Transfer returns back to treasury
                let treasury_account = Self::account_id();
                T::Currency::transfer(
                    &agent,
                    &treasury_account,
                    returned_amount,
                    ExistenceRequirement::KeepAlive,
                )?;

                // Update strategy stats
                strategy.current_allocation =
                    strategy.current_allocation.saturating_sub(original_amount);

                if returned_amount > original_amount {
                    let profit = returned_amount.saturating_sub(original_amount);
                    strategy.total_profit = strategy.total_profit.saturating_add(profit);

                    Stats::<T>::mutate(|stats| {
                        stats.total_yield_earned = stats.total_yield_earned.saturating_add(profit);
                    });
                } else {
                    let loss = original_amount.saturating_sub(returned_amount);
                    strategy.total_loss = strategy.total_loss.saturating_add(loss);
                }

                Ok(())
            })
        }

        /// Deactivate a yield strategy.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::deactivate_yield_strategy())]
        pub fn deactivate_yield_strategy(origin: OriginFor<T>, strategy_id: u32) -> DispatchResult {
            T::YieldConfigOrigin::ensure_origin(origin)?;

            YieldStrategies::<T>::try_mutate(strategy_id, |maybe_strategy| -> DispatchResult {
                let strategy = maybe_strategy
                    .as_mut()
                    .ok_or(Error::<T>::YieldStrategyNotFound)?;
                strategy.active = false;
                Ok(())
            })?;

            Self::deposit_event(Event::YieldStrategyDeactivated { strategy_id });

            Ok(())
        }

        /// Emergency pause the treasury.
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::pause())]
        pub fn pause(
            origin: OriginFor<T>,
            reason: BoundedVec<u8, ConstU32<256>>,
        ) -> DispatchResult {
            // Verify pause origin (e.g., root, council, or emergency multisig)
            T::PauseOrigin::ensure_origin(origin.clone())?;

            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);

            IsPaused::<T>::put(true);

            // Try to extract the actual caller; fall back to treasury account for
            // root/collective origins that don't have an associated AccountId
            let paused_by =
                frame_system::ensure_signed(origin).unwrap_or_else(|_| Self::account_id());

            let pause_info = EmergencyPause {
                paused_by: paused_by.clone(),
                paused_at: frame_system::Pallet::<T>::block_number(),
                reason: reason.clone(),
            };
            PauseInfo::<T>::put(pause_info);

            Self::deposit_event(Event::TreasuryPaused {
                by: paused_by,
                reason,
            });

            Ok(())
        }

        /// Unpause the treasury.
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::unpause())]
        pub fn unpause(origin: OriginFor<T>) -> DispatchResult {
            T::PauseOrigin::ensure_origin(origin.clone())?;

            ensure!(IsPaused::<T>::get(), Error::<T>::TreasuryNotPaused);

            IsPaused::<T>::put(false);
            PauseInfo::<T>::kill();

            // Try to extract caller; fall back to treasury account for root/collective
            let unpaused_by =
                frame_system::ensure_signed(origin).unwrap_or_else(|_| Self::account_id());

            Self::deposit_event(Event::TreasuryUnpaused { by: unpaused_by });

            Ok(())
        }

        /// Update the list of signers.
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::update_signers())]
        pub fn update_signers(
            origin: OriginFor<T>,
            new_signers: Vec<T::AccountId>,
        ) -> DispatchResult {
            T::CriticalSpendOrigin::ensure_origin(origin)?;

            let signers: BoundedVec<T::AccountId, T::MaxSigners> =
                new_signers
                    .clone()
                    .try_into()
                    .map_err(|_| Error::<T>::TooManySigners)?;

            Signers::<T>::put(signers);

            Self::deposit_event(Event::SignersUpdated {
                signers: new_signers,
            });

            Ok(())
        }

        /// Deposit funds to treasury.
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::deposit())]
        pub fn deposit(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);

            let treasury_account = Self::account_id();
            T::Currency::transfer(
                &who,
                &treasury_account,
                amount,
                ExistenceRequirement::KeepAlive,
            )?;

            Stats::<T>::mutate(|stats| {
                stats.total_deposited = stats.total_deposited.saturating_add(amount);
            });

            Self::deposit_event(Event::Deposited { from: who, amount });

            Ok(())
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Treasury account ID.
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Treasury balance.
        pub fn balance() -> BalanceOf<T> {
            T::Currency::free_balance(&Self::account_id())
        }

        /// Determine spending track based on amount.
        fn determine_track(amount: BalanceOf<T>) -> SpendingTrack {
            if amount <= T::SmallSpendLimit::get() {
                SpendingTrack::Small
            } else if amount <= T::MediumSpendLimit::get() {
                SpendingTrack::Medium
            } else if amount <= T::LargeSpendLimit::get() {
                SpendingTrack::Large
            } else {
                SpendingTrack::Critical
            }
        }

        /// Get approval threshold for track.
        fn get_threshold(track: &SpendingTrack) -> u32 {
            let signer_count = Signers::<T>::get().len() as u32;
            match track {
                SpendingTrack::Small => (signer_count + 2) / 3, // ~33%
                SpendingTrack::Medium => (signer_count + 1) / 2, // ~50%
                SpendingTrack::Large => (signer_count * 2 + 2) / 3, // ~67%
                SpendingTrack::Critical => signer_count,        // 100%
            }
        }

        /// Calculate proposal bond.
        fn calculate_bond(amount: BalanceOf<T>) -> BalanceOf<T> {
            let bond = T::ProposalBond::get().mul_floor(amount);
            bond.max(T::ProposalBondMinimum::get())
        }

        /// Execute a proposal internally.
        fn do_execute_proposal(proposal_id: u32) -> DispatchResult {
            let mut proposal =
                Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(
                proposal.status == ProposalStatus::Pending,
                Error::<T>::AlreadyExecuted
            );

            // Verify threshold
            let approvals = Approvals::<T>::get(proposal_id);
            let threshold = Self::get_threshold(&proposal.track);
            ensure!(
                approvals.len() as u32 >= threshold,
                Error::<T>::ThresholdNotReached
            );

            // Transfer funds
            let treasury_account = Self::account_id();
            T::Currency::transfer(
                &treasury_account,
                &proposal.beneficiary,
                proposal.amount,
                ExistenceRequirement::AllowDeath,
            )?;

            // Return bond
            T::Currency::unreserve(&proposal.proposer, proposal.bond);

            // Update proposal
            proposal.status = ProposalStatus::Executed;
            proposal.executed_at = Some(frame_system::Pallet::<T>::block_number());
            Proposals::<T>::insert(proposal_id, proposal.clone());

            // Update stats
            Stats::<T>::mutate(|stats| {
                stats.total_spent = stats.total_spent.saturating_add(proposal.amount);
                stats.proposals_executed = stats.proposals_executed.saturating_add(1);
            });

            Self::deposit_event(Event::ProposalExecuted {
                proposal_id,
                beneficiary: proposal.beneficiary,
                amount: proposal.amount,
            });

            Ok(())
        }

        /// Process recurring payments due this block.
        fn process_recurring_payments(current_block: BlockNumberFor<T>) -> Weight {
            let mut weight = Weight::zero();

            for payment_id in 0..RecurringPaymentCount::<T>::get() {
                if let Some(mut payment) = RecurringPayments::<T>::get(payment_id) {
                    if payment.active && current_block >= payment.next_payment {
                        // Check if max payments reached
                        if let Some(max) = payment.total_payments {
                            if payment.payments_made >= max {
                                payment.active = false;
                                RecurringPayments::<T>::insert(payment_id, payment);
                                continue;
                            }
                        }

                        // Execute payment
                        let treasury_account = Self::account_id();
                        if T::Currency::transfer(
                            &treasury_account,
                            &payment.beneficiary,
                            payment.amount,
                            ExistenceRequirement::KeepAlive,
                        )
                        .is_ok()
                        {
                            payment.payments_made = payment.payments_made.saturating_add(1);
                            payment.next_payment = current_block.saturating_add(payment.interval);

                            Self::deposit_event(Event::RecurringPaymentExecuted {
                                payment_id,
                                beneficiary: payment.beneficiary.clone(),
                                amount: payment.amount,
                            });
                        }

                        RecurringPayments::<T>::insert(payment_id, payment);
                        weight = weight.saturating_add(T::DbWeight::get().reads_writes(2, 1));
                    }
                }
            }

            weight
        }
    }
}
