use crate as pallet_vibly_emergency;
use frame::{
    deps::{frame_support::weights::constants::RocksDbWeight, frame_system::GenesisConfig},
    prelude::*,
    runtime::prelude::*,
    testing_prelude::*,
};

// In unit-tests we use simple origins:
//   PauseOrigin  = EnsureSigned<u64>   (any signed account simulates a Guardian member)
//   CancelOrigin = EnsureRoot<u64>     (root simulates Guardian collective m/n)
//   ResumeOrigin = EnsureRoot<u64>
use frame_system::{EnsureRoot, EnsureSigned};

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
    pub type ViblyEmergency = pallet_vibly_emergency;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Nonce = u64;
    type Block = MockBlock<Test>;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = RocksDbWeight;
}

impl pallet_vibly_emergency::Config for Test {
    type WeightInfo = ();
    type PauseOrigin = EnsureSigned<u64>;
    type CancelOrigin = EnsureRoot<u64>;
    type ResumeOrigin = EnsureRoot<u64>;
}

pub fn new_test_ext() -> TestState {
    let storage = GenesisConfig::<Test>::default().build_storage().unwrap();
    let mut ext: TestState = storage.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
