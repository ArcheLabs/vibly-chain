use crate as pallet_vib_claim;
use frame::{
    deps::{frame_support::weights::constants::RocksDbWeight, frame_system::GenesisConfig},
    prelude::*,
    runtime::prelude::*,
    testing_prelude::*,
};

pub type Balance = u128;
pub const RESERVE: u64 = 99;

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
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(2)]
    pub type VibClaim = pallet_vib_claim;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type AccountId = u64;
    type Nonce = u64;
    type Block = MockBlock<Test>;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = RocksDbWeight;
    type AccountData = pallet_balances::AccountData<Balance>;
}

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type AdminOrigin = frame_system::EnsureRoot<u64>;
    type ClaimReserveAccount = ConstU64<RESERVE>;
    type MaxNetworkIdLen = ConstU32<64>;
    type MaxIdentityIdLen = ConstU32<128>;
    type MaxProofLen = ConstU32<64>;
}

pub fn network_id() -> crate::BoundedNetworkIdOf<Test> {
    b"substrate:vibly-solo".to_vec().try_into().unwrap()
}

pub fn identity_id() -> crate::BoundedIdentityIdOf<Test> {
    b"identity-1".to_vec().try_into().unwrap()
}

pub fn new_test_ext() -> TestState {
    let mut storage = GenesisConfig::<Test>::default().build_storage().unwrap();
    let _ = pallet_balances::GenesisConfig::<Test> {
        balances: vec![(RESERVE, 1_000_000), (1, 10), (2, 10)],
        dev_accounts: None,
    }
    .assimilate_storage(&mut storage);
    let mut ext: TestState = storage.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
