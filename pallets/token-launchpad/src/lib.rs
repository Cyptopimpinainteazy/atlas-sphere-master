//! Token Launchpad Pallet
//!
//! Fair token distribution and bonding curve mechanisms:
//! - Linear/exponential bonding curves
//! - Capped/uncapped token sales
//! - Vesting schedules for team/treasury
//! - Refund mechanisms if targets not met
//! - Governance-controlled parameters

#![cfg_attr(not(feature = "std"), no_std)]


mod bonding;
mod types;

pub use types::*;

#[frame_support::pallet]
pub mod pallet {
    use crate::types::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Zero, One, Saturating};
    use sp_std::vec::Vec;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        #[pallet::constant]
        type MaxTokenNameLength: Get<u32>;

        #[pallet::constant]
        type MaxTeamMembers: Get<u32>;
    }

    // Storage

    #[pallet::storage]
    pub type Tokens<T: Config> = StorageMap<_, Blake2_128Concat, u32, TokenInfo<T::AccountId, T::BlockNumber>>;

    #[pallet::storage]
    pub type TokenCounter<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    pub type Contributions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32,  // token_id
        Blake2_128Concat,
        T::AccountId,
        u128,  // amount
    >;

    #[pallet::storage]
    pub type VestingSchedules<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32,  // token_id
        Blake2_128Concat,
        T::AccountId,
        VestingSchedule<T::BlockNumber>,
    >;

    // Extrinsics

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new token with bonding curve
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 2))]
        pub fn create_token(
            origin: OriginFor<T>,
            name: Vec<u8>,
            symbol: Vec<u8>,
            initial_supply: u128,
            curve_type: BondingCurveType,
            min_purchase: u128,
            max_purchase: u128,
            hard_cap: u128,
            duration_blocks: T::BlockNumber,
        ) -> DispatchResult {
            let creator = ensure_signed(origin)?;

            // Validate inputs
            ensure!(name.len() <= T::MaxTokenNameLength::get() as usize, Error::<T>::NameTooLong);
            ensure!(min_purchase > Zero::zero(), Error::<T>::InvalidMinimum);
            ensure!(max_purchase >= min_purchase, Error::<T>::InvalidMaximum);
            ensure!(hard_cap > Zero::zero(), Error::<T>::InvalidCap);

            let token_id = <TokenCounter<T>>::get();
            let now = <frame_system::Pallet<T>>::block_number();

            let token = TokenInfo {
                id: token_id,
                creator: creator.clone(),
                name,
                symbol,
                initial_supply,
                total_raised: Zero::zero(),
                curve_type,
                min_purchase,
                max_purchase,
                hard_cap,
                created_at: now,
                end_at: now + duration_blocks,
                status: SaleStatus::Active,
                team_members: Default::default(),
            };

            <Tokens<T>>::insert(token_id, token);
            <TokenCounter<T>>::put(token_id + 1);

            Self::deposit_event(Event::TokenCreated {
                token_id,
                creator,
                initial_supply,
            });

            Ok(())
        }

        /// Contribute to token sale
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
        pub fn contribute(
            origin: OriginFor<T>,
            token_id: u32,
            amount: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut token = <Tokens<T>>::get(token_id)
                .ok_or(Error::<T>::TokenNotFound)?;

            // Validate sale is active
            ensure!(token.status == SaleStatus::Active, Error::<T>::SaleNotActive);

            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(now <= token.end_at, Error::<T>::SaleEnded);

            // Validate amount
            ensure!(amount >= token.min_purchase, Error::<T>::BelowMinimum);
            ensure!(amount <= token.max_purchase, Error::<T>::AboveMaximum);
            ensure!(token.total_raised + amount <= token.hard_cap, Error::<T>::HardCapExceeded);

            // Transfer funds
            <pallet_balances::Pallet<T> as pallet_balances::fungible::Mutate<T::AccountId>>::transfer(
                &who,
                &token.creator,
                amount.try_into().unwrap_or_default(),
                true,
            )?;

            // Record contribution
            <Contributions<T>>::insert(token_id, &who, amount);

            token.total_raised += amount;
            <Tokens<T>>::insert(token_id, token.clone());

            Self::deposit_event(Event::ContributionReceived {
                token_id,
                contributor: who,
                amount,
            });

            Ok(())
        }

        /// Finalize sale and distribute tokens
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn finalize_sale(
            origin: OriginFor<T>,
            token_id: u32,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let mut token = <Tokens<T>>::get(token_id)
                .ok_or(Error::<T>::TokenNotFound)?;

            // Can only finalize after end time
            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(now > token.end_at, Error::<T>::SaleNotEnded);

            // Check if hard cap reached
            if token.total_raised >= token.hard_cap {
                token.status = SaleStatus::Success;
            } else {
                token.status = SaleStatus::Failed;
            }

            <Tokens<T>>::insert(token_id, token.clone());

            Self::deposit_event(Event::SaleFinalized {
                token_id,
                status: token.status,
                total_raised: token.total_raised,
            });

            Ok(())
        }

        /// Add team member with vesting schedule
        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
        pub fn add_team_member(
            origin: OriginFor<T>,
            token_id: u32,
            member: T::AccountId,
            tokens: u128,
            cliff_blocks: T::BlockNumber,
            vesting_blocks: T::BlockNumber,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut token = <Tokens<T>>::get(token_id)
                .ok_or(Error::<T>::TokenNotFound)?;

            // Only creator can add team members
            ensure!(who == token.creator, Error::<T>::Unauthorized);
            ensure!(token.team_members.len() < T::MaxTeamMembers::get() as usize, Error::<T>::TooManyTeamMembers);

            let now = <frame_system::Pallet<T>>::block_number();
            let vesting = VestingSchedule {
                total: tokens,
                claimed: Zero::zero(),
                start_block: now,
                cliff_block: now + cliff_blocks,
                end_block: now + vesting_blocks,
            };

            <VestingSchedules<T>>::insert(token_id, &member, vesting);
            token.team_members.push(member.clone());
            <Tokens<T>>::insert(token_id, token);

            Self::deposit_event(Event::TeamMemberAdded {
                token_id,
                member,
                amount: tokens,
            });

            Ok(())
        }
    }

    // Events

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TokenCreated {
            token_id: u32,
            creator: T::AccountId,
            initial_supply: u128,
        },
        ContributionReceived {
            token_id: u32,
            contributor: T::AccountId,
            amount: u128,
        },
        SaleFinalized {
            token_id: u32,
            status: SaleStatus,
            total_raised: u128,
        },
        TeamMemberAdded {
            token_id: u32,
            member: T::AccountId,
            amount: u128,
        },
    }

    // Errors

    #[pallet::error]
    pub enum Error<T> {
        TokenNotFound,
        NameTooLong,
        InvalidMinimum,
        InvalidMaximum,
        InvalidCap,
        SaleNotActive,
        SaleNotEnded,
        SaleEnded,
        BelowMinimum,
        AboveMaximum,
        HardCapExceeded,
        Unauthorized,
        TooManyTeamMembers,
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub _phantom: sp_std::marker::PhantomData<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                _phantom: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {}
    }
}
