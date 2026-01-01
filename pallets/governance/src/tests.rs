//! Unit tests for the governance pallet.

use crate::{mock::*, types::*, *};
use frame_support::{assert_noop, assert_ok, traits::ConstU32, BoundedVec};
use sp_runtime::Percent;

// ============================================================================
// Proposal Tests
// ============================================================================

#[test]
fn submit_proposal_works() {
    new_test_ext().execute_with(|| {
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test Proposal".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> =
            b"A test proposal".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        assert_eq!(Governance::proposal_count(), 1);

        let proposal = Governance::proposals(0).unwrap();
        assert_eq!(proposal.proposer, account(1));
        assert_eq!(proposal.status, ProposalStatus::Voting);
    });
}

#[test]
fn submit_proposal_reserves_deposit() {
    new_test_ext().execute_with(|| {
        let initial_balance = Balances::free_balance(account(1));

        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        assert_eq!(
            Balances::reserved_balance(account(1)),
            100 // ProposalDeposit
        );
        assert_eq!(Balances::free_balance(account(1)), initial_balance - 100);
    });
}

// ============================================================================
// Voting Tests
// ============================================================================

#[test]
fn vote_works() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Vote
        assert_ok!(Governance::vote(
            RuntimeOrigin::signed(account(2)),
            0,
            VoteDirection::Aye,
            1000,
            Conviction::Locked1x,
        ));

        let tally = Governance::proposal_votes(0);
        assert_eq!(tally.ayes, 1000);
        assert_eq!(tally.aye_voters, 1);
        assert_eq!(tally.turnout, 1000);
    });
}

#[test]
fn vote_with_conviction_multiplies_power() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Vote with 3x conviction
        assert_ok!(Governance::vote(
            RuntimeOrigin::signed(account(2)),
            0,
            VoteDirection::Aye,
            1000,
            Conviction::Locked3x,
        ));

        let tally = Governance::proposal_votes(0);
        assert_eq!(tally.ayes, 3000); // 3x voting power
        assert_eq!(tally.turnout, 1000); // Raw balance
    });
}

#[test]
fn vote_fails_after_voting_period() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Advance past voting period
        run_to_block(102);

        // Try to vote
        assert_noop!(
            Governance::vote(
                RuntimeOrigin::signed(account(2)),
                0,
                VoteDirection::Aye,
                1000,
                Conviction::None,
            ),
            Error::<Test>::VotingEnded
        );
    });
}

#[test]
fn change_vote_works() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Initial vote
        assert_ok!(Governance::vote(
            RuntimeOrigin::signed(account(2)),
            0,
            VoteDirection::Aye,
            1000,
            Conviction::Locked1x,
        ));

        let tally = Governance::proposal_votes(0);
        assert_eq!(tally.ayes, 1000);
        assert_eq!(tally.nays, 0);

        // Change vote
        assert_ok!(Governance::vote(
            RuntimeOrigin::signed(account(2)),
            0,
            VoteDirection::Nay,
            1000,
            Conviction::Locked1x,
        ));

        let tally = Governance::proposal_votes(0);
        assert_eq!(tally.ayes, 0);
        assert_eq!(tally.nays, 1000);
    });
}

// ============================================================================
// Delegation Tests
// ============================================================================

#[test]
fn delegation_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Governance::delegate(
            RuntimeOrigin::signed(account(2)),
            account(1),
            Conviction::Locked1x,
        ));

        let delegation = Governance::delegations(account(2)).unwrap();
        assert_eq!(delegation.target, account(1));
        assert_eq!(delegation.conviction, Conviction::Locked1x);

        let delegators = Governance::delegators(account(1));
        assert!(delegators.contains(&account(2)));
    });
}

#[test]
fn cannot_delegate_to_self() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Governance::delegate(
                RuntimeOrigin::signed(account(1)),
                account(1),
                Conviction::Locked1x,
            ),
            Error::<Test>::SelfDelegation
        );
    });
}

