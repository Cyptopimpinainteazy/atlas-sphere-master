//! Mock runtime for audit governance pallet tests

use crate as pallet_audit_governance;
use frame_support::parameter_types;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet
frame_support::construct_runtime!(
    pub struct Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        AuditGovernance: pallet_audit_governance::{Pallet, Call, Storage, Event<T>},
    }
);

// System pallet configuration
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u32;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = frame_support::traits::ConstU32<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

// Audit governance pallet configuration
parameter_types! {
    pub const MaxAuditArtifacts: u32 = 1_000;
    pub const MaxFindings: u32 = 100;
    pub const AuditAppealPeriod: u32 = 100;  // 100 blocks for testing (faster)
    pub const OverrideThreshold: u32 = 67;
}

impl pallet_audit_governance::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MaxAuditArtifacts = MaxAuditArtifacts;
    type MaxFindings = MaxFindings;
    type AuditAppealPeriod = AuditAppealPeriod;
    type OverrideThreshold = OverrideThreshold;
}

/// Build genesis storage for tests
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}

/// Helper function to create test audit artifact data
pub fn create_test_audit_data() -> (H256, H256, Vec<(Vec<u8>, u8, Vec<u8>)>) {
    let audit_id = H256::from_low_u64_be(1);
    let commit_hash = H256::from_low_u64_be(100);
    let findings = vec![
        (
            "Security".as_bytes().to_vec(),
            9u8,
            "Critical vulnerability".as_bytes().to_vec(),
        ),
        (
            "Architecture".as_bytes().to_vec(),
            7u8,
            "Design flaw".as_bytes().to_vec(),
        ),
    ];
    (audit_id, commit_hash, findings)
}

/// Helper function to whitelist a test account
pub fn whitelist_account(account: u64) {
    let _ = pallet_audit_governance::Pallet::<Test>::whitelist_audit_submitter(
        frame_system::RawOrigin::Root.into(),
        account,
    );
}

/// Helper function to submit test audit
pub fn submit_test_audit(
    caller: u64,
    audit_id: H256,
    commit_hash: H256,
    decision: u8,
    critical_count: u32,
    findings: Vec<(Vec<u8>, u8, Vec<u8>)>,
) -> Result<(), sp_runtime::DispatchError> {
    pallet_audit_governance::Pallet::<Test>::submit_audit_artifact(
        frame_system::RawOrigin::Signed(caller).into(),
        audit_id,
        commit_hash,
        decision,
        critical_count,
        findings,
        0u64, // timestamp
    )
}
