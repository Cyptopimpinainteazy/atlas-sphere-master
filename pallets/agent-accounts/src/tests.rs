//! Tests for the Agent Accounts pallet.

use crate::{mock::*, ActionType, AgentPermissions, AgentStatus, Error, Event};
use frame_support::{assert_noop, assert_ok, BoundedVec};

// ============================================================================
// Registration Tests
// ============================================================================

#[test]
fn register_agent_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"TestAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name.clone(),
            metadata,
        ));

        let agent = AgentAccounts::agents(0).unwrap();
        assert_eq!(agent.controller, ALICE);
        assert_eq!(agent.operator, OPERATOR1);
        assert_eq!(agent.name, name);
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(agent.reputation, 100);

        assert_eq!(AgentAccounts::total_agents(), 1);
        assert_eq!(AgentAccounts::active_agents(), 1);

        System::assert_has_event(RuntimeEvent::AgentAccounts(Event::AgentRegistered {
            agent_id: 0,
            controller: ALICE,
            operator: OPERATOR1,
        }));
    });
}

#[test]
fn register_agent_reserves_deposit() {
    new_test_ext().execute_with(|| {
        let initial_balance = Balances::free_balance(ALICE);
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_eq!(Balances::reserved_balance(ALICE), 1000);
        assert_eq!(Balances::free_balance(ALICE), initial_balance - 1000);
    });
}

#[test]
fn cannot_register_with_same_operator() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name.clone(),
            metadata.clone(),
        ));

        // Try to register another agent with same operator
        assert_noop!(
            AgentAccounts::register_agent(RuntimeOrigin::signed(BOB), OPERATOR1, name, metadata,),
            Error::<Test>::OperatorAlreadyAssigned
        );
    });
}

#[test]
fn cannot_exceed_max_agents_per_controller() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        // Register max agents (10)
        for i in 0..10u64 {
            assert_ok!(AgentAccounts::register_agent(
                RuntimeOrigin::signed(ALICE),
                100 + i, // Different operators
                name.clone(),
                metadata.clone(),
            ));
        }

        // 11th should fail
        assert_noop!(
            AgentAccounts::register_agent(RuntimeOrigin::signed(ALICE), 200, name, metadata,),
            Error::<Test>::TooManyAgents
        );
    });
}

// ============================================================================
// Operator Management Tests
// ============================================================================

#[test]
fn update_operator_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_ok!(AgentAccounts::update_operator(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR2,
        ));

        let agent = AgentAccounts::agents(0).unwrap();
        assert_eq!(agent.operator, OPERATOR2);
        assert!(AgentAccounts::operator_agent(OPERATOR2).is_some());
        assert!(AgentAccounts::operator_agent(OPERATOR1).is_none());
    });
}

#[test]
fn only_controller_can_update_operator() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_noop!(
            AgentAccounts::update_operator(
                RuntimeOrigin::signed(BOB), // Not controller
                0,
                OPERATOR2,
            ),
            Error::<Test>::NotController
        );
    });
}

// ============================================================================
// Permissions Tests
// ============================================================================

#[test]
fn update_permissions_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        let new_perms = AgentPermissions {
            can_deploy: true,
            can_stake: true,
            can_vote: true,
            can_trade: true,
            can_transfer: true,
            can_call_contracts: true,
        };

        assert_ok!(AgentAccounts::update_permissions(
            RuntimeOrigin::signed(ALICE),
            0,
            new_perms.clone(),
        ));

        let perms = AgentAccounts::permissions(0);
        assert!(perms.can_deploy);
        assert!(perms.can_stake);
        assert!(perms.can_vote);
    });
}

#[test]
fn default_permissions_are_restrictive() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        let perms = AgentAccounts::permissions(0);
        assert!(!perms.can_deploy); // Default false
        assert!(!perms.can_stake); // Default false
        assert!(!perms.can_vote); // Default false
        assert!(perms.can_trade); // Default true
        assert!(perms.can_transfer); // Default true
    });
}

// ============================================================================
// Quota Tests
// ============================================================================

#[test]
fn update_quota_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_ok!(AgentAccounts::update_quota(
            RuntimeOrigin::root(),
            0,
            2_000_000,   // gas per block
            1_000_000,   // compute per block
            200_000_000, // gas per epoch
            100_000_000, // compute per epoch
        ));

        let quota = AgentAccounts::quotas(0).unwrap();
        assert_eq!(quota.gas_per_block, 2_000_000);
        assert_eq!(quota.compute_per_block, 1_000_000);
    });
}

// ============================================================================
// Resource Consumption Tests
// ============================================================================

#[test]
fn record_consumption_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_ok!(AgentAccounts::record_consumption(
            RuntimeOrigin::root(),
            0,
            100_000, // gas
            50_000,  // compute
        ));

        let activity = AgentAccounts::activity(0);
        assert_eq!(activity.gas_used_block, 100_000);
        assert_eq!(activity.compute_used_block, 50_000);
        assert_eq!(activity.total_actions, 1);
    });
}

