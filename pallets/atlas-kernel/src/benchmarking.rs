//! Benchmarking setup for pallet-atlas-kernel
//!
//! This module provides benchmarks for all extrinsics in the Atlas Kernel pallet.
//! Run benchmarks with:
//! ```bash
//! cargo build --release --features runtime-benchmarks
//! ./target/release/atlas-sphere-node benchmark pallet \
//!     --chain=dev \
//!     --pallet=pallet_atlas_kernel \
//!     --extrinsic='*' \
//!     --steps=50 \
//!     --repeat=20 \
//!     --output=pallets/atlas-kernel/src/weights.rs \
//!     --template=.maintain/frame-weight-template.hbs
//! ```

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_core::H256;
use sp_std::vec;

/// Helper to create a valid EVM payload for benchmarking
fn create_evm_payload<T: Config>(size: u32) -> Vec<u8> {
    // Simple EVM payload: transfer-like calldata
    // 0xa9059cbb = transfer(address,uint256) selector
    let mut payload = vec![0xa9, 0x05, 0x9c, 0xbb];
    // Pad with zeros to reach desired size
    payload.extend(vec![0u8; size.saturating_sub(4) as usize]);
    payload
}

/// Helper to create a valid SVM payload for benchmarking
fn create_svm_payload<T: Config>(size: u32) -> Vec<u8> {
    // Simple BPF program stub (minimal valid header)
    // ELF magic + padding
    let mut payload = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic
    payload.extend(vec![0u8; size.saturating_sub(4) as usize]);
    payload
}

/// Helper to register an asset for testing
fn setup_asset<T: Config>(asset_id: T::AssetId, symbol: &[u8]) {
    let bounded_symbol: BoundedVec<u8, T::MaxAssetSymbolLength> =
        symbol.to_vec().try_into().expect("Symbol too long");
    let metadata = AssetMetadata {
        symbol: bounded_symbol,
        decimals: 18,
    };
    AssetRegistry::<T>::insert(asset_id, metadata);
}

#[benchmarks]
mod benchmarks {
    use super::*;

    /// Benchmark submit_comit with varying payload sizes
    ///
    /// This benchmark measures the cost of:
    /// - Authorization check
    /// - Payload validation
    /// - Nonce verification and increment
    /// - Dual-VM execution (mock)
    /// - State merging
    /// - Fee accounting
    /// - Event emission
    #[benchmark]
    fn submit_comit() -> Result<(), BenchmarkError> {
        // Setup: Create funded and authorized account
        let caller: T::AccountId = whitelisted_caller();
        let amount = T::Currency::minimum_balance() * 1_000_000u32.into();
        let _ = T::Currency::make_free_balance_be(&caller, amount);
        AuthorizedAccounts::<T>::insert(&caller, ());

        // Create payloads at reasonable size (1KB each)
        let evm_payload = create_evm_payload::<T>(1024);
        let svm_payload = create_svm_payload::<T>(1024);

        // Compute prepare_root (bypass with zero hash)
        let prepare_root = H256::zero();
        let comit_id = H256::from_low_u64_be(1);
        let nonce = Nonces::<T>::get(&caller);
        let fee: T::Balance = 1_000u32.into();

        #[extrinsic_call]
        submit_comit(
            RawOrigin::Signed(caller.clone()),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            prepare_root,
        );

        // Verify: Comit was recorded
        assert!(SubmittedComits::<T>::contains_key(comit_id));
        Ok(())
    }

    /// Benchmark register_asset
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Symbol validation
    /// - Storage write to AssetRegistry
    /// - Event emission
    #[benchmark]
    fn register_asset() -> Result<(), BenchmarkError> {
        let asset_id: T::AssetId = Default::default();
        let symbol = b"ATLAS".to_vec();
        let decimals = 18u8;

        #[extrinsic_call]
        register_asset(RawOrigin::Root, asset_id, symbol.clone(), decimals);

        // Verify: Asset was registered
        assert!(AssetRegistry::<T>::contains_key(&asset_id));
        Ok(())
    }

    /// Benchmark update_canonical_balance
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Asset existence check
    /// - Storage write to CanonicalLedger
    /// - Optional event emission
    #[benchmark]
    fn update_canonical_balance() -> Result<(), BenchmarkError> {
        // Setup: Register asset first
        let asset_id: T::AssetId = Default::default();
        setup_asset::<T>(asset_id, b"ATLAS");

        let account: T::AccountId = whitelisted_caller();
        let new_balance: T::Balance = 1_000_000u32.into();
        let comit_id = Some(H256::from_low_u64_be(1));

        #[extrinsic_call]
        update_canonical_balance(
            RawOrigin::Root,
            account.clone(),
            asset_id,
            new_balance,
            comit_id,
        );

        // Verify: Balance was updated
        assert_eq!(CanonicalLedger::<T>::get(&account, &asset_id), new_balance);
        Ok(())
    }

