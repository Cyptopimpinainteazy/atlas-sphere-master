//! Benchmarks for pallet-audit-governance

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_core::H256;

benchmarks! {
    submit_proposal {
        let caller = whitelisted_caller();
    }: submit_audit_artifact(
        RawOrigin::Signed(caller),
        H256::default(),
        H256::default(),
        0u8,
        0u32,
        vec![],
        0u64
    )

    vote_on_proposal {
        let caller = whitelisted_caller();
    }: appeal_audit(RawOrigin::Signed(caller), H256::default())

    execute_proposal {
    }: _(RawOrigin::Root, whitelisted_caller(), H256::default())

    cancel_proposal {
    }: toggle_emergency_pause(RawOrigin::Root)

    update_audit_parameters {
    }: set_audit_threshold(RawOrigin::Root, 50u32)

    set_audit_threshold {
    }: set_audit_threshold(RawOrigin::Root, 50u32)

    create_audit_schedule {
    }: create_audit_schedule(RawOrigin::Root, 1000u32)

    perform_audit {
    }: _(RawOrigin::Root)

    approve_audit {
    }: _(RawOrigin::Root, H256::default())

    register_auditor {
        let auditor = whitelisted_caller();
    }: register_auditor(RawOrigin::Root, auditor)

    remove_auditor {
        let auditor = whitelisted_caller();
    }: remove_auditor(RawOrigin::Root, auditor)

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
