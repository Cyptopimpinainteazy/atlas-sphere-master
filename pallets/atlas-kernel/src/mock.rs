#![cfg(test)]

use crate as pallet_atlas_kernel;
use frame_support::{construct_runtime, parameter_types, traits::ConstU32};
use frame_system as system;
use parity_scale_codec::Encode;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u128;
pub type AssetId = u32;
pub type AtlasId = u32;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const INITIAL_BALANCE: Balance = 1_000_000_000_000;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const ExistentialDeposit: Balance = 1;
}

construct_runtime!(
    pub enum Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        AtlasKernel: pallet_atlas_kernel,
    }
);

pub type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = system::mocking::MockBlock<Test>;

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 6000;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
}

impl pallet_atlas_kernel::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Balance = Balance;
    type AssetId = AssetId;
    type AtlasId = AtlasId;
    type MaxAssetsPerAccount = ConstU32<16>;
    type MaxAssetSymbolLength = ConstU32<16>;
    type MaxEvmPayloadLength = ConstU32<4096>;
    type MaxSvmPayloadLength = ConstU32<4096>;
    type MaxCombinedPayloadLength = ConstU32<8192>;
    type MaxAuthorities = ConstU32<100>;
    type MinAuthorities = ConstU32<1>;
    type WeightInfo = ();
    type EvmAdapter = MockEvmAdapter;
    type SvmAdapter = MockSvmAdapter;
    type GovernanceOrigin = frame_system::EnsureRoot<AccountId>;
    type StorageRentPeriod = ConstU32<43200>;
}

#[derive(Default)]
pub struct MockEvmAdapter;

impl pallet_atlas_kernel::EvmExecutionAdapter for MockEvmAdapter {
    fn validate_bytecode(
        &self,
        payload: &[u8],
    ) -> Result<(), frame_support::dispatch::DispatchError> {
        // Use real EVM adapter for validation
        use crate::adapters::MockEvmAdapter as RealEvmAdapter;
        let real_adapter = RealEvmAdapter;
        real_adapter.validate(&payload)
    }

    fn execute(
        &self,
        payload: &[u8],
        caller: &[u8; 20],
        context: &pallet_atlas_kernel::EvmExecutionContext,
    ) -> Result<pallet_atlas_kernel::ExecutionReceipt, frame_support::dispatch::DispatchError> {
        // Use real EVM adapter for execution
        use crate::adapters::MockEvmAdapter as RealEvmAdapter;
        let real_adapter = RealEvmAdapter;

        // Convert context to the format expected by the real adapter
        let gas_limit = context.gas_limit;

        match real_adapter.execute(payload, gas_limit) {
            Ok(receipt) => {
                // Convert from adapters::ExecutionReceipt to pallet's ExecutionReceipt
                Ok(pallet_atlas_kernel::ExecutionReceipt {
                    success: receipt.success,
                    gas_used: receipt.gas_used,
                    return_data: receipt.return_data.into(),
                    logs: receipt.logs.into(),
                    state_changes: receipt.state_changes.into(),
                })
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Default)]
pub struct MockSvmAdapter;

impl pallet_atlas_kernel::SvmExecutionAdapter for MockSvmAdapter {
    fn validate_program(
        &self,
        payload: &[u8],
    ) -> Result<(), frame_support::dispatch::DispatchError> {
        // Use real SVM adapter for validation
        use crate::adapters::MockSvmAdapter as RealSvmAdapter;
        let real_adapter = RealSvmAdapter;
        real_adapter.validate(&payload)
    }

    fn execute(
        &self,
        payload: &[u8],
        payer: &[u8; 32],
        context: &pallet_atlas_kernel::SvmExecutionContext,
    ) -> Result<pallet_atlas_kernel::ExecutionReceipt, frame_support::dispatch::DispatchError> {
        // Use real SVM adapter for execution
        use crate::adapters::MockSvmAdapter as RealSvmAdapter;
        let real_adapter = RealSvmAdapter;

        // Convert context to the format expected by the real adapter
        let compute_limit = context.compute_unit_limit;

        match real_adapter.execute(payload, compute_limit) {
            Ok(receipt) => {
                // Convert from adapters::ExecutionReceipt to pallet's ExecutionReceipt
                Ok(pallet_atlas_kernel::ExecutionReceipt {
                    success: receipt.success,
                    gas_used: receipt.gas_used,
                    return_data: receipt.return_data.into(),
                    logs: receipt.logs.into(),
                    state_changes: receipt.state_changes.into(),
                })
            }
            Err(e) => Err(e),
        }
    }
}

pub struct ExtBuilder {
    balances: Vec<(AccountId, Balance)>,
    authorized_accounts: Vec<AccountId>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: vec![],
            authorized_accounts: vec![],
        }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn authorized_accounts(mut self, accounts: Vec<AccountId>) -> Self {
        self.authorized_accounts = accounts;
        self
    }

    pub fn build(self) -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .expect("Failed to build system genesis storage");

        // Apply balances genesis
        pallet_balances::GenesisConfig::<Test> {
            balances: self.balances,
        }
        .assimilate_storage(&mut storage)
        .expect("Failed to assimilate balances storage");

        let mut t = TestExternalities::new(storage);

        t.execute_with(|| {
            System::set_block_number(1);
            // Set initial timestamp
            Timestamp::set_timestamp(12000);
            // Initialize authorized accounts
            for account in self.authorized_accounts {
                pallet_atlas_kernel::AuthorizedAccounts::<Test>::insert(account, ());
            }
        });
        t
    }
}

