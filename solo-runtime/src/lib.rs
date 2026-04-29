#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(test)]
mod tests;

extern crate alloc;

use alloc::vec::Vec;
use frame_support::{
    derive_impl,
    parameter_types,
    traits::{ConstBool, ConstU32, ConstU64, ConstU8, EnsureOrigin, Get, InitializeMembers, ChangeMembers, SortedMembers, VariantCountOf},
    weights::{
        constants::WEIGHT_REF_TIME_PER_SECOND, ConstantMultiplier, Weight, WeightToFeeCoefficient,
        WeightToFeeCoefficients, WeightToFeePolynomial,
    },
    PalletId,
};
use frame_system::EnsureRoot;
use pallet_collective::{EnsureProportionAtLeast, Instance1};
use pallet_vibly_emergency::EmergencyScope;
use polkadot_sdk::*;
use sp_runtime::traits::AccountIdConversion;
use smallvec::smallvec;
use sp_runtime::{
    generic, impl_opaque_keys,
    traits::{AccountIdLookup, BlakeTwo256, IdentifyAccount, Verify},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, MultiSignature, Perbill,
};
use sp_version::RuntimeVersion;

#[cfg(feature = "std")]
use sp_version::NativeVersion;

pub use pallet_aura::Authorities as AuraAuthorities;
pub use pallet_grandpa::fg_primitives;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use sp_runtime::{MultiAddress, OpaqueExtrinsic};
use polkadot_sdk_frame::runtime::prelude::build_state;

pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Balance = u128;
pub type Nonce = u32;
pub type Hash = sp_core::H256;
pub type BlockNumber = u32;
pub type Address = MultiAddress<AccountId, ()>;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type SignedBlock = generic::SignedBlock<Block>;
pub type BlockId = generic::BlockId<Block>;

