#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::{Weight, constants::RocksDbWeight as DbWeight};

pub trait WeightInfo {
    fn add_header() -> Weight;
    fn request_pegin() -> Weight;
    fn finalize_pegin() -> Weight;
}

impl WeightInfo for () {
    fn add_header() -> Weight {
        DbWeight::get().reads_writes(2, 2)
    }

    fn request_pegin() -> Weight {
        DbWeight::get().reads_writes(3, 2)
    }

    fn finalize_pegin() -> Weight {
        DbWeight::get().reads_writes(4, 3)
    }
}
