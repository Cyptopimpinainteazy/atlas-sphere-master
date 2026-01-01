//! Runtime storage migrations for `pallet-meme-overlord`.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;
use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
use sp_std::marker::PhantomData;

use crate::pallet;

pub struct Migration<T>(PhantomData<T>);

impl<T: crate::Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
        if StorageVersion::get::<pallet::Pallet<T>>() < pallet::STORAGE_VERSION {
            StorageVersion::put::<pallet::Pallet<T>>(pallet::STORAGE_VERSION);
            <Weight as From<u64>>::from(2u64)
        } else {
            Weight::zero()
        }
    }
}
