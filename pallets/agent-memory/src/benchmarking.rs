//! Benchmarking for the Agent Memory pallet.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::types::EntryType;
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn initialize_memory() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        #[extrinsic_call]
        initialize_memory(RawOrigin::Signed(caller), 0, operator);
    }

    #[benchmark]
    fn append_entry() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        // Initialize memory first
        let _ = Pallet::<T>::initialize_memory(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            caller.clone(),
        );

        let content: BoundedVec<u8, T::MaxContentLength> = vec![0u8; 1000].try_into().unwrap();

        #[extrinsic_call]
        append_entry(
            RawOrigin::Signed(caller),
            0,
            EntryType::Observation,
            content,
            None,
        );
    }

    #[benchmark]
    fn append_batch() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        let _ = Pallet::<T>::initialize_memory(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            caller.clone(),
        );

        let entries: Vec<(EntryType, BoundedVec<u8, T::MaxContentLength>)> = (0..10)
            .map(|_| {
                let content: BoundedVec<u8, T::MaxContentLength> =
                    vec![0u8; 100].try_into().unwrap();
                (EntryType::Observation, content)
            })
            .collect();

        #[extrinsic_call]
        append_batch(RawOrigin::Signed(caller), 0, entries);
    }

    #[benchmark]
    fn update_permissions() {
        let caller: T::AccountId = whitelisted_caller();
        let reader: T::AccountId = account("reader", 0, 0);
        let writer: T::AccountId = account("writer", 0, 0);

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        let _ = Pallet::<T>::initialize_memory(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            caller.clone(),
        );

        #[extrinsic_call]
        update_permissions(
            RawOrigin::Signed(caller),
            0,
            true,
            vec![reader],
            vec![writer],
        );
    }

    #[benchmark]
    fn prune_memory() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        let _ = Pallet::<T>::initialize_memory(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            caller.clone(),
        );

        // Add some entries
        for _ in 0..10 {
            let content: BoundedVec<u8, T::MaxContentLength> = vec![0u8; 100].try_into().unwrap();
            let _ = Pallet::<T>::append_entry(
                RawOrigin::Signed(caller.clone()).into(),
                0,
                EntryType::Observation,
                content,
                None,
            );
        }

        #[extrinsic_call]
        prune_memory(RawOrigin::Root, 0, 0);
    }

    #[benchmark]
    fn increase_deposit() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        let _ = Pallet::<T>::initialize_memory(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            caller.clone(),
        );

        #[extrinsic_call]
        increase_deposit(RawOrigin::Signed(caller), 0, 10_000u32.into());
    }

    #[benchmark]
    fn withdraw_deposit() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        let _ = Pallet::<T>::initialize_memory(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            caller.clone(),
        );

        // Add deposit first
        let _ = Pallet::<T>::increase_deposit(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            10_000u32.into(),
        );

        #[extrinsic_call]
        withdraw_deposit(RawOrigin::Signed(caller), 0, 5_000u32.into());
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