pub type TxExtension = (
    frame_system::AuthorizeCall<Runtime>,
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

pub mod opaque {
    use super::*;
    use sp_runtime::traits::Hash as HashT;

    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    pub type Block = generic::Block<Header, OpaqueExtrinsic>;
    pub type BlockId = generic::BlockId<Block>;
    pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
        pub grandpa: Grandpa,
    }
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("vibly-solo-runtime"),
    impl_name: alloc::borrow::Cow::Borrowed("vibly-solo-runtime"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};

#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

pub const MILLI_SECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLI_SECS_PER_BLOCK;
pub const MINUTES: BlockNumber = 60_000 / (MILLI_SECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const UNIT: Balance = 1_000_000_000_000;
pub const CENTS: Balance = UNIT / 100;
pub const MILLI_UNIT: Balance = 1_000_000_000;
pub const MICRO_UNIT: Balance = 1_000_000;
pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_UNIT;

const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
    u64::MAX,
);

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;

    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        let p = MILLI_UNIT / 10;
        let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
        smallvec![WeightToFeeCoefficient {
            degree: 1,
            negative: false,
            coeff_frac: Perbill::from_rational(p % q, q),
            coeff_integer: p / q,
        }]
    }
}

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: frame_system::limits::BlockLength =
        frame_system::limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(frame_support::dispatch::DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(frame_support::dispatch::DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(frame_support::dispatch::DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            weights.reserved = Some(MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u16 = 42;
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const TransactionByteFee: Balance = 10 * MICRO_UNIT;
    pub const TreasuryPalletId: PalletId = PalletId(*b"vby/tsry");
    pub const GuardianMotionDuration: BlockNumber = 5 * MINUTES;
    pub const GuardianMaxProposals: u32 = 100;
    pub const GuardianMaxMembers: u32 = 100;
    pub MaxProposalWeight: Weight = MAXIMUM_BLOCK_WEIGHT;

    // Treasury
    pub const ProposalBond: sp_runtime::Permill = sp_runtime::Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = UNIT;
    pub const ProposalBondMaximum: Balance = 100 * UNIT;
    pub const SpendPeriod: BlockNumber = 7 * DAYS;
    pub const MaxApprovals: u32 = 100;
    pub const TreasurySpendOriginMaxAmount: Balance = Balance::MAX;
    pub TreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();

    // Referenda / ConvictionVoting
    pub const VoteLockingPeriod: BlockNumber = 7 * DAYS;
    pub const MaxVotes: u32 = 512;
    pub const MaxTurnout: u128 = 1_000_000 * UNIT;
    pub const AlarmInterval: BlockNumber = 1;
    pub const SubmissionDeposit: Balance = UNIT;
    pub const UndecidingTimeout: BlockNumber = 14 * DAYS;

    // Preimage
    pub const PreimageBaseDeposit: Balance = UNIT;
    pub const PreimageByteDeposit: Balance = MICRO_UNIT;
    pub const PreimageHoldReason: RuntimeHoldReason =
        RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);

    // Scheduler
    pub MaximumSchedulerWeight: Weight = MAXIMUM_BLOCK_WEIGHT;
    pub const MaxScheduledPerBlock: u32 = 512;
    pub const NoPreimagePostponement: Option<BlockNumber> = Some(10);
}

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
    type AccountId = AccountId;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type Nonce = Nonce;
    type Hash = Hash;
    type Block = Block;
    type BlockHashCount = ConstU32<2400>;
    type Version = Version;
    type AccountData = pallet_balances::AccountData<Balance>;
    type DbWeight = RocksDbWeight;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type MaxConsumers = ConstU32<16>;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<0>;
    type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<100_000>;
    type AllowMultipleBlocksPerSlot = ConstBool<true>;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = ConstU32<100_000>;
    type MaxNominators = ConstU32<0>;
    type MaxSetIdSessionEntries = ConstU64<0>;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
    type DoneSlashHandler = ();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    type WeightToFee = WeightToFee;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = ();
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct IdentityMaxCidLen;
impl Get<u32> for IdentityMaxCidLen {
    fn get() -> u32 { 96 }
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct IdentityMaxUriLen;
impl Get<u32> for IdentityMaxUriLen {
    fn get() -> u32 { 256 }
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct IdentityMaxTransportAccountLen;
impl Get<u32> for IdentityMaxTransportAccountLen {
    fn get() -> u32 { 128 }
}

#[derive(Clone, Debug, Eq, PartialEq, scale_info::TypeInfo)]
pub struct PaymentMaxNamespaceLen;
impl Get<u32> for PaymentMaxNamespaceLen {
    fn get() -> u32 { 64 }
}

impl pallet_identity_core::Config for Runtime {
    type WeightInfo = pallet_identity_core::weights::SubstrateWeight<Runtime>;
    type TimeProvider = Timestamp;
    type MaxCidLen = IdentityMaxCidLen;
    type MaxUriLen = IdentityMaxUriLen;
    type MaxTransportAccountLen = IdentityMaxTransportAccountLen;
}

impl pallet_payment_intent::Config for Runtime {
    type WeightInfo = pallet_payment_intent::weights::SubstrateWeight<Runtime>;
    type TimeProvider = Timestamp;
    type IdentityProvider = IdentityCore;
    type Currency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type MaxNamespaceLen = PaymentMaxNamespaceLen;
    type MaxCidLen = IdentityMaxCidLen;
    type MaxUriLen = IdentityMaxUriLen;
}

pub struct GuardianMembershipManager;
impl SortedMembers<AccountId> for GuardianMembershipManager {
    fn sorted_members() -> Vec<AccountId> {
        GuardianMembership::members().into_inner()
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn add(_member: &AccountId) {}
}

impl InitializeMembers<AccountId> for GuardianMembershipManager {
    fn initialize_members(members: &[AccountId]) {
        GuardianCollective::initialize_members(members);
    }
}

impl ChangeMembers<AccountId> for GuardianMembershipManager {
    fn change_members_sorted(incoming: &[AccountId], outgoing: &[AccountId], new: &[AccountId]) {
        GuardianCollective::change_members_sorted(incoming, outgoing, new);
    }

    fn set_prime(who: Option<AccountId>) {
        GuardianCollective::set_prime(who);
    }

    fn get_prime() -> Option<AccountId> {
        GuardianCollective::get_prime()
    }
}

impl pallet_membership::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRoot<AccountId>;
    type RemoveOrigin = EnsureRoot<AccountId>;
    type SwapOrigin = EnsureRoot<AccountId>;
    type ResetOrigin = EnsureRoot<AccountId>;
    type PrimeOrigin = EnsureRoot<AccountId>;
    type MembershipInitialized = GuardianMembershipManager;
    type MembershipChanged = GuardianMembershipManager;
    type MaxMembers = ConstU32<100>;
    type WeightInfo = ();
}

impl pallet_collective::Config<Instance1> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = GuardianMotionDuration;
    type MaxProposals = GuardianMaxProposals;
    type MaxMembers = GuardianMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = ();
    type SetMembersOrigin = EnsureRoot<AccountId>;
    type MaxProposalWeight = MaxProposalWeight;
    type DisapproveOrigin = EnsureRoot<AccountId>;
    type KillOrigin = EnsureRoot<AccountId>;
    type Consideration = ();
}

// ── OpenGov pallets ──────────────────────────────────────────────────────────

impl pallet_preimage::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type Consideration = ();
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = ();
    type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
    type Preimages = Preimage;
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
    type Id = u16;
    type RuntimeOrigin = OriginCaller;
    fn tracks() -> impl Iterator<Item = alloc::borrow::Cow<'static, pallet_referenda::Track<Self::Id, Balance, BlockNumber>>> {
        [pallet_referenda::Track {
            id: 0,
            info: pallet_referenda::TrackInfo {
                name: *b"root\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                max_deciding: 1,
                decision_deposit: 10 * UNIT,
                prepare_period: HOURS,
                decision_period: 7 * DAYS,
                confirm_period: HOURS,
                min_enactment_period: HOURS,
                min_approval: pallet_referenda::Curve::LinearDecreasing {
                    length: sp_runtime::Perbill::from_percent(100),
                    floor: sp_runtime::Perbill::from_percent(50),
                    ceil: sp_runtime::Perbill::from_percent(100),
                },
                min_support: pallet_referenda::Curve::LinearDecreasing {
                    length: sp_runtime::Perbill::from_percent(100),
                    floor: sp_runtime::Perbill::from_percent(0),
                    ceil: sp_runtime::Perbill::from_percent(50),
                },
            },
        }]
        .into_iter()
        .map(alloc::borrow::Cow::Owned)
    }
    fn track_for(_id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
        // Single root track; all proposals route here for early testnet.
        Ok(0)
    }
}

impl pallet_referenda::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Scheduler = Scheduler;
    type Currency = Balances;
    type SubmitOrigin = frame_system::EnsureSigned<AccountId>;
    type CancelOrigin = EnsureRoot<AccountId>;
    type KillOrigin = EnsureRoot<AccountId>;
    type Slash = ();
    type Votes = pallet_conviction_voting::VotesOf<Runtime>;
    type Tally = pallet_conviction_voting::TallyOf<Runtime>;
    type SubmissionDeposit = SubmissionDeposit;
    type MaxQueued = ConstU32<100>;
    type UndecidingTimeout = UndecidingTimeout;
    type AlarmInterval = AlarmInterval;
    type Tracks = TracksInfo;
    type Preimages = Preimage;
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

impl pallet_conviction_voting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type VoteLockingPeriod = VoteLockingPeriod;
    type MaxVotes = MaxVotes;
    type MaxTurnout = frame_support::traits::TotalIssuanceOf<Balances, AccountId>;
    type Polls = Referenda;
    type WeightInfo = ();
    type VotingHooks = ();
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

impl pallet_whitelist::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WhitelistOrigin = EnsureRoot<AccountId>;
    type DispatchWhitelistedOrigin = EnsureRoot<AccountId>;
    type Preimages = Preimage;
    type WeightInfo = ();
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type RejectOrigin = EnsureRoot<AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type SpendPeriod = SpendPeriod;
    type Burn = ();
    type BurnDestination = ();
    type SpendFunds = Bounties;
    type MaxApprovals = MaxApprovals;
    type WeightInfo = ();
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
    type AssetKind = ();
    type Beneficiary = AccountId;
    type BeneficiaryLookup = sp_runtime::traits::IdentityLookup<AccountId>;
    type Paymaster = frame_support::traits::tokens::pay::PayFromAccount<Balances, TreasuryAccount>;
    type BalanceConverter = frame_support::traits::tokens::UnityAssetBalanceConversion;
    type PayoutPeriod = ConstU32<10>;
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub const BountyDepositBase: Balance = UNIT;
    pub const BountyDepositPayoutDelay: BlockNumber = 2 * DAYS;
    pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
    pub const CuratorDepositMultiplier: sp_runtime::Permill = sp_runtime::Permill::from_percent(50);
    pub const CuratorDepositMin: Balance = UNIT / 2;
    pub const CuratorDepositMax: Balance = 100 * UNIT;
    pub const BountyValueMinimum: Balance = UNIT;
    pub const DataDepositPerByte: Balance = MICRO_UNIT;
}

