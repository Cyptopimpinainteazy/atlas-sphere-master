//! Tests for the Treasury pallet.

use crate::{mock::*, Error, Event, ProposalStatus, RiskLevel, SpendingTrack};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use sp_runtime::Percent;

// ============================================================================
// Proposal Tests
// ============================================================================

#[test]
fn submit_proposal_works() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Test proposal".to_vec().try_into().unwrap();

        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            500, // Small spend
            description,
        ));

        let proposal = Treasury::proposals(0).unwrap();
        assert_eq!(proposal.proposer, ALICE);
        assert_eq!(proposal.beneficiary, BOB);
        assert_eq!(proposal.amount, 500);
        assert_eq!(proposal.track, SpendingTrack::Small);
        assert_eq!(proposal.status, ProposalStatus::Pending);

        System::assert_has_event(RuntimeEvent::Treasury(Event::ProposalSubmitted {
            proposal_id: 0,
            proposer: ALICE,
            beneficiary: BOB,
            amount: 500,
            track: SpendingTrack::Small,
        }));
    });
}

#[test]
fn submit_proposal_reserves_bond() {
    new_test_ext().execute_with(|| {
        let initial_balance = Balances::free_balance(ALICE);
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            500,
            description,
        ));

        // Bond should be 5% of 500 = 25, but minimum is 10
        let expected_bond = 25u128;
        assert!(Balances::reserved_balance(ALICE) >= 10);
        assert_eq!(
            Balances::free_balance(ALICE),
            initial_balance - expected_bond
        );
    });
}

#[test]
fn determines_correct_spending_track() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        // Small: <= 1000
        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            1000,
            description.clone(),
        ));
        assert_eq!(Treasury::proposals(0).unwrap().track, SpendingTrack::Small);

        // Medium: <= 10000
        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            5000,
            description.clone(),
        ));
        assert_eq!(Treasury::proposals(1).unwrap().track, SpendingTrack::Medium);

        // Large: <= 100000
        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            50000,
            description.clone(),
        ));
        assert_eq!(Treasury::proposals(2).unwrap().track, SpendingTrack::Large);

        // Critical: > 100000
        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            200000,
            description,
        ));
        assert_eq!(
            Treasury::proposals(3).unwrap().track,
            SpendingTrack::Critical
        );
    });
}

#[test]
fn cannot_submit_zero_amount() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_noop!(
            Treasury::submit_proposal(RuntimeOrigin::signed(ALICE), BOB, 0, description,),
            Error::<Test>::ZeroAmount
        );
    });
}

// ============================================================================
// Multi-sig Approval Tests
// ============================================================================

#[test]
fn approve_proposal_works() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            500,
            description,
        ));

        assert_ok!(Treasury::approve_proposal(RuntimeOrigin::signed(ALICE), 0));

        let approvals = Treasury::approvals(0);
        assert_eq!(approvals.len(), 1);
        assert!(approvals.contains(&ALICE));
    });
}

#[test]
fn only_signers_can_approve() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            500,
            description,
        ));

        // DAVE is not a signer
        assert_noop!(
            Treasury::approve_proposal(RuntimeOrigin::signed(DAVE), 0),
            Error::<Test>::NotSigner
        );
    });
}

#[test]
fn cannot_double_approve() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        // Use a large amount to get into a higher track requiring more approvals
        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            8000, // Large track needs 67% = 2 approvals
            description,
        ));

        assert_ok!(Treasury::approve_proposal(RuntimeOrigin::signed(ALICE), 0));
        assert_noop!(
            Treasury::approve_proposal(RuntimeOrigin::signed(ALICE), 0),
            Error::<Test>::AlreadyApproved
        );
    });
}

#[test]
fn proposal_auto_executes_on_threshold() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();
        let initial_bob_balance = Balances::free_balance(BOB);

        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            500,
            description,
        ));

        // Small track with 3 signers needs ~33% = 1 approval
        assert_ok!(Treasury::approve_proposal(RuntimeOrigin::signed(ALICE), 0));

        // Proposal should be executed
        let proposal = Treasury::proposals(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
        assert_eq!(Balances::free_balance(BOB), initial_bob_balance + 500);
    });
}

