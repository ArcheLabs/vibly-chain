use crate as pallet_onboarding_distribution;
use frame::{
    deps::{frame_support::weights::constants::RocksDbWeight, frame_system::GenesisConfig},
    prelude::*,
    runtime::prelude::*,
    testing_prelude::*,
};

pub type Balance = u128;

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
    pub type OnboardingDistribution = pallet_onboarding_distribution;
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct MaxCidLen;
impl Get<u32> for MaxCidLen {
    fn get() -> u32 { 96 }
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct MaxUriLen;
impl Get<u32> for MaxUriLen {
    fn get() -> u32 { 256 }
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct MaxTransportAccountLen;
impl Get<u32> for MaxTransportAccountLen {
    fn get() -> u32 { 128 }
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Nonce = u64;
    type Block = MockBlock<Test>;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = RocksDbWeight;
    type AccountData = pallet_balances::AccountData<Balance>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<0>;
    type WeightInfo = ();
}

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

impl pallet_identity_core::Config for Test {
    type WeightInfo = ();
    type TimeProvider = Timestamp;
    type MaxCidLen = MaxCidLen;
    type MaxUriLen = MaxUriLen;
    type MaxTransportAccountLen = MaxTransportAccountLen;
}

impl crate::Config for Test {
    type WeightInfo = ();
    type AdminOrigin = frame_system::EnsureRoot<u64>;
    type TimeProvider = Timestamp;
    type IdentityProvider = IdentityCore;
    type Currency = Balances;
    type MaxAgentRefCidLen = MaxCidLen;
    type MaxAgentRefUriLen = MaxUriLen;
}

pub fn new_test_ext() -> TestState {
    let mut ext: TestState = GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
