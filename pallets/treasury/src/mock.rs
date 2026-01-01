//! Mock runtime for treasury pallet tests.

use crate as pallet_treasury;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Hooks},
    PalletId,
};
use frame_system::EnsureRoot;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, Percent,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Treasury: pallet_treasury,
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
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const ProposalBond: Percent = Percent::from_percent(5);
    pub const ProposalBondMinimum: u128 = 10;
    pub const SmallSpendLimit: u128 = 1_000;
    pub const MediumSpendLimit: u128 = 10_000;
    pub const LargeSpendLimit: u128 = 100_000;
}

impl pallet_treasury::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type SmallSpendOrigin = EnsureRoot<u64>;
    type MediumSpendOrigin = EnsureRoot<u64>;
    type LargeSpendOrigin = EnsureRoot<u64>;
    type CriticalSpendOrigin = EnsureRoot<u64>;
    type PauseOrigin = EnsureRoot<u64>;
    type YieldConfigOrigin = EnsureRoot<u64>;
    type PalletId = TreasuryPalletId;
    type MaxSigners = ConstU32<10>;
    type MaxProposals = ConstU32<100>;
    type MaxRecurringPayments = ConstU32<50>;
    type MaxYieldStrategies = ConstU32<20>;
    type SmallSpendLimit = SmallSpendLimit;
    type MediumSpendLimit = MediumSpendLimit;
    type LargeSpendLimit = LargeSpendLimit;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type WeightInfo = ();
}

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;
pub const EVE: u64 = 5;
pub const TREASURY: u64 = 6;

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
            (DAVE, 100_000),
            (EVE, 100_000),
            (Treasury::account_id(), 1_000_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_treasury::GenesisConfig::<Test> {
        initial_signers: vec![ALICE, BOB, CHARLIE],
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
        <Treasury as Hooks<u64>>::on_initialize(System::block_number());
    }
}

#[test]
fn migration_sets_storage_version() {
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        use frame_support::traits::StorageVersion;
        use crate::pallet as treasury_pallet;

        StorageVersion::put::<treasury_pallet::Pallet<Test>>(StorageVersion::new(0));
        let _w = crate::migrations::Migration::<Test>::on_runtime_upgrade();
        assert!(StorageVersion::get::<treasury_pallet::Pallet<Test>>() >= treasury_pallet::STORAGE_VERSION);
    });
}
