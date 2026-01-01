//! Benchmarking for the Agent Accounts pallet.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::types::{ActionType, AgentPermissions};
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_agent() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"TestAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![0u8; 100].try_into().unwrap();

        // Fund caller
        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        #[extrinsic_call]
        register_agent(RawOrigin::Signed(caller), operator, name, metadata);
    }

    #[benchmark]
    fn update_operator() {
        let caller: T::AccountId = whitelisted_caller();
        let operator1: T::AccountId = account("operator1", 0, 0);
        let operator2: T::AccountId = account("operator2", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator1,
            name,
            metadata,
        );

        #[extrinsic_call]
        update_operator(RawOrigin::Signed(caller), 0, operator2);
    }

    #[benchmark]
    fn update_permissions() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
        );

        let permissions = AgentPermissions {
            can_deploy: true,
            can_stake: true,
            can_vote: true,
            can_trade: true,
            can_transfer: true,
            can_call_contracts: true,
        };

        #[extrinsic_call]
        update_permissions(RawOrigin::Signed(caller), 0, permissions);
    }

    #[benchmark]
    fn update_quota() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ =
            Pallet::<T>::register_agent(RawOrigin::Signed(caller).into(), operator, name, metadata);

        #[extrinsic_call]
        update_quota(
            RawOrigin::Root,
            0,
            2_000_000,
            1_000_000,
            200_000_000,
            100_000_000,
        );
    }

    #[benchmark]
    fn suspend_agent() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();
        let reason: BoundedVec<u8, ConstU32<256>> = b"Violation".to_vec().try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ =
            Pallet::<T>::register_agent(RawOrigin::Signed(caller).into(), operator, name, metadata);

        #[extrinsic_call]
        suspend_agent(RawOrigin::Root, 0, reason);
    }

    #[benchmark]
    fn reactivate_agent() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();
        let reason: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ =
            Pallet::<T>::register_agent(RawOrigin::Signed(caller).into(), operator, name, metadata);
        let _ = Pallet::<T>::suspend_agent(RawOrigin::Root.into(), 0, reason);

        #[extrinsic_call]
        reactivate_agent(RawOrigin::Root, 0);
    }

    #[benchmark]
    fn terminate_agent() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
        );

        #[extrinsic_call]
        terminate_agent(RawOrigin::Signed(caller), 0);
    }

    #[benchmark]
    fn record_consumption() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ =
            Pallet::<T>::register_agent(RawOrigin::Signed(caller).into(), operator, name, metadata);

        #[extrinsic_call]
        record_consumption(RawOrigin::Root, 0, 100_000, 50_000);
    }

    #[benchmark]
    fn update_reputation() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ =
            Pallet::<T>::register_agent(RawOrigin::Signed(caller).into(), operator, name, metadata);

        #[extrinsic_call]
        update_reputation(RawOrigin::Root, 0, 50);
    }

    #[benchmark]
    fn emit_action() {
        let caller: T::AccountId = whitelisted_caller();
        let name: BoundedVec<u8, ConstU32<64>> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, T::MaxMetadataLength> = vec![].try_into().unwrap();
        let action_data: BoundedVec<u8, ConstU32<512>> = b"{}".to_vec().try_into().unwrap();

        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            caller.clone(), // Self as operator
            name,
            metadata,
        );

        #[extrinsic_call]
        emit_action(RawOrigin::Signed(caller), ActionType::Trade, action_data);
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
