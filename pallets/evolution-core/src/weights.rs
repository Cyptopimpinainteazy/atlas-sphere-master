//! Weight information for the Evolution Core pallet.

use frame_support::{traits::Get, weights::Weight};

/// Weight functions for pallet_evolution_core.
pub trait WeightInfo {
    fn propose_mutation() -> Weight;
    fn approve_mutation() -> Weight;
    fn record_metrics() -> Weight;
    fn toggle_evolution() -> Weight;
    fn register_ai_agent() -> Weight;
    fn emergency_stop() -> Weight;
    fn rollback_mutation() -> Weight;
}

/// Default weights for development
impl WeightInfo for () {
    fn propose_mutation() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }

    fn approve_mutation() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn record_metrics() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }

    fn toggle_evolution() -> Weight {
        Weight::from_parts(10_000_000, 0)
    }

    fn register_ai_agent() -> Weight {
        Weight::from_parts(15_000_000, 0)
    }

    fn emergency_stop() -> Weight {
        Weight::from_parts(5_000_000, 0)
    }

    fn rollback_mutation() -> Weight {
        Weight::from_parts(40_000_000, 0)
    }
}

/// Substrate-style weight implementation for production use
pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn propose_mutation() -> Weight {
        Weight::from_parts(50_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn approve_mutation() -> Weight {
        Weight::from_parts(30_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn record_metrics() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn toggle_evolution() -> Weight {
        Weight::from_parts(10_000_000, 0).saturating_add(T::DbWeight::get().writes(1))
    }

    fn register_ai_agent() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn emergency_stop() -> Weight {
        Weight::from_parts(5_000_000, 0).saturating_add(T::DbWeight::get().writes(2))
    }

    fn rollback_mutation() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }
}
