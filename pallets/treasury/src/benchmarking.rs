//! Benchmarking for the Treasury pallet.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::types::RiskLevel;
use frame_benchmarking::v2::*;
use frame_support::{traits::Get, BoundedVec};
use frame_system::RawOrigin;
use sp_runtime::Percent;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn submit_proposal() {
        let caller: T::AccountId = whitelisted_caller();
        let beneficiary: T::AccountId = account("beneficiary", 0, 0);
        let amount: BalanceOf<T> = 1000u32.into();
        let description: BoundedVec<u8, ConstU32<1024>> = vec![0u8; 100].try_into().unwrap();

        // Fund the caller for bond
        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        #[extrinsic_call]
        submit_proposal(RawOrigin::Signed(caller), beneficiary, amount, description);
    }

    #[benchmark]
    fn approve_proposal() {
        let proposer: T::AccountId = account("proposer", 0, 0);
        let beneficiary: T::AccountId = account("beneficiary", 0, 0);
        let signer: T::AccountId = whitelisted_caller();

        // Setup signer
        let signers: BoundedVec<T::AccountId, T::MaxSigners> =
            vec![signer.clone()].try_into().unwrap();
        Signers::<T>::put(signers);

        // Fund proposer
        let _ = T::Currency::make_free_balance_be(&proposer, 1_000_000u32.into());

        // Create proposal
        let description: BoundedVec<u8, ConstU32<1024>> = vec![0u8; 100].try_into().unwrap();
        let _ = Pallet::<T>::submit_proposal(
            RawOrigin::Signed(proposer).into(),
            beneficiary,
            1000u32.into(),
            description,
        );

        #[extrinsic_call]
        approve_proposal(RawOrigin::Signed(signer), 0);
    }

    #[benchmark]
    fn execute_proposal() {
        let proposer: T::AccountId = account("proposer", 0, 0);
        let beneficiary: T::AccountId = account("beneficiary", 0, 0);
        let signer: T::AccountId = whitelisted_caller();

        // Setup signers
        let signers: BoundedVec<T::AccountId, T::MaxSigners> =
            vec![signer.clone()].try_into().unwrap();
        Signers::<T>::put(signers);

        // Fund proposer and treasury
        let _ = T::Currency::make_free_balance_be(&proposer, 1_000_000u32.into());
        let treasury_account = Pallet::<T>::account_id();
        let _ = T::Currency::make_free_balance_be(&treasury_account, 1_000_000u32.into());

        // Create and approve proposal
        let description: BoundedVec<u8, ConstU32<1024>> = vec![0u8; 100].try_into().unwrap();
        let _ = Pallet::<T>::submit_proposal(
            RawOrigin::Signed(proposer).into(),
            beneficiary,
            1000u32.into(),
            description,
        );
        let _ = Pallet::<T>::approve_proposal(RawOrigin::Signed(signer.clone()).into(), 0);

        #[extrinsic_call]
        execute_proposal(RawOrigin::Signed(signer), 0);
    }

    #[benchmark]
    fn reject_proposal() {
        let proposer: T::AccountId = account("proposer", 0, 0);
        let beneficiary: T::AccountId = account("beneficiary", 0, 0);

        // Fund proposer
        let _ = T::Currency::make_free_balance_be(&proposer, 1_000_000u32.into());

        // Create proposal
        let description: BoundedVec<u8, ConstU32<1024>> = vec![0u8; 100].try_into().unwrap();
        let _ = Pallet::<T>::submit_proposal(
            RawOrigin::Signed(proposer).into(),
            beneficiary,
            500u32.into(), // Small track
            description,
        );

        #[extrinsic_call]
        reject_proposal(RawOrigin::Root, 0);
    }

    #[benchmark]
    fn create_recurring_payment() {
        let beneficiary: T::AccountId = account("beneficiary", 0, 0);
        let amount: BalanceOf<T> = 1000u32.into();
        let interval: BlockNumberFor<T> = 100u32.into();
        let description: BoundedVec<u8, ConstU32<256>> = vec![0u8; 50].try_into().unwrap();

        #[extrinsic_call]
        create_recurring_payment(
            RawOrigin::Root,
            beneficiary,
            amount,
            interval,
            Some(12),
            description,
        );
    }

    #[benchmark]
    fn cancel_recurring_payment() {
        let beneficiary: T::AccountId = account("beneficiary", 0, 0);
        let description: BoundedVec<u8, ConstU32<256>> = vec![0u8; 50].try_into().unwrap();

        // Create payment
        let _ = Pallet::<T>::create_recurring_payment(
            RawOrigin::Root.into(),
            beneficiary,
            1000u32.into(),
            100u32.into(),
            None,
            description,
        );

        #[extrinsic_call]
        cancel_recurring_payment(RawOrigin::Root, 0);
    }

    #[benchmark]
    fn register_yield_strategy() {
        let agent: T::AccountId = account("agent", 0, 0);
        let max_allocation: BalanceOf<T> = 50_000u32.into();
        let description: BoundedVec<u8, ConstU32<256>> = vec![0u8; 50].try_into().unwrap();

        #[extrinsic_call]
        register_yield_strategy(
            RawOrigin::Root,
            agent,
            max_allocation,
            Percent::from_percent(5),
            RiskLevel::Medium,
            description,
        );
    }

    #[benchmark]
    fn execute_yield_strategy() {
        let agent: T::AccountId = whitelisted_caller();
        let description: BoundedVec<u8, ConstU32<256>> = vec![0u8; 50].try_into().unwrap();

        // Fund treasury
        let treasury_account = Pallet::<T>::account_id();
        let _ = T::Currency::make_free_balance_be(&treasury_account, 1_000_000u32.into());

        // Register strategy
        let _ = Pallet::<T>::register_yield_strategy(
            RawOrigin::Root.into(),
            agent.clone(),
            100_000u32.into(),
            Percent::from_percent(5),
            RiskLevel::Low,
            description,
        );

        #[extrinsic_call]
        execute_yield_strategy(RawOrigin::Signed(agent), 0, 10_000u32.into(), 500u32.into());
    }

    #[benchmark]
    fn report_yield_return() {
        let agent: T::AccountId = whitelisted_caller();
        let description: BoundedVec<u8, ConstU32<256>> = vec![0u8; 50].try_into().unwrap();

        // Fund treasury and agent
        let treasury_account = Pallet::<T>::account_id();
        let _ = T::Currency::make_free_balance_be(&treasury_account, 1_000_000u32.into());
        let _ = T::Currency::make_free_balance_be(&agent, 1_000_000u32.into());

        // Register and execute strategy
        let _ = Pallet::<T>::register_yield_strategy(
            RawOrigin::Root.into(),
            agent.clone(),
            100_000u32.into(),
            Percent::from_percent(5),
            RiskLevel::Low,
            description,
        );
        let _ = Pallet::<T>::execute_yield_strategy(
            RawOrigin::Signed(agent.clone()).into(),
            0,
            10_000u32.into(),
            500u32.into(),
        );

        #[extrinsic_call]
        report_yield_return(
            RawOrigin::Signed(agent),
            0,
            10_500u32.into(),
            10_000u32.into(),
        );
    }

    #[benchmark]
    fn deactivate_yield_strategy() {
        let agent: T::AccountId = account("agent", 0, 0);
        let description: BoundedVec<u8, ConstU32<256>> = vec![0u8; 50].try_into().unwrap();

        // Register strategy
        let _ = Pallet::<T>::register_yield_strategy(
            RawOrigin::Root.into(),
            agent,
            50_000u32.into(),
            Percent::from_percent(5),
            RiskLevel::Low,
            description,
        );

        #[extrinsic_call]
        deactivate_yield_strategy(RawOrigin::Root, 0);
    }

    #[benchmark]
    fn pause() {
        let reason: BoundedVec<u8, ConstU32<256>> = b"Emergency".to_vec().try_into().unwrap();

        #[extrinsic_call]
        pause(RawOrigin::Root, reason);
    }

    #[benchmark]
    fn unpause() {
        let reason: BoundedVec<u8, ConstU32<256>> = b"Emergency".to_vec().try_into().unwrap();
        let _ = Pallet::<T>::pause(RawOrigin::Root.into(), reason);

        #[extrinsic_call]
        unpause(RawOrigin::Root);
    }

    #[benchmark]
    fn update_signers() {
        let signer1: T::AccountId = account("signer1", 0, 0);
        let signer2: T::AccountId = account("signer2", 0, 0);
        let signers = vec![signer1, signer2];

        #[extrinsic_call]
        update_signers(RawOrigin::Root, signers);
    }

    #[benchmark]
    fn deposit() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 1_000_000u32.into());

        #[extrinsic_call]
        deposit(RawOrigin::Signed(caller), 5000u32.into());
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
