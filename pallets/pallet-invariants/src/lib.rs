#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    #![allow(dead_code)]
    use frame_support::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: frame_support::traits::Currency<Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        InvariantCreated,
    }

    #[pallet::error]
    pub enum Error<T> {
        InvariantViolated,
    }

    #[allow(dead_code)]
    #[allow(unused)]
    impl<T: Config> Pallet<T> {
        // Stub implementation
    }
}
