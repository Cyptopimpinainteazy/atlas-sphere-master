//! Mock runtime for governance pallet tests.

use crate as pallet_governance;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU32, EqualPrivilegeOnly, Hooks},
    weights::Weight,
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, Percent,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure mock runtime
frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Scheduler: pallet_scheduler,
        Governance: pallet_governance,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
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

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type MaxHolds = ConstU32<0>;
    type RuntimeHoldReason = RuntimeHoldReason;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Weight::from_parts(10_000_000, 0);
    pub const MaxScheduledPerBlock: u32 = 10;
}

impl pallet_scheduler::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = frame_system::EnsureRoot<u64>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = ();
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = ();
}

parameter_types! {
    pub const ProposalDeposit: u128 = 100;
    pub const VotingPeriod: u64 = 100;
    pub const EnactmentPeriod: u64 = 10;
    pub const Quorum: Percent = Percent::from_percent(10);
    pub const ApprovalThreshold: Percent = Percent::from_percent(50);
    pub const MaxProposals: u32 = 100;
    pub const MaxVotes: u32 = 100;
    pub const MaxDelegations: u32 = 100;
    pub const ConvictionPeriod: u64 = 10;
}

impl pallet_governance::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type SubmitOrigin = frame_system::EnsureSigned<u64>;
    type FastTrackOrigin = frame_system::EnsureRoot<u64>;
    type CancelOrigin = frame_system::EnsureRoot<u64>;
    type RuntimeUpgradeOrigin = frame_system::EnsureRoot<u64>;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type ProposalDeposit = ProposalDeposit;
    type VotingPeriod = VotingPeriod;
    type EnactmentPeriod = EnactmentPeriod;
    type Quorum = Quorum;
    type ApprovalThreshold = ApprovalThreshold;
    type MaxProposals = MaxProposals;
    type MaxVotes = MaxVotes;
    type MaxDelegations = MaxDelegations;
    type ConvictionPeriod = ConvictionPeriod;
    type WeightInfo = ();
}

/// Build genesis storage for testing
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 10_000),
            (2, 10_000),
            (3, 10_000),
            (4, 10_000),
            (5, 10_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_governance::GenesisConfig::<Test>::default()
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Helper to advance blocks
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        <Governance as Hooks<u64>>::on_initialize(System::block_number());
        System::set_block_number(System::block_number() + 1);
    }
}

/// Helper to get test account
pub fn account(id: u64) -> u64 {
    id
}

#[test]
fn migration_sets_storage_version() {
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        use frame_support::traits::StorageVersion;
        use crate::pallet as governance_pallet;

        StorageVersion::put::<governance_pallet::Pallet<Test>>(StorageVersion::new(0));
        let _w = crate::migrations::Migration::<Test>::on_runtime_upgrade();
        assert!(StorageVersion::get::<governance_pallet::Pallet<Test>>() >= governance_pallet::STORAGE_VERSION);
    });
}
