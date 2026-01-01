//! Weights for pallet-agent-accounts.

use frame_support::weights::Weight;

/// Weight functions needed for pallet_agent_accounts.
pub trait WeightInfo {
    fn register_agent() -> Weight;
    fn update_operator() -> Weight;
    fn update_permissions() -> Weight;
    fn update_quota() -> Weight;
    fn suspend_agent() -> Weight;
    fn reactivate_agent() -> Weight;
    fn terminate_agent() -> Weight;
    fn record_consumption() -> Weight;
    fn update_reputation() -> Weight;
    fn emit_action() -> Weight;
}

/// Default weights for testing.
impl WeightInfo for () {
    fn register_agent() -> Weight {
        Weight::from_parts(60_000_000, 0)
    }

    fn update_operator() -> Weight {
        Weight::from_parts(40_000_000, 0)
    }

    fn update_permissions() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn update_quota() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn suspend_agent() -> Weight {
        Weight::from_parts(35_000_000, 0)
    }

    fn reactivate_agent() -> Weight {
        Weight::from_parts(35_000_000, 0)
    }

    fn terminate_agent() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }

    fn record_consumption() -> Weight {
        Weight::from_parts(25_000_000, 0)
    }

    fn update_reputation() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn emit_action() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }
}
