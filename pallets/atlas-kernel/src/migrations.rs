//! Runtime storage migrations for `pallet-atlas-kernel`.

use frame_support::weights::Weight;
use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
use sp_std::marker::PhantomData;

use crate::pallet;

pub struct Migration<T>(PhantomData<T>);

impl<T: pallet::Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
        // Current migration: Ensure storage version is set to 1
        if StorageVersion::get::<pallet::Pallet<T>>() < pallet::STORAGE_VERSION {
            (&pallet::STORAGE_VERSION).put::<pallet::Pallet<T>>();
            // Reads: get + put; Writes: put
            Weight::from_parts(2_000, 0)
        } else {
            Weight::zero()
        }
    }
}