impl pallet_bounties::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type CuratorDepositMultiplier = CuratorDepositMultiplier;
    type CuratorDepositMin = CuratorDepositMin;
    type CuratorDepositMax = CuratorDepositMax;
    type BountyValueMinimum = BountyValueMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = ConstU32<16384>;
    type WeightInfo = ();
    type ChildBountyManager = ChildBounties;
    type OnSlash = ();
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

impl pallet_child_bounties::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxActiveChildBountyCount = ConstU32<100>;
    type ChildBountyValueMinimum = BountyValueMinimum;
    type WeightInfo = ();
}

// ── Guardian Emergency origin ─────────────────────────────────────────────────

pub struct EnsureGuardianMember;
impl EnsureOrigin<RuntimeOrigin> for EnsureGuardianMember {
    type Success = AccountId;

    fn try_origin(origin: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
        match frame_system::ensure_signed(origin.clone()) {
            Ok(who) if GuardianMembership::members().contains(&who) => Ok(who),
            _ => Err(origin),
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        GuardianMembership::members()
            .first()
            .cloned()
            .map(RuntimeOrigin::signed)
            .ok_or(())
    }
}

pub type GuardianCollectiveTwoThirds =
    EnsureProportionAtLeast<AccountId, Instance1, 2, 3>;

impl pallet_vibly_emergency::Config for Runtime {
    type WeightInfo = pallet_vibly_emergency::weights::SubstrateWeight<Runtime>;
    type PauseOrigin = EnsureGuardianMember;
    type CancelOrigin = GuardianCollectiveTwoThirds;
    type ResumeOrigin = GuardianCollectiveTwoThirds;
}

#[frame_support::runtime]
mod runtime {
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
    pub struct Runtime;

    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    #[runtime::pallet_index(1)]
    pub type Timestamp = pallet_timestamp;

