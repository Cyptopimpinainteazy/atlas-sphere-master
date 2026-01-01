//! Benchmarks for pallet-governance.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn submit_proposal() {
        let caller: T::AccountId = whitelisted_caller();
        let call = frame_system::Call::<T>::remark { remark: vec![] }.into();
        let title: BoundedVec<u8, ConstU32<256>> = vec![0u8; 256].try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = vec![0u8; 4096].try_into().unwrap();

        // Ensure caller has funds for deposit
        let deposit = T::ProposalDeposit::get();
        T::Currency::make_free_balance_be(&caller, deposit * 10u32.into());

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            Box::new(call),
            title,
            description,
        );

        assert_eq!(Pallet::<T>::proposal_count(), 1);
    }

    #[benchmark]
    fn vote() {
        let caller: T::AccountId = whitelisted_caller();
        let voter: T::AccountId = account("voter", 0, 0);

        // Setup proposal
        let call = frame_system::Call::<T>::remark { remark: vec![] }.into();
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        let deposit = T::ProposalDeposit::get();
        T::Currency::make_free_balance_be(&caller, deposit * 10u32.into());
        T::Currency::make_free_balance_be(&voter, deposit * 10u32.into());

        Pallet::<T>::submit_proposal(
            RawOrigin::Signed(caller).into(),
            Box::new(call),
            title,
            description,
        )
        .unwrap();

        let balance = deposit;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(voter),
            0u32,
            VoteDirection::Aye,
            balance,
            Conviction::Locked1x,
        );

        let tally = Pallet::<T>::proposal_votes(0);
        assert!(tally.ayes > 0u32.into());
    }

    #[benchmark]
    fn delegate() {
        let delegator: T::AccountId = whitelisted_caller();
        let target: T::AccountId = account("target", 0, 0);

        T::Currency::make_free_balance_be(&delegator, T::ProposalDeposit::get() * 10u32.into());

        #[extrinsic_call]
        _(
            RawOrigin::Signed(delegator.clone()),
            target.clone(),
            Conviction::Locked1x,
        );

        assert!(Pallet::<T>::delegations(&delegator).is_some());
    }

    #[benchmark]
    fn undelegate() {
        let delegator: T::AccountId = whitelisted_caller();
        let target: T::AccountId = account("target", 0, 0);

        T::Currency::make_free_balance_be(&delegator, T::ProposalDeposit::get() * 10u32.into());

        Pallet::<T>::delegate(
            RawOrigin::Signed(delegator.clone()).into(),
            target,
            Conviction::Locked1x,
        )
        .unwrap();

        #[extrinsic_call]
        _(RawOrigin::Signed(delegator.clone()));

        assert!(Pallet::<T>::delegations(&delegator).is_none());
    }

    #[benchmark]
    fn fast_track() {
        let caller: T::AccountId = whitelisted_caller();

        // Setup proposal
        let call = frame_system::Call::<T>::remark { remark: vec![] }.into();
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        let deposit = T::ProposalDeposit::get();
        T::Currency::make_free_balance_be(&caller, deposit * 10u32.into());

        Pallet::<T>::submit_proposal(
            RawOrigin::Signed(caller).into(),
            Box::new(call),
            title,
            description,
        )
        .unwrap();

        #[extrinsic_call]
        _(RawOrigin::Root, 0u32, 10u32.into());
    }

    #[benchmark]
    fn cancel_proposal() {
        let caller: T::AccountId = whitelisted_caller();

        // Setup proposal
        let call = frame_system::Call::<T>::remark { remark: vec![] }.into();
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        let deposit = T::ProposalDeposit::get();
        T::Currency::make_free_balance_be(&caller, deposit * 10u32.into());

        Pallet::<T>::submit_proposal(
            RawOrigin::Signed(caller).into(),
            Box::new(call),
            title,
            description,
        )
        .unwrap();

        #[extrinsic_call]
        _(RawOrigin::Root, 0u32);

        assert!(Pallet::<T>::proposals(0).is_none());
    }

    #[benchmark]
    fn finalize_proposal() {
        let caller: T::AccountId = whitelisted_caller();
        let voter: T::AccountId = account("voter", 0, 0);

        // Setup proposal
        let call = frame_system::Call::<T>::remark { remark: vec![] }.into();
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        let deposit = T::ProposalDeposit::get();
        T::Currency::make_free_balance_be(&caller, deposit * 100u32.into());
        T::Currency::make_free_balance_be(&voter, deposit * 100u32.into());

        Pallet::<T>::submit_proposal(
            RawOrigin::Signed(caller.clone()).into(),
            Box::new(call),
            title,
            description,
        )
        .unwrap();

        // Vote
        Pallet::<T>::vote(
            RawOrigin::Signed(voter).into(),
            0,
            VoteDirection::Aye,
            deposit * 50u32.into(),
            Conviction::Locked1x,
        )
        .unwrap();

        // Advance past voting period
        let voting_end = Pallet::<T>::proposals(0).unwrap().voting_end;
        frame_system::Pallet::<T>::set_block_number(voting_end + 1u32.into());

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), 0u32);
    }

    #[benchmark]
    fn unlock() {
        let caller: T::AccountId = whitelisted_caller();

        T::Currency::make_free_balance_be(&caller, T::ProposalDeposit::get() * 10u32.into());

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), caller);
    }

    #[benchmark]
    fn update_config() {
        #[extrinsic_call]
        _(
            RawOrigin::Root,
            Some(sp_runtime::Percent::from_percent(20)),
            Some(sp_runtime::Percent::from_percent(60)),
            None,
            None,
        );
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