#[test]
fn consumption_exceeding_block_quota_fails() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        // Default gas per block is 1_000_000, try to use more
        assert_noop!(
            AgentAccounts::record_consumption(
                RuntimeOrigin::root(),
                0,
                2_000_000, // Exceeds quota
                0,
            ),
            Error::<Test>::QuotaExceeded
        );
    });
}

// ============================================================================
// Status Transition Tests
// ============================================================================

#[test]
fn suspend_agent_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let reason: BoundedVec<_, _> = b"Violation".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_ok!(AgentAccounts::suspend_agent(
            RuntimeOrigin::root(),
            0,
            reason
        ));

        let agent = AgentAccounts::agents(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Suspended);
        assert_eq!(AgentAccounts::active_agents(), 0);
    });
}

#[test]
fn reactivate_agent_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let reason: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_ok!(AgentAccounts::suspend_agent(
            RuntimeOrigin::root(),
            0,
            reason
        ));
        assert_ok!(AgentAccounts::reactivate_agent(RuntimeOrigin::root(), 0));

        let agent = AgentAccounts::agents(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(AgentAccounts::active_agents(), 1);
    });
}

#[test]
fn terminate_agent_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let initial_balance = Balances::free_balance(ALICE);

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_ok!(AgentAccounts::terminate_agent(
            RuntimeOrigin::signed(ALICE),
            0
        ));

        // Deposit returned
        assert_eq!(Balances::free_balance(ALICE), initial_balance);

        // Agent marked terminated
        let agent = AgentAccounts::agents(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Terminated);

        // Operator mapping removed
        assert!(AgentAccounts::operator_agent(OPERATOR1).is_none());

        // Quotas/Permissions cleaned
        assert!(AgentAccounts::quotas(0).is_none());
    });
}

#[test]
fn suspended_agent_cannot_consume_resources() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let reason: BoundedVec<_, _> = b"Test".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_ok!(AgentAccounts::suspend_agent(
            RuntimeOrigin::root(),
            0,
            reason
        ));

        assert_noop!(
            AgentAccounts::record_consumption(RuntimeOrigin::root(), 0, 1000, 1000),
            Error::<Test>::AgentNotActive
        );
    });
}

// ============================================================================
// Reputation Tests
// ============================================================================

#[test]
fn update_reputation_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        // Increase reputation
        assert_ok!(AgentAccounts::update_reputation(
            RuntimeOrigin::root(),
            0,
            50
        ));
        assert_eq!(AgentAccounts::agents(0).unwrap().reputation, 150);

        // Decrease reputation
        assert_ok!(AgentAccounts::update_reputation(
            RuntimeOrigin::root(),
            0,
            -30
        ));
        assert_eq!(AgentAccounts::agents(0).unwrap().reputation, 120);
    });
}

#[test]
fn reputation_is_clamped() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        // Try to exceed max (200)
        assert_ok!(AgentAccounts::update_reputation(
            RuntimeOrigin::root(),
            0,
            200
        ));
        assert_eq!(AgentAccounts::agents(0).unwrap().reputation, 200);

        // Try to go below 0
        assert_ok!(AgentAccounts::update_reputation(
            RuntimeOrigin::root(),
            0,
            -500
        ));
        assert_eq!(AgentAccounts::agents(0).unwrap().reputation, 0);
    });
}

// ============================================================================
// Event Emission Tests
// ============================================================================

#[test]
fn emit_action_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let action_data: BoundedVec<_, _> = b"{\"trade\":\"BTC/ETH\"}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        assert_ok!(AgentAccounts::emit_action(
            RuntimeOrigin::signed(OPERATOR1),
            ActionType::Trade,
            action_data.clone(),
        ));

        System::assert_has_event(RuntimeEvent::AgentAccounts(Event::AgentAction {
            agent_id: 0,
            action_type: ActionType::Trade,
            data: action_data,
        }));
    });
}

#[test]
fn only_operator_can_emit_action() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let action_data: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        // BOB is not an operator
        assert_noop!(
            AgentAccounts::emit_action(RuntimeOrigin::signed(BOB), ActionType::Trade, action_data,),
            Error::<Test>::NotOperator
        );
    });
}

// ============================================================================
// Epoch Tests
// ============================================================================

#[test]
fn epoch_advances_on_schedule() {
    new_test_ext().execute_with(|| {
        assert_eq!(AgentAccounts::current_epoch(), 0);

        // Advance 100 blocks (BlocksPerEpoch)
        run_to_block(101);

        assert_eq!(AgentAccounts::current_epoch(), 1);
    });
}

#[test]
fn epoch_activity_resets() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentAccounts::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
        ));

        // Record some consumption
        assert_ok!(AgentAccounts::record_consumption(
            RuntimeOrigin::root(),
            0,
            100_000,
            50_000
        ));
        assert_eq!(AgentAccounts::activity(0).gas_used_epoch, 100_000);

        // Advance to new epoch
        run_to_block(101);

        // Epoch activity should be reset
        assert_eq!(AgentAccounts::activity(0).gas_used_epoch, 0);
    });
}