    #[runtime::pallet_index(10)]
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(11)]
    pub type TransactionPayment = pallet_transaction_payment;

    #[runtime::pallet_index(15)]
    pub type Sudo = pallet_sudo;

    #[runtime::pallet_index(20)]
    pub type Aura = pallet_aura;
    #[runtime::pallet_index(21)]
    pub type Grandpa = pallet_grandpa;

    #[runtime::pallet_index(40)]
    pub type GuardianMembership = pallet_membership;
    #[runtime::pallet_index(41)]
    pub type GuardianCollective = pallet_collective<Instance1>;

    #[runtime::pallet_index(50)]
    pub type IdentityCore = pallet_identity_core;
    #[runtime::pallet_index(51)]
    pub type PaymentIntent = pallet_payment_intent;
    #[runtime::pallet_index(52)]
    pub type ViblyEmergency = pallet_vibly_emergency;

    // OpenGov
    #[runtime::pallet_index(60)]
    pub type Preimage = pallet_preimage;
    #[runtime::pallet_index(61)]
    pub type Scheduler = pallet_scheduler;
    #[runtime::pallet_index(62)]
    pub type Referenda = pallet_referenda;
    #[runtime::pallet_index(63)]
    pub type ConvictionVoting = pallet_conviction_voting;
    #[runtime::pallet_index(64)]
    pub type Whitelist = pallet_whitelist;
    #[runtime::pallet_index(65)]
    pub type Treasury = pallet_treasury;
    #[runtime::pallet_index(66)]
    pub type Bounties = pallet_bounties;
    #[runtime::pallet_index(67)]
    pub type ChildBounties = pallet_child_bounties;
}