// ============================================================================
// Recurring Payment Tests
// ============================================================================

#[test]
fn create_recurring_payment_works() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Salary".to_vec().try_into().unwrap();

        assert_ok!(Treasury::create_recurring_payment(
            RuntimeOrigin::root(),
            BOB,
            1000,
            100,      // Every 100 blocks
            Some(12), // 12 payments total
            description,
        ));

        let payment = Treasury::recurring_payments(0).unwrap();
        assert_eq!(payment.beneficiary, BOB);
        assert_eq!(payment.amount, 1000);
        assert_eq!(payment.interval, 100);
        assert_eq!(payment.total_payments, Some(12));
        assert!(payment.active);
    });
}

#[test]
fn recurring_payment_executes_on_schedule() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Salary".to_vec().try_into().unwrap();
        let initial_balance = Balances::free_balance(BOB);

        assert_ok!(Treasury::create_recurring_payment(
            RuntimeOrigin::root(),
            BOB,
            1000,
            10,   // Every 10 blocks
            None, // Unlimited
            description,
        ));

        // Advance to payment time
        run_to_block(12);

        // Payment should have been made
        assert_eq!(Balances::free_balance(BOB), initial_balance + 1000);

        let payment = Treasury::recurring_payments(0).unwrap();
        assert_eq!(payment.payments_made, 1);
    });
}

#[test]
fn cancel_recurring_payment_works() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Grant".to_vec().try_into().unwrap();

        assert_ok!(Treasury::create_recurring_payment(
            RuntimeOrigin::root(),
            BOB,
            500,
            50,
            None,
            description,
        ));

        assert_ok!(Treasury::cancel_recurring_payment(RuntimeOrigin::root(), 0));

        let payment = Treasury::recurring_payments(0).unwrap();
        assert!(!payment.active);
    });
}

// ============================================================================
// Yield Strategy Tests
// ============================================================================

#[test]
fn register_yield_strategy_works() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"DeFi Strategy".to_vec().try_into().unwrap();

        assert_ok!(Treasury::register_yield_strategy(
            RuntimeOrigin::root(),
            DAVE,                     // Agent
            50_000,                   // Max allocation
            Percent::from_percent(5), // 5% min return
            RiskLevel::Medium,
            description,
        ));

        let strategy = Treasury::yield_strategies(0).unwrap();
        assert_eq!(strategy.agent, DAVE);
        assert_eq!(strategy.max_allocation, 50_000);
        assert_eq!(strategy.risk_level, RiskLevel::Medium);
        assert!(strategy.active);
    });
}

#[test]
fn only_agent_can_execute_strategy() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Strategy".to_vec().try_into().unwrap();

        assert_ok!(Treasury::register_yield_strategy(
            RuntimeOrigin::root(),
            DAVE,
            50_000,
            Percent::from_percent(5),
            RiskLevel::Low,
            description,
        ));

        // EVE is not the agent
        assert_noop!(
            Treasury::execute_yield_strategy(RuntimeOrigin::signed(EVE), 0, 10_000, 500,),
            Error::<Test>::NotStrategyAgent
        );

        // DAVE is the agent
        assert_ok!(Treasury::execute_yield_strategy(
            RuntimeOrigin::signed(DAVE),
            0,
            10_000,
            500,
        ));
    });
}

#[test]
fn cannot_exceed_max_allocation() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Strategy".to_vec().try_into().unwrap();

        assert_ok!(Treasury::register_yield_strategy(
            RuntimeOrigin::root(),
            DAVE,
            50_000,
            Percent::from_percent(5),
            RiskLevel::Low,
            description,
        ));

        assert_noop!(
            Treasury::execute_yield_strategy(
                RuntimeOrigin::signed(DAVE),
                0,
                60_000, // Exceeds max
                3_000,
            ),
            Error::<Test>::AllocationExceeded
        );
    });
}

