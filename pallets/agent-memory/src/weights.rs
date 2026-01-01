//! Weights for pallet-agent-memory.

use frame_support::weights::Weight;

/// Weight functions needed for pallet_agent_memory.
pub trait WeightInfo {
    fn initialize_memory() -> Weight;
    fn append_entry() -> Weight;
    fn append_batch() -> Weight;
    fn update_permissions() -> Weight;
    fn prune_memory() -> Weight;
    fn increase_deposit() -> Weight;
    fn withdraw_deposit() -> Weight;
}

/// Default weights for testing.
impl WeightInfo for () {
    fn initialize_memory() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }

    fn append_entry() -> Weight {
        Weight::from_parts(40_000_000, 0)
    }

    fn append_batch() -> Weight {
        Weight::from_parts(100_000_000, 0)
    }

    fn update_permissions() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn prune_memory() -> Weight {
        Weight::from_parts(80_000_000, 0)
    }

    fn increase_deposit() -> Weight {
        Weight::from_parts(35_000_000, 0)
    }

    fn withdraw_deposit() -> Weight {
        Weight::from_parts(35_000_000, 0)
    }
}