#[test]
fn undelegation_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Governance::delegate(
            RuntimeOrigin::signed(account(2)),
            account(1),
            Conviction::Locked1x,
        ));

        assert_ok!(Governance::undelegate(RuntimeOrigin::signed(account(2))));

        assert!(Governance::delegations(account(2)).is_none());
        assert!(!Governance::delegators(account(1)).contains(&account(2)));
    });
}

#[test]
fn delegated_account_cannot_vote() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Delegate
        assert_ok!(Governance::delegate(
            RuntimeOrigin::signed(account(2)),
            account(1),
            Conviction::Locked1x,
        ));

        // Try to vote
        assert_noop!(
            Governance::vote(
                RuntimeOrigin::signed(account(2)),
                0,
                VoteDirection::Aye,
                1000,
                Conviction::None,
            ),
            Error::<Test>::AlreadyDelegated
        );
    });
}

// ============================================================================
// Proposal Finalization Tests
// ============================================================================

#[test]
fn finalize_approved_proposal() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Vote to exceed quorum (10% of 50000 = 5000) and approval (50%)
        assert_ok!(Governance::vote(
            RuntimeOrigin::signed(account(2)),
            0,
            VoteDirection::Aye,
            6000,
            Conviction::Locked1x,
        ));

        // Advance past voting period
        run_to_block(102);

        // Finalize
        assert_ok!(Governance::finalize_proposal(
            RuntimeOrigin::signed(account(3)),
            0
        ));

        let proposal = Governance::proposals(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
    });
}

#[test]
fn finalize_rejected_proposal() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Vote Nay with enough for quorum
        assert_ok!(Governance::vote(
            RuntimeOrigin::signed(account(2)),
            0,
            VoteDirection::Nay,
            6000,
            Conviction::Locked1x,
        ));

        // Advance past voting period
        run_to_block(102);

        // Finalize
        assert_ok!(Governance::finalize_proposal(
            RuntimeOrigin::signed(account(3)),
            0
        ));

        let proposal = Governance::proposals(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Rejected);
    });
}

#[test]
fn cannot_finalize_during_voting() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Try to finalize immediately
        assert_noop!(
            Governance::finalize_proposal(RuntimeOrigin::signed(account(3)), 0),
            Error::<Test>::VotingNotEnded
        );
    });
}

// ============================================================================
// Fast Track Tests
// ============================================================================

#[test]
fn fast_track_works() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Fast track (root origin)
        assert_ok!(Governance::fast_track(RuntimeOrigin::root(), 0, 10));

        let proposal = Governance::proposals(0).unwrap();
        assert_eq!(proposal.voting_end, 11); // current block (1) + 10
    });
}

// ============================================================================
// Cancel Tests
// ============================================================================

#[test]
fn cancel_proposal_works() {
    new_test_ext().execute_with(|| {
        // Submit proposal
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let title: BoundedVec<u8, ConstU32<256>> = b"Test".to_vec().try_into().unwrap();
        let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Governance::submit_proposal(
            RuntimeOrigin::signed(account(1)),
            Box::new(call),
            title,
            description,
        ));

        // Cancel (root origin)
        assert_ok!(Governance::cancel_proposal(RuntimeOrigin::root(), 0));

        assert!(Governance::proposals(0).is_none());
    });
}

// ============================================================================
// Governance Snapshot Tests
// ============================================================================

#[test]
fn governance_snapshot_works() {
    new_test_ext().execute_with(|| {
        // Submit proposals
        for i in 0..3 {
            let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![i] });
            let title: BoundedVec<u8, ConstU32<256>> =
                format!("Proposal {}", i).into_bytes().try_into().unwrap();
            let description: BoundedVec<u8, ConstU32<4096>> = b"Test".to_vec().try_into().unwrap();

            assert_ok!(Governance::submit_proposal(
                RuntimeOrigin::signed(account(1)),
                Box::new(call),
                title,
                description,
            ));
        }

        let snapshot = Governance::get_governance_snapshot();
        assert_eq!(snapshot.proposal_count, 3);
        assert_eq!(snapshot.active_proposals.len(), 3);
        assert_eq!(snapshot.config.quorum, Percent::from_percent(10));
    });
}
