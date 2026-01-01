//! Weights for pallet-governance.

use frame_support::weights::Weight;

/// Weight functions for `pallet_governance`.
pub trait WeightInfo {
    fn submit_proposal() -> Weight;
    fn vote() -> Weight;
    fn delegate() -> Weight;
    fn undelegate() -> Weight;
    fn fast_track() -> Weight;
    fn cancel_proposal() -> Weight;
    fn finalize_proposal() -> Weight;
    fn unlock() -> Weight;
    fn update_config() -> Weight;
}

/// Default weights for testing.
impl WeightInfo for () {
    fn submit_proposal() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }

    fn vote() -> Weight {
        Weight::from_parts(40_000_000, 0)
    }

    fn delegate() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn undelegate() -> Weight {
        Weight::from_parts(25_000_000, 0)
    }

    fn fast_track() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }

    fn cancel_proposal() -> Weight {
        Weight::from_parts(35_000_000, 0)
    }

    fn finalize_proposal() -> Weight {
        Weight::from_parts(60_000_000, 0)
    }

    fn unlock() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn update_config() -> Weight {
        Weight::from_parts(15_000_000, 0)
    }
}