#[test]
fn deactivate_strategy_works() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Strategy".to_vec().try_into().unwrap();

        assert_ok!(Treasury::register_yield_strategy(
            RuntimeOrigin::root(),
            DAVE,
            50_000,
            Percent::from_percent(5),
            RiskLevel::Low,
            description,
        ));

        assert_ok!(Treasury::deactivate_yield_strategy(
            RuntimeOrigin::root(),
            0
        ));

        let strategy = Treasury::yield_strategies(0).unwrap();
        assert!(!strategy.active);

        // Cannot execute deactivated strategy
        assert_noop!(
            Treasury::execute_yield_strategy(RuntimeOrigin::signed(DAVE), 0, 10_000, 500,),
            Error::<Test>::StrategyNotActive
        );
    });
}

// ============================================================================
// Emergency Pause Tests
// ============================================================================

#[test]
fn pause_works() {
    new_test_ext().execute_with(|| {
        let reason: BoundedVec<_, _> = b"Security incident".to_vec().try_into().unwrap();

        assert_ok!(Treasury::pause(RuntimeOrigin::root(), reason.clone()));
        assert!(Treasury::is_paused());

        let pause_info = Treasury::pause_info().unwrap();
        assert_eq!(pause_info.reason, reason);
    });
}

#[test]
fn paused_treasury_blocks_operations() {
    new_test_ext().execute_with(|| {
        let reason: BoundedVec<_, _> = b"Emergency".to_vec().try_into().unwrap();
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Treasury::pause(RuntimeOrigin::root(), reason));

        // Cannot submit proposals when paused
        assert_noop!(
            Treasury::submit_proposal(RuntimeOrigin::signed(ALICE), BOB, 500, description,),
            Error::<Test>::TreasuryPaused
        );
    });
}

#[test]
fn unpause_works() {
    new_test_ext().execute_with(|| {
        let reason: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Treasury::pause(RuntimeOrigin::root(), reason));
        assert!(Treasury::is_paused());

        assert_ok!(Treasury::unpause(RuntimeOrigin::root()));
        assert!(!Treasury::is_paused());
        assert!(Treasury::pause_info().is_none());
    });
}

// ============================================================================
// Signer Management Tests
// ============================================================================

#[test]
fn update_signers_works() {
    new_test_ext().execute_with(|| {
        let new_signers = vec![DAVE, EVE];

        assert_ok!(Treasury::update_signers(
            RuntimeOrigin::root(),
            new_signers.clone()
        ));

        let signers = Treasury::signers();
        assert_eq!(signers.len(), 2);
        assert!(signers.contains(&DAVE));
        assert!(signers.contains(&EVE));
        assert!(!signers.contains(&ALICE)); // Removed
    });
}

// ============================================================================
// Deposit Tests
// ============================================================================

#[test]
fn deposit_works() {
    new_test_ext().execute_with(|| {
        let initial_treasury = Treasury::balance();
        let initial_alice = Balances::free_balance(ALICE);

        assert_ok!(Treasury::deposit(RuntimeOrigin::signed(ALICE), 5000));

        assert_eq!(Treasury::balance(), initial_treasury + 5000);
        assert_eq!(Balances::free_balance(ALICE), initial_alice - 5000);

        let stats = Treasury::stats();
        assert_eq!(stats.total_deposited, 5000);
    });
}

#[test]
fn cannot_deposit_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Treasury::deposit(RuntimeOrigin::signed(ALICE), 0),
            Error::<Test>::ZeroAmount
        );
    });
}

// ============================================================================
// Statistics Tests
// ============================================================================

#[test]
fn stats_update_on_execution() {
    new_test_ext().execute_with(|| {
        let description: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(Treasury::submit_proposal(
            RuntimeOrigin::signed(ALICE),
            BOB,
            500,
            description,
        ));

        assert_ok!(Treasury::approve_proposal(RuntimeOrigin::signed(ALICE), 0));

        let stats = Treasury::stats();
        assert_eq!(stats.total_spent, 500);
        assert_eq!(stats.proposals_executed, 1);
    });
}