pub fn new_test_ext() -> TestExternalities {
    ExtBuilder::default()
        .balances(vec![
            (ALICE, INITIAL_BALANCE),
            (BOB, INITIAL_BALANCE),
            (CHARLIE, INITIAL_BALANCE),
        ])
        .authorized_accounts(vec![ALICE, BOB, CHARLIE])
        .build()
}

/// Mock implementation of DualVmDispatcher for testing
pub struct MockDispatcher;

impl pallet_atlas_kernel::DualVmDispatcher for MockDispatcher {
    type AccountId = AccountId;
    type Balance = Balance;

    fn execute_evm_tx(
        &self,
        _tx: Vec<u8>,
    ) -> Result<pallet_atlas_kernel::ExecutionReceipt, frame_support::dispatch::DispatchError> {
        Ok(pallet_atlas_kernel::ExecutionReceipt {
            success: true,
            gas_used: 21000,
            return_data: Default::default(),
            logs: Default::default(),
            state_changes: Default::default(),
        })
    }

    fn execute_svm_tx(
        &self,
        _tx: Vec<u8>,
    ) -> Result<pallet_atlas_kernel::ExecutionReceipt, frame_support::dispatch::DispatchError> {
        Ok(pallet_atlas_kernel::ExecutionReceipt {
            success: true,
            gas_used: 0,
            return_data: Default::default(),
            logs: Default::default(),
            state_changes: Default::default(),
        })
    }

    fn execute_dual_tx(
        &self,
        evm_tx: Option<Vec<u8>>,
        svm_tx: Option<Vec<u8>>,
    ) -> Result<pallet_atlas_kernel::SphereState, frame_support::dispatch::DispatchError> {
        let _evm_receipt = if evm_tx.is_some() {
            Some(self.execute_evm_tx(evm_tx.unwrap())?)
        } else {
            None
        };

        let _svm_receipt = if svm_tx.is_some() {
            Some(self.execute_svm_tx(svm_tx.unwrap())?)
        } else {
            None
        };

        Ok(pallet_atlas_kernel::SphereState {
            state_root: H256::zero(),
            block_number: 1,
            timestamp: 12000,
        })
    }

    fn merge_receipts(
        &self,
        _evm_receipt: Option<&pallet_atlas_kernel::ExecutionReceipt>,
        _svm_receipt: Option<&pallet_atlas_kernel::ExecutionReceipt>,
    ) -> pallet_atlas_kernel::SphereState {
        pallet_atlas_kernel::SphereState {
            state_root: H256::zero(),
            block_number: 1,
            timestamp: 12000,
        }
    }

    /// Check authorization - in mock, always allow ALICE, deny others for non-empty ops
    fn auth_check(
        &self,
        caller: &Self::AccountId,
        operation: &[u8],
    ) -> Result<(), frame_support::dispatch::DispatchError> {
        if *caller == ALICE {
            Ok(())
        } else if operation.is_empty() {
            Ok(())
        } else {
            Err(frame_support::dispatch::DispatchError::BadOrigin)
        }
    }

    /// Calculate fees: 1 unit per 1000 gas + 1 unit per 1000 compute units
    fn fee_accounting(
        &self,
        evm_gas_used: u64,
        svm_compute_units: u64,
        base_fee: Self::Balance,
    ) -> Result<Self::Balance, frame_support::dispatch::DispatchError> {
        let evm_fee = (evm_gas_used as u128) / 1000;
        let svm_fee = (svm_compute_units as u128) / 1000;
        let total = base_fee + evm_fee + svm_fee;
        Ok(total)
    }

    /// Update canonical ledger - in mock, just verify state changes are well-formed
    fn canonical_ledger_update(
        &self,
        _comit_id: H256,
        state_changes: &[pallet_atlas_kernel::StateChange],
    ) -> Result<(), frame_support::dispatch::DispatchError> {
        // Verify all state changes have valid addresses
        for change in state_changes {
            if change.address.is_empty() {
                return Err(frame_support::dispatch::DispatchError::Other(
                    "Invalid state change address",
                ));
            }
        }
        Ok(())
    }
}
