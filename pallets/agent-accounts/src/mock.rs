//! Mock runtime for agent-accounts pallet tests.

use crate as pallet_agent_accounts;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Hooks},
};
use frame_system::EnsureRoot;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        AgentAccounts: pallet_agent_accounts,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type MaxHolds = ConstU32<0>;
    type RuntimeHoldReason = ();
}

parameter_types! {
    pub const RegistrationDeposit: u128 = 1000;
    pub const DefaultGasPerBlock: u128 = 1_000_000;
    pub const DefaultComputePerBlock: u128 = 500_000;
    pub const DefaultGasPerEpoch: u128 = 100_000_000;
    pub const DefaultComputePerEpoch: u128 = 50_000_000;
    pub const BlocksPerEpoch: u64 = 100;
}

impl pallet_agent_accounts::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type RegisterOrigin = EnsureRoot<u64>;
    type AdminOrigin = EnsureRoot<u64>;
    type MaxAgentsPerController = ConstU32<10>;
    type RegistrationDeposit = RegistrationDeposit;
    type DefaultGasPerBlock = DefaultGasPerBlock;
    type DefaultComputePerBlock = DefaultComputePerBlock;
    type DefaultGasPerEpoch = DefaultGasPerEpoch;
    type DefaultComputePerEpoch = DefaultComputePerEpoch;
    type BlocksPerEpoch = BlocksPerEpoch;
    type WeightInfo = ();
}

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const OPERATOR1: u64 = 10;
pub const OPERATOR2: u64 = 11;
pub const OPERATOR3: u64 = 12;

/// Build test externalities.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE, 100_000),
            (BOB, 100_000),
            (CHARLIE, 100_000),
            (OPERATOR1, 10_000),
            (OPERATOR2, 10_000),
            (OPERATOR3, 10_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Advance to a specific block.
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
        <AgentAccounts as Hooks<u64>>::on_initialize(System::block_number());
    }
}

#[test]
fn migration_sets_storage_version() {
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        use frame_support::traits::StorageVersion;
        use crate::pallet as accounts_pallet;

        StorageVersion::put::<accounts_pallet::Pallet<Test>>(StorageVersion::new(0));
        let _w = crate::migrations::Migration::<Test>::on_runtime_upgrade();
        assert!(StorageVersion::get::<accounts_pallet::Pallet<Test>>() >= accounts_pallet::STORAGE_VERSION);
    });
}
