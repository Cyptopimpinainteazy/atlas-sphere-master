//! Tests for the Agent Memory pallet.

use crate::{mock::*, EntryType, Error, Event};
use frame_support::{assert_noop, assert_ok, BoundedVec};

// ============================================================================
// Initialization Tests
// ============================================================================

#[test]
fn initialize_memory_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0, // agent_id
            OPERATOR1,
        ));

        assert!(AgentMemory::agent_controller(0).is_some());
        assert!(AgentMemory::agent_operator(0).is_some());
        assert_eq!(AgentMemory::current_chunk(0), 0);

        System::assert_has_event(RuntimeEvent::AgentMemory(Event::MemoryInitialized {
            agent_id: 0,
            controller: ALICE,
            operator: OPERATOR1,
        }));
    });
}

// ============================================================================
// Append Entry Tests
// ============================================================================

#[test]
fn append_entry_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        let content: BoundedVec<_, _> = b"{\"observation\":\"test\"}".to_vec().try_into().unwrap();

        assert_ok!(AgentMemory::append_entry(
            RuntimeOrigin::signed(OPERATOR1),
            0,
            EntryType::Observation,
            content,
            None,
        ));

        assert_eq!(AgentMemory::entry_count(0), 1);

        let chunk = AgentMemory::memory_chunks(0, 0).unwrap();
        assert_eq!(chunk.entries.len(), 1);
        assert_eq!(chunk.entries[0].entry_type, EntryType::Observation);
    });
}

#[test]
fn controller_can_append() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        // Controller should also be able to append
        assert_ok!(AgentMemory::append_entry(
            RuntimeOrigin::signed(ALICE),
            0,
            EntryType::Action,
            content,
            None,
        ));
    });
}

#[test]
fn unauthorized_cannot_append() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        // BOB is not controller or operator
        assert_noop!(
            AgentMemory::append_entry(
                RuntimeOrigin::signed(BOB),
                0,
                EntryType::Action,
                content,
                None,
            ),
            Error::<Test>::WritePermissionDenied
        );
    });
}

#[test]
fn append_with_metadata_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        let content: BoundedVec<_, _> = b"{\"action\":\"trade\"}".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> =
            b"{\"source\":\"market_data\"}".to_vec().try_into().unwrap();

        assert_ok!(AgentMemory::append_entry(
            RuntimeOrigin::signed(OPERATOR1),
            0,
            EntryType::Action,
            content,
            Some(metadata),
        ));

        let chunk = AgentMemory::memory_chunks(0, 0).unwrap();
        assert!(chunk.entries[0].metadata.is_some());
    });
}

// ============================================================================
// Batch Append Tests
// ============================================================================

#[test]
fn append_batch_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        let entries = vec![
            (EntryType::Observation, b"{}".to_vec().try_into().unwrap()),
            (
                EntryType::Thought,
                b"{\"thought\":1}".to_vec().try_into().unwrap(),
            ),
            (
                EntryType::Action,
                b"{\"action\":1}".to_vec().try_into().unwrap(),
            ),
        ];

        assert_ok!(AgentMemory::append_batch(
            RuntimeOrigin::signed(OPERATOR1),
            0,
            entries,
        ));

        assert_eq!(AgentMemory::entry_count(0), 3);
    });
}

// ============================================================================
// Chunk Management Tests
// ============================================================================

#[test]
fn chunk_finalizes_when_full() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        // Fill first chunk (MaxEntriesPerChunk = 100)
        for _ in 0..100 {
            let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
            assert_ok!(AgentMemory::append_entry(
                RuntimeOrigin::signed(OPERATOR1),
                0,
                EntryType::Observation,
                content,
                None,
            ));
        }

        // First chunk should have 100 entries but not be finalized yet
        let chunk0 = AgentMemory::memory_chunks(0, 0).unwrap();
        assert_eq!(chunk0.entries.len(), 100);
        assert!(!chunk0.finalized);

        // Add one more entry - this should finalize chunk0 and create chunk1
        let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        assert_ok!(AgentMemory::append_entry(
            RuntimeOrigin::signed(OPERATOR1),
            0,
            EntryType::Observation,
            content,
            None,
        ));

        // Now first chunk should be finalized
        let chunk0 = AgentMemory::memory_chunks(0, 0).unwrap();
        assert!(chunk0.finalized);

        // Should be in new chunk
        assert_eq!(AgentMemory::current_chunk(0), 1);
        let chunk1 = AgentMemory::memory_chunks(0, 1).unwrap();
        assert_eq!(chunk1.entries.len(), 1);
    });
}

// ============================================================================
// Permission Tests
// ============================================================================

#[test]
fn update_permissions_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        assert_ok!(AgentMemory::update_permissions(
            RuntimeOrigin::signed(ALICE),
            0,
            true,          // public read
            vec![BOB],     // allowed readers
            vec![CHARLIE], // allowed writers
        ));

        let perms = AgentMemory::permissions(0);
        assert!(perms.can_public_read);
        assert!(perms.allowed_readers.contains(&BOB));
        assert!(perms.allowed_writers.contains(&CHARLIE));
    });
}

