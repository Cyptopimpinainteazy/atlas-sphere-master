//! Benchmarks for pallet-swarm-evolution

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	spawn_agent {
		let caller = whitelisted_caller();
		let genome = vec![0u8; 512];
	}: _(RawOrigin::Signed(caller), genome, 5000)

	mutate_agent {
		let caller = whitelisted_caller();
		// Setup: create an agent first
		let agent_id = 1u64;
		let genome = vec![0u8; 512];
	}: evolve_agent(RawOrigin::Signed(caller), agent_id, genome, 7000)

	terminate_agent {
		let caller = whitelisted_caller();
		let agent_id = 1u64;
	}: _(RawOrigin::Signed(caller), agent_id)

	evolve_population {
	}: _(RawOrigin::Signed(whitelisted_caller()))

	update_config {
	}: _(RawOrigin::Root, 30u8, 1000u32, 10u32)

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