pub fn guardian_scope(id: u64) -> EmergencyScope {
    EmergencyScope::Proposal(id)
}

sp_api::impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: <Block as sp_runtime::traits::Block>::LazyBlock) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as sp_runtime::traits::Block>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> sp_core::OpaqueMetadata {
            sp_core::OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<sp_core::OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl frame_support::view_functions::runtime_api::RuntimeViewFunction<Block> for Runtime {
        fn execute_view_function(
            id: frame_support::view_functions::ViewFunctionId,
            input: Vec<u8>,
        ) -> Result<Vec<u8>, frame_support::view_functions::ViewFunctionDispatchError> {
            Runtime::execute_view_function(id, input)
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as sp_runtime::traits::Block>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as sp_runtime::traits::Block>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as sp_runtime::traits::Block>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: <Block as sp_runtime::traits::Block>::LazyBlock,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as sp_runtime::traits::Block>::Extrinsic,
            block_hash: <Block as sp_runtime::traits::Block>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as sp_runtime::traits::Block>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
        }

        fn authorities() -> Vec<AuraId> {
            AuraAuthorities::<Runtime>::get().into_inner()
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> sp_consensus_grandpa::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: sp_consensus_grandpa::EquivocationProof<
                <Block as sp_runtime::traits::Block>::Hash,
                sp_runtime::traits::NumberFor<Block>,
            >,
            _key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_grandpa::SetId,
            _authority_id: sp_consensus_grandpa::AuthorityId,
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(uxt: <Block as sp_runtime::traits::Block>::Extrinsic, len: u32) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }

        fn query_fee_details(uxt: <Block as sp_runtime::traits::Block>::Extrinsic, len: u32) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }

        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            genesis_config_presets::get_preset(id)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            genesis_config_presets::preset_names()
        }
    }
}

pub mod genesis_config_presets {
    use super::*;
    use sp_genesis_builder::PresetId;

    pub const DEV_RUNTIME_PRESET: &str = "development";
    pub const LOCAL_RUNTIME_PRESET: &str = "local_testnet";

    pub fn development_config(
        sudo: AccountId,
        aura: Vec<AuraId>,
        grandpa: Vec<fg_primitives::AuthorityId>,
        guardians: Vec<AccountId>,
        endowed_accounts: Vec<AccountId>,
    ) -> RuntimeGenesisConfig {
        RuntimeGenesisConfig {
            system: Default::default(),
            balances: pallet_balances::GenesisConfig {
                balances: endowed_accounts
                    .into_iter()
                    .map(|account| (account, 1_000_000 * UNIT))
                    .collect(),
                ..Default::default()
            },
            sudo: pallet_sudo::GenesisConfig { key: Some(sudo) },
            aura: pallet_aura::GenesisConfig { authorities: aura },
            grandpa: pallet_grandpa::GenesisConfig {
                authorities: grandpa.into_iter().map(|authority| (authority, 1)).collect(),
                ..Default::default()
            },
            guardian_membership: pallet_membership::GenesisConfig {
                members: guardians.try_into().expect("guardian set fits MaxMembers"),
                ..Default::default()
            },
            guardian_collective: Default::default(),
            transaction_payment: Default::default(),
            treasury: Default::default(),
        }
    }

    pub fn get_preset(id: &Option<PresetId>) -> Option<Vec<u8>> {
        let preset: &str = id.as_ref()?.as_ref();
        if preset == DEV_RUNTIME_PRESET || preset == LOCAL_RUNTIME_PRESET {
            Some(
                serde_json::to_vec(&RuntimeGenesisConfig::default())
                    .expect("default solo genesis config serializes"),
            )
        } else {
            None
        }
    }

    pub fn preset_names() -> Vec<PresetId> {
        Vec::from([
            PresetId::from(DEV_RUNTIME_PRESET),
            PresetId::from(LOCAL_RUNTIME_PRESET),
        ])
    }
}

pub use genesis_config_presets::{DEV_RUNTIME_PRESET, LOCAL_RUNTIME_PRESET};

mod weights {
    pub use polkadot_sdk::frame_support::weights::constants::{
        BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight,
    };
}

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};