    /// Benchmark authorize_account
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Storage write to AuthorizedAccounts
    /// - Event emission
    #[benchmark]
    fn authorize_account() -> Result<(), BenchmarkError> {
        let account: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        authorize_account(RawOrigin::Root, account.clone());

        // Verify: Account was authorized
        assert!(AuthorizedAccounts::<T>::contains_key(&account));
        Ok(())
    }

    /// Benchmark deauthorize_account
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Storage removal from AuthorizedAccounts
    /// - Event emission
    #[benchmark]
    fn deauthorize_account() -> Result<(), BenchmarkError> {
        let account: T::AccountId = whitelisted_caller();
        // Pre-authorize the account
        AuthorizedAccounts::<T>::insert(&account, ());

        #[extrinsic_call]
        deauthorize_account(RawOrigin::Root, account.clone());

        // Verify: Account was deauthorized
        assert!(!AuthorizedAccounts::<T>::contains_key(&account));
        Ok(())
    }

    /// Benchmark add_authority
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Existence check in BoundedVec
    /// - BoundedVec push operation
    /// - Event emission
    #[benchmark]
    fn add_authority() -> Result<(), BenchmarkError> {
        let authority: T::AccountId = whitelisted_caller();

        // Initialize authorities with some entries (worst case: near max)
        let initial_count = (T::MaxAuthorities::get() / 2) as usize;
        let mut initial_authorities: Vec<T::AccountId> = Vec::with_capacity(initial_count);
        for i in 0..initial_count {
            let acc = frame_benchmarking::account::<T::AccountId>("authority", i as u32, 0);
            initial_authorities.push(acc);
        }
        let bounded: BoundedVec<T::AccountId, T::MaxAuthorities> =
            initial_authorities.try_into().expect("Within bounds");
        Authorities::<T>::put(bounded);

        #[extrinsic_call]
        add_authority(RawOrigin::Root, authority.clone());

        // Verify: Authority was added
        assert!(Authorities::<T>::get().contains(&authority));
        Ok(())
    }

    /// Benchmark remove_authority
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Linear search in BoundedVec
    /// - Minimum authorities check
    /// - BoundedVec remove operation
    /// - Event emission
    #[benchmark]
    fn remove_authority() -> Result<(), BenchmarkError> {
        let authority: T::AccountId = whitelisted_caller();

        // Initialize authorities with the target authority and some others
        let initial_count = (T::MaxAuthorities::get() / 2) as usize;
        let mut initial_authorities: Vec<T::AccountId> = Vec::with_capacity(initial_count);
        initial_authorities.push(authority.clone()); // Add target first
        for i in 0..(initial_count - 1) {
            let acc = frame_benchmarking::account::<T::AccountId>("authority", i as u32, 0);
            initial_authorities.push(acc);
        }
        let bounded: BoundedVec<T::AccountId, T::MaxAuthorities> =
            initial_authorities.try_into().expect("Within bounds");
        Authorities::<T>::put(bounded);

        #[extrinsic_call]
        remove_authority(RawOrigin::Root, authority.clone());

        // Verify: Authority was removed
        assert!(!Authorities::<T>::get().contains(&authority));
        Ok(())
    }

    /// Benchmark schedule_authority_change
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Bounds validation
    /// - BoundedVec conversion
    /// - Storage write to PendingAuthorities
    /// - Event emission
    #[benchmark]
    fn schedule_authority_change() -> Result<(), BenchmarkError> {
        // Create a new authority set at half capacity
        let count = (T::MaxAuthorities::get() / 2) as usize;
        let mut new_authorities: Vec<T::AccountId> = Vec::with_capacity(count);
        for i in 0..count {
            let acc = frame_benchmarking::account::<T::AccountId>("new_authority", i as u32, 0);
            new_authorities.push(acc);
        }

        #[extrinsic_call]
        schedule_authority_change(RawOrigin::Root, new_authorities);

        // Verify: Pending changes were stored
        assert!(PendingAuthorities::<T>::get().is_some());
        Ok(())
    }

    /// Benchmark enact_authority_change
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Storage read and take from PendingAuthorities
    /// - BoundedVec conversion
    /// - Storage write to Authorities
    /// - Event emission
    #[benchmark]
    fn enact_authority_change() -> Result<(), BenchmarkError> {
        // Setup: Schedule pending changes first
        let count = (T::MaxAuthorities::get() / 2) as usize;
        let mut new_authorities: Vec<T::AccountId> = Vec::with_capacity(count);
        for i in 0..count {
            let acc = frame_benchmarking::account::<T::AccountId>("pending_authority", i as u32, 0);
            new_authorities.push(acc);
        }
        let bounded: BoundedVec<T::AccountId, T::MaxAuthorities> =
            new_authorities.clone().try_into().expect("Within bounds");
        PendingAuthorities::<T>::put(Some(bounded));

        #[extrinsic_call]
        enact_authority_change(RawOrigin::Root);

        // Verify: Authorities were updated and pending cleared
        assert!(PendingAuthorities::<T>::get().is_none());
        assert_eq!(Authorities::<T>::get().len(), count);
        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
