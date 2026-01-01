//! Weight information for the X3 Verifier pallet.

use frame_support::{traits::Get, weights::Weight};

/// Weight functions for pallet_x3_verifier.
pub trait WeightInfo {
    fn register_executor() -> Weight;
    fn submit_job() -> Weight;
    fn submit_receipt() -> Weight;
    fn dispute_receipt() -> Weight;
    fn toggle_verification() -> Weight;
    fn deactivate_executor() -> Weight;
}

/// Default weights for development
impl WeightInfo for () {
    fn register_executor() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }

    fn submit_job() -> Weight {
        Weight::from_parts(40_000_000, 0)
    }

    fn submit_receipt() -> Weight {
        Weight::from_parts(100_000_000, 0)
    }

    fn dispute_receipt() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn toggle_verification() -> Weight {
        Weight::from_parts(10_000_000, 0)
    }

    fn deactivate_executor() -> Weight {
        Weight::from_parts(25_000_000, 0)
    }
}

/// Substrate-style weight implementation for production use
pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn register_executor() -> Weight {
        Weight::from_parts(50_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn submit_job() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn submit_receipt() -> Weight {
        Weight::from_parts(100_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn dispute_receipt() -> Weight {
        Weight::from_parts(30_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn toggle_verification() -> Weight {
        Weight::from_parts(10_000_000, 0).saturating_add(T::DbWeight::get().writes(1))
    }

    fn deactivate_executor() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}
