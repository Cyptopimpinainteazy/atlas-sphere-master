//! Weights for pallet-treasury.

use frame_support::weights::Weight;

/// Weight functions needed for pallet_treasury.
pub trait WeightInfo {
    fn submit_proposal() -> Weight;
    fn approve_proposal() -> Weight;
    fn execute_proposal() -> Weight;
    fn reject_proposal() -> Weight;
    fn create_recurring_payment() -> Weight;
    fn cancel_recurring_payment() -> Weight;
    fn register_yield_strategy() -> Weight;
    fn execute_yield_strategy() -> Weight;
    fn report_yield_return() -> Weight;
    fn deactivate_yield_strategy() -> Weight;
    fn pause() -> Weight;
    fn unpause() -> Weight;
    fn update_signers() -> Weight;
    fn deposit() -> Weight;
}

/// Default weights for testing.
impl WeightInfo for () {
    fn submit_proposal() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }

    fn approve_proposal() -> Weight {
        Weight::from_parts(40_000_000, 0)
    }

    fn execute_proposal() -> Weight {
        Weight::from_parts(60_000_000, 0)
    }

    fn reject_proposal() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn create_recurring_payment() -> Weight {
        Weight::from_parts(45_000_000, 0)
    }

    fn cancel_recurring_payment() -> Weight {
        Weight::from_parts(25_000_000, 0)
    }

    fn register_yield_strategy() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }

    fn execute_yield_strategy() -> Weight {
        Weight::from_parts(70_000_000, 0)
    }

    fn report_yield_return() -> Weight {
        Weight::from_parts(55_000_000, 0)
    }

    fn deactivate_yield_strategy() -> Weight {
        Weight::from_parts(25_000_000, 0)
    }

    fn pause() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }

    fn unpause() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }

    fn update_signers() -> Weight {
        Weight::from_parts(35_000_000, 0)
    }

    fn deposit() -> Weight {
        Weight::from_parts(40_000_000, 0)
    }
}
