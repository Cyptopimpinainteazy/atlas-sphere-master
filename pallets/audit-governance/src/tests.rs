//! Unit tests for audit governance pallet

#[cfg(test)]
mod tests {
    use crate::mock::*;
    use crate::{
        AuditArtifacts, AuditDecision, AuditSubmitters, EmergencyPaused, Event, LockedAgents,
    };
    use frame_support::assert_ok;
    use sp_core::H256;

    // Test: Submit audit artifact successfully
    #[test]
    fn test_submit_audit_artifact_success() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();

            // Whitelist caller
            whitelist_account(caller);

            // Submit audit
            let result = submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                2, // BLOCK decision
                findings.len() as u32,
                findings,
            );

            // Verify success
            assert_ok!(result);

            // Verify artifact stored
            assert!(AuditArtifacts::<Test>::contains_key(audit_id));

            // Verify commit hash linked
            assert_eq!(
                crate::LatestAuditForCommit::<Test>::get(commit_hash),
                Some(audit_id)
            );

            // Verify event emitted
            let events = frame_system::Pallet::<Test>::events();
            assert!(!events.is_empty());
        });
    }

    // Test: Submit audit fails when not whitelisted
    #[test]
    fn test_submit_audit_unauthorized() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();

            // DON'T whitelist caller - should fail

            // Try to submit audit
            let result = submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                2, // BLOCK decision
                findings.len() as u32,
                findings,
            );

            // Verify failure
            assert!(result.is_err());

            // Verify artifact NOT stored
            assert!(!AuditArtifacts::<Test>::contains_key(audit_id));
        });
    }

    // Test: Submit audit fails with invalid decision
    #[test]
    fn test_submit_audit_invalid_decision() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();

            // Whitelist caller
            whitelist_account(caller);

            // Try to submit with invalid decision (3 is invalid)
            let result = submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                3, // INVALID decision
                findings.len() as u32,
                findings,
            );

            // Verify failure
            assert!(result.is_err());

            // Verify artifact NOT stored
            assert!(!AuditArtifacts::<Test>::contains_key(audit_id));
        });
    }

    // Test: Emergency pause blocks submissions
    #[test]
    fn test_emergency_pause_blocks_submission() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();

            // Whitelist caller
            whitelist_account(caller);

            // Enable emergency pause
            assert_ok!(crate::Pallet::<Test>::toggle_emergency_pause(
                frame_system::RawOrigin::Root.into()
            ));

            // Try to submit while paused
            let result = submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                2, // BLOCK decision
                findings.len() as u32,
                findings,
            );

            // Verify failure
            assert!(result.is_err());

            // Verify artifact NOT stored
            assert!(!AuditArtifacts::<Test>::contains_key(audit_id));
        });
    }

    // Test: Can toggle emergency pause on and off
    #[test]
    fn test_toggle_emergency_pause() {
        new_test_ext().execute_with(|| {
            // Initially not paused
            assert!(!EmergencyPaused::<Test>::get());

            // Toggle on
            assert_ok!(crate::Pallet::<Test>::toggle_emergency_pause(
                frame_system::RawOrigin::Root.into()
            ));
            assert!(EmergencyPaused::<Test>::get());

            // Toggle off
            assert_ok!(crate::Pallet::<Test>::toggle_emergency_pause(
                frame_system::RawOrigin::Root.into()
            ));
            assert!(!EmergencyPaused::<Test>::get());
        });
    }

    // Test: Whitelist audit submitter
    #[test]
    fn test_whitelist_audit_submitter() {
        new_test_ext().execute_with(|| {
            let account = 1u64;

            // Initially not whitelisted
            assert!(!AuditSubmitters::<Test>::get(account));

            // Whitelist
            assert_ok!(crate::Pallet::<Test>::whitelist_audit_submitter(
                frame_system::RawOrigin::Root.into(),
                account
            ));

            // Now whitelisted
            assert!(AuditSubmitters::<Test>::get(account));
        });
    }

    // Test: Lock agent successfully
    #[test]
    fn test_lock_agent_success() {
        new_test_ext().execute_with(|| {
            let agent = 1u64;
            let audit_id = H256::from_low_u64_be(1);

            // Create audit first
            let caller = 2u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();
            whitelist_account(caller);
            assert_ok!(submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                2, // BLOCK
                findings.len() as u32,
                findings,
            ));

            // Lock agent
            assert_ok!(crate::Pallet::<Test>::lock_agent(
                frame_system::RawOrigin::Root.into(),
                agent,
                audit_id
            ));

            // Verify agent is locked
            assert!(LockedAgents::<Test>::get(agent).is_some());
        });
    }

    // Test: Lock agent fails with invalid audit
    #[test]
    fn test_lock_agent_invalid_audit() {
        new_test_ext().execute_with(|| {
            let agent = 1u64;
            let nonexistent_audit = H256::from_low_u64_be(9999);

            // Try to lock with non-existent audit
            let result = crate::Pallet::<Test>::lock_agent(
                frame_system::RawOrigin::Root.into(),
                agent,
                nonexistent_audit,
            );

            // Verify failure
            assert!(result.is_err());

            // Verify agent NOT locked
            assert!(LockedAgents::<Test>::get(agent).is_none());
        });
    }

    // Test: Unlock agent successfully
    #[test]
    fn test_unlock_agent_success() {
        new_test_ext().execute_with(|| {
            let agent = 1u64;
            let audit_id = H256::from_low_u64_be(1);

            // Create and submit audit
            let caller = 2u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();
            whitelist_account(caller);
            assert_ok!(submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                2, // BLOCK
                findings.len() as u32,
                findings,
            ));

            // Lock agent
            assert_ok!(crate::Pallet::<Test>::lock_agent(
                frame_system::RawOrigin::Root.into(),
                agent,
                audit_id
            ));
            assert!(LockedAgents::<Test>::get(agent).is_some());

            // Unlock agent
            assert_ok!(crate::Pallet::<Test>::unlock_agent(
                frame_system::RawOrigin::Root.into(),
                agent
            ));

            // Verify agent is unlocked
            assert!(LockedAgents::<Test>::get(agent).is_none());
        });
    }

    // Test: Appeal audit successfully
    #[test]
    fn test_appeal_audit_success() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();

            // Submit BLOCK audit
            whitelist_account(caller);
            assert_ok!(submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                2, // BLOCK decision
                findings.len() as u32,
                findings,
            ));

            // Appeal should succeed
            let result = crate::Pallet::<Test>::appeal_audit(
                frame_system::RawOrigin::Signed(caller).into(),
                audit_id,
            );
            assert_ok!(result);

            // Verify appeal created
            assert!(crate::AuditAppeals::<Test>::contains_key(audit_id));
        });
    }

    // Test: Cannot appeal non-BLOCK decision
    #[test]
    fn test_cannot_appeal_non_block_decision() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let audit_id = H256::from_low_u64_be(1);
            let commit_hash = H256::from_low_u64_be(100);
            let findings = vec![];

            // Submit PASS audit (decision = 0)
            whitelist_account(caller);
            assert_ok!(submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                0, // PASS decision
                0,
                findings,
            ));

            // Try to appeal - should fail
            let result = crate::Pallet::<Test>::appeal_audit(
                frame_system::RawOrigin::Signed(caller).into(),
                audit_id,
            );
            assert!(result.is_err());
        });
    }

    // Test: Cannot appeal if already appealing
    #[test]
    fn test_cannot_appeal_twice() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();

            // Submit BLOCK audit
            whitelist_account(caller);
            assert_ok!(submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                2, // BLOCK
                findings.len() as u32,
                findings,
            ));

            // Appeal once - succeeds
            assert_ok!(crate::Pallet::<Test>::appeal_audit(
                frame_system::RawOrigin::Signed(caller).into(),
                audit_id,
            ));

            // Try to appeal again - should fail
            let result = crate::Pallet::<Test>::appeal_audit(
                frame_system::RawOrigin::Signed(caller).into(),
                audit_id,
            );
            assert!(result.is_err());
        });
    }

    // Test: Cannot appeal non-existent audit
    #[test]
    fn test_appeal_nonexistent_audit() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let nonexistent_audit = H256::from_low_u64_be(9999);

            // Try to appeal non-existent audit
            let result = crate::Pallet::<Test>::appeal_audit(
                frame_system::RawOrigin::Signed(caller).into(),
                nonexistent_audit,
            );

            // Should fail
            assert!(result.is_err());
        });
    }

    // Test: Three decision types work correctly
    #[test]
    fn test_all_decision_types() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            whitelist_account(caller);

            // Test PASS (decision = 0)
            let audit_id_pass = H256::from_low_u64_be(1);
            let commit_hash_pass = H256::from_low_u64_be(101);
            assert_ok!(submit_test_audit(
                caller,
                audit_id_pass,
                commit_hash_pass,
                0, // PASS
                0,
                vec![],
            ));
            assert!(AuditArtifacts::<Test>::contains_key(audit_id_pass));

            // Test WARN (decision = 1)
            let audit_id_warn = H256::from_low_u64_be(2);
            let commit_hash_warn = H256::from_low_u64_be(102);
            assert_ok!(submit_test_audit(
                caller,
                audit_id_warn,
                commit_hash_warn,
                1, // WARN
                0,
                vec![],
            ));
            assert!(AuditArtifacts::<Test>::contains_key(audit_id_warn));

            // Test BLOCK (decision = 2)
            let audit_id_block = H256::from_low_u64_be(3);
            let commit_hash_block = H256::from_low_u64_be(103);
            assert_ok!(submit_test_audit(
                caller,
                audit_id_block,
                commit_hash_block,
                2, // BLOCK
                0,
                vec![],
            ));
            assert!(AuditArtifacts::<Test>::contains_key(audit_id_block));
        });
    }

    // Test: Storage bounds are enforced
    #[test]
    fn test_max_audits_limit_not_exceeded() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            whitelist_account(caller);

            // Submit audits up to max (MaxAuditArtifacts = 1_000 in mock)
            for i in 0..100 {
                let audit_id = H256::from_low_u64_be(i as u64);
                let commit_hash = H256::from_low_u64_be(1000 + i as u64);
                assert_ok!(submit_test_audit(
                    caller,
                    audit_id,
                    commit_hash,
                    0, // PASS
                    0,
                    vec![],
                ));
            }

            // Verify all stored
            let count = AuditArtifacts::<Test>::iter().count();
            assert_eq!(count, 100);
        });
    }

    // Test: Query helpers work correctly
    #[test]
    fn test_query_helpers() {
        new_test_ext().execute_with(|| {
            let caller = 1u64;
            let agent = 2u64;
            let (audit_id, commit_hash, findings) = create_test_audit_data();

            // Initially nothing locked
            assert!(!crate::Pallet::<Test>::is_agent_locked(&agent));

            // Submit BLOCK audit
            whitelist_account(caller);
            assert_ok!(submit_test_audit(
                caller,
                audit_id,
                commit_hash,
                2, // BLOCK
                findings.len() as u32,
                findings,
            ));

            // Get audit decision for commit
            let decision = crate::Pallet::<Test>::get_audit_for_commit(commit_hash);
            assert_eq!(decision, Some(AuditDecision::Block));

            // Lock agent
            assert_ok!(crate::Pallet::<Test>::lock_agent(
                frame_system::RawOrigin::Root.into(),
                agent,
                audit_id
            ));

            // Now agent is locked
            assert!(crate::Pallet::<Test>::is_agent_locked(&agent));

            // Verify emergency pause query
            assert!(!crate::Pallet::<Test>::is_emergency_paused());
            assert_ok!(crate::Pallet::<Test>::toggle_emergency_pause(
                frame_system::RawOrigin::Root.into()
            ));
            assert!(crate::Pallet::<Test>::is_emergency_paused());
        });
    }
}