#[test]
fn allowed_writer_can_append() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        // Add BOB as allowed writer
        assert_ok!(AgentMemory::update_permissions(
            RuntimeOrigin::signed(ALICE),
            0,
            false,
            vec![],
            vec![BOB],
        ));

        let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        // BOB should now be able to append
        assert_ok!(AgentMemory::append_entry(
            RuntimeOrigin::signed(BOB),
            0,
            EntryType::Action,
            content,
            None,
        ));
    });
}

#[test]
fn only_controller_can_update_permissions() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        // BOB is not controller
        assert_noop!(
            AgentMemory::update_permissions(RuntimeOrigin::signed(BOB), 0, true, vec![], vec![],),
            Error::<Test>::NotController
        );
    });
}

// ============================================================================
// Storage Deposit Tests
// ============================================================================

#[test]
fn storage_deposit_increases_on_append() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        let initial_deposit = AgentMemory::storage_deposit(0);

        let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        assert_ok!(AgentMemory::append_entry(
            RuntimeOrigin::signed(OPERATOR1),
            0,
            EntryType::Observation,
            content,
            None,
        ));

        assert!(AgentMemory::storage_deposit(0) > initial_deposit);
    });
}

#[test]
fn increase_deposit_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        let initial_deposit = AgentMemory::storage_deposit(0);
        let initial_balance = Balances::free_balance(ALICE);

        assert_ok!(AgentMemory::increase_deposit(
            RuntimeOrigin::signed(ALICE),
            0,
            10_000,
        ));

        assert_eq!(AgentMemory::storage_deposit(0), initial_deposit + 10_000);
        assert_eq!(Balances::free_balance(ALICE), initial_balance - 10_000);
    });
}

// ============================================================================
// Prune Tests
// ============================================================================

#[test]
fn prune_memory_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        // Add some entries
        for _ in 0..10 {
            let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
            assert_ok!(AgentMemory::append_entry(
                RuntimeOrigin::signed(OPERATOR1),
                0,
                EntryType::Observation,
                content,
                None,
            ));
        }

        let initial_storage = AgentMemory::storage_used(0);

        // Advance past TTL (DefaultTtl = 10000)
        run_to_block(10002);

        // Prune
        assert_ok!(AgentMemory::prune_memory(RuntimeOrigin::root(), 0, 0));

        // Storage should be reduced
        assert!(AgentMemory::storage_used(0) < initial_storage);
    });
}

// ============================================================================
// Entry Type Tests
// ============================================================================

#[test]
fn all_entry_types_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        let types = vec![
            EntryType::Observation,
            EntryType::Action,
            EntryType::Result,
            EntryType::Thought,
            EntryType::Goal,
            EntryType::Plan,
            EntryType::Error,
            EntryType::Checkpoint,
            EntryType::Delta,
            EntryType::Custom,
        ];

        for entry_type in types {
            let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
            assert_ok!(AgentMemory::append_entry(
                RuntimeOrigin::signed(OPERATOR1),
                0,
                entry_type,
                content,
                None,
            ));
        }

        assert_eq!(AgentMemory::entry_count(0), 10);
    });
}

// ============================================================================
// JSONL Output Tests
// ============================================================================

#[test]
fn get_memory_jsonl_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        // Add entries
        for i in 0..5 {
            let content: BoundedVec<_, _> =
                format!("{{\"id\":{}}}", i).into_bytes().try_into().unwrap();
            assert_ok!(AgentMemory::append_entry(
                RuntimeOrigin::signed(OPERATOR1),
                0,
                EntryType::Observation,
                content,
                None,
            ));
        }

        // Get JSONL entries
        let entries = AgentMemory::get_memory_jsonl(0, 0, 10);
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].id, 0);
        assert_eq!(entries[4].id, 4);
    });
}

#[test]
fn get_memory_jsonl_pagination_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        // Add 10 entries
        for i in 0..10 {
            let content: BoundedVec<_, _> =
                format!("{{\"id\":{}}}", i).into_bytes().try_into().unwrap();
            assert_ok!(AgentMemory::append_entry(
                RuntimeOrigin::signed(OPERATOR1),
                0,
                EntryType::Observation,
                content,
                None,
            ));
        }

        // Get with offset
        let entries = AgentMemory::get_memory_jsonl(0, 5, 3);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, 5);
        assert_eq!(entries[2].id, 7);
    });
}

// ============================================================================
// Summary Tests
// ============================================================================

#[test]
fn get_memory_summary_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentMemory::initialize_memory(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR1,
        ));

        // Add some entries
        for _ in 0..5 {
            let content: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
            assert_ok!(AgentMemory::append_entry(
                RuntimeOrigin::signed(OPERATOR1),
                0,
                EntryType::Observation,
                content,
                None,
            ));
        }

        let summary = AgentMemory::get_memory_summary(0);
        assert_eq!(summary.agent_id, 0);
        assert_eq!(summary.total_entries, 5);
        assert_eq!(summary.total_chunks, 1);
        assert!(summary.storage_used > 0);
    });
}
