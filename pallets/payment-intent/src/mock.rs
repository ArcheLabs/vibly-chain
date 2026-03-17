use crate as pallet_payment_intent;
use frame::{
    deps::{frame_support::weights::constants::RocksDbWeight, frame_system::GenesisConfig},
    prelude::*,
    runtime::prelude::*,
    testing_prelude::*,
};
use polkadot_sdk::{pallet_balances, pallet_timestamp};

#[frame_construct_runtime]
mod test_runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask,
        RuntimeViewFunction
    )]
    pub struct Test;

    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    #[runtime::pallet_index(1)]
    pub type Timestamp = pallet_timestamp;
    #[runtime::pallet_index(2)]
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(3)]
    pub type IdentityCore = pallet_identity_core;
    #[runtime::pallet_index(4)]
    pub type PaymentIntent = pallet_payment_intent;
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct IdentityCoreMaxCidLen;
impl Get<u32> for IdentityCoreMaxCidLen {
    fn get() -> u32 {
        96
    }
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct IdentityCoreMaxUriLen;
impl Get<u32> for IdentityCoreMaxUriLen {
    fn get() -> u32 {
        256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct IdentityCoreMaxTransportAccountLen;
impl Get<u32> for IdentityCoreMaxTransportAccountLen {
    fn get() -> u32 {
        128
    }
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct PaymentIntentMaxNamespaceLen;
impl Get<u32> for PaymentIntentMaxNamespaceLen {
    fn get() -> u32 {
        64
    }
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Nonce = u64;
    type Block = MockBlock<Test>;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = RocksDbWeight;
    type AccountData = pallet_balances::AccountData<u128>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<0>;
    type WeightInfo = ();
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<16>;
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ConstU32<0>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<0>;
    type DoneSlashHandler = ();
}

impl pallet_identity_core::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type TimeProvider = Timestamp;
    type MaxCidLen = IdentityCoreMaxCidLen;
    type MaxUriLen = IdentityCoreMaxUriLen;
    type MaxTransportAccountLen = IdentityCoreMaxTransportAccountLen;
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type TimeProvider = Timestamp;
    type IdentityProvider = IdentityCore;
    type Currency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type MaxNamespaceLen = PaymentIntentMaxNamespaceLen;
    type MaxCidLen = IdentityCoreMaxCidLen;
    type MaxUriLen = IdentityCoreMaxUriLen;
}

pub fn new_test_ext() -> TestState {
    let mut storage = GenesisConfig::<Test>::default().build_storage().unwrap();
    let balances = vec![
        (1, 1_000_000),
        (2, 1_000_000),
        (3, 1_000_000),
        (4, 1_000_000),
        (5, 1_000_000),
    ];
    let _ = pallet_balances::GenesisConfig::<Test> {
        balances,
        dev_accounts: None,
    }
    .assimilate_storage(&mut storage);
    let mut ext: TestState = storage.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
