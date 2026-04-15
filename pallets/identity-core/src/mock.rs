use crate as pallet_identity_core;
use frame::{
    deps::{frame_support::weights::constants::RocksDbWeight, frame_system::GenesisConfig},
    prelude::*,
    runtime::prelude::*,
    testing_prelude::*,
};

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
    pub type IdentityCore = pallet_identity_core;
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

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Nonce = u64;
    type Block = MockBlock<Test>;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = RocksDbWeight;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<0>;
    type WeightInfo = ();
}

impl crate::Config for Test {
    type WeightInfo = ();
    type TimeProvider = Timestamp;
    type MaxCidLen = IdentityCoreMaxCidLen;
    type MaxUriLen = IdentityCoreMaxUriLen;
    type MaxTransportAccountLen = IdentityCoreMaxTransportAccountLen;
}

pub fn new_test_ext() -> TestState {
    let mut ext: TestState = GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
