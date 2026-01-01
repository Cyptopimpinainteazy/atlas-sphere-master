//! Weight info for Meme Overlord pallet

use frame_support::weights::Weight;

pub trait WeightInfo {
    fn register_template() -> Weight;
    fn generate_meme() -> Weight;
    fn like_meme() -> Weight;
    fn share_meme() -> Weight;
    fn deactivate_template() -> Weight;
}

/// Default weights for testing
impl WeightInfo for () {
    fn register_template() -> Weight {
        Weight::from_parts(10_000_000, 0)
    }

    fn generate_meme() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }

    fn like_meme() -> Weight {
        Weight::from_parts(5_000_000, 0)
    }

    fn share_meme() -> Weight {
        Weight::from_parts(5_000_000, 0)
    }

    fn deactivate_template() -> Weight {
        Weight::from_parts(5_000_000, 0)
    }
}
