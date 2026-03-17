#![cfg_attr(not(feature = "std"), no_std)]

use frame::prelude::*;
use vibly_primitives_common::{CapabilityMask, ContentRef, Hash256, IdentityNonce, TimestampMs};

pub type IdentityId = Hash256;
pub type ActorId = IdentityId;
pub type KeyId = Hash256;
pub type TransportBindingId = Hash256;
pub type PurposeCode = u16;

pub const CAP_MANAGE_POINTERS: CapabilityMask = 1 << 0;
pub const CAP_MANAGE_TRANSPORTS: CapabilityMask = 1 << 1;
pub const CAP_MANAGE_PAYMENT: CapabilityMask = 1 << 2;
pub const CAP_ADMIN: CapabilityMask = 1 << 3;

#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum IdentityStatus {
    Active,
    Frozen,
    Disabled,
}

#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum KeyPurpose {
    Owner,
    Recovery,
    Admin,
    Session,
    Butler,
    Finance,
    Custom(PurposeCode),
}

#[derive(
    Clone,
    Eq,
    PartialEq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxCidLen, MaxUriLen, AccountId))]
pub struct RootIdentity<AccountId, MaxCidLen: Get<u32>, MaxUriLen: Get<u32>> {
    pub identity_id: IdentityId,
    pub owner: AccountId,
    pub recovery: Option<AccountId>,
    pub active_profile: Option<ContentRef<MaxCidLen, MaxUriLen>>,
    pub active_agent_registry: Option<ContentRef<MaxCidLen, MaxUriLen>>,
    pub active_auth_registry: Option<ContentRef<MaxCidLen, MaxUriLen>>,
    pub active_relation_policy: Option<ContentRef<MaxCidLen, MaxUriLen>>,
    pub status: IdentityStatus,
    pub nonce: IdentityNonce,
    pub created_at: TimestampMs,
    pub updated_at: TimestampMs,
}

#[derive(
    Clone,
    Eq,
    PartialEq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(AccountId))]
pub struct AuthorizedKeyRecord<AccountId> {
    pub key_id: KeyId,
    pub identity_id: IdentityId,
    pub account: AccountId,
    pub purpose: KeyPurpose,
    pub capability_mask: CapabilityMask,
    pub expires_at: Option<TimestampMs>,
    pub revoked_at: Option<TimestampMs>,
    pub created_at: TimestampMs,
}

#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum TransportKind {
    Matrix,
    Discord,
    Telegram,
    Email,
    Custom(u16),
}

#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum TransportBindingStatus {
    Pending,
    Verified,
    Revoked,
}

#[derive(
    Clone,
    Eq,
    PartialEq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxTransportAccountLen, MaxCidLen, MaxUriLen))]
pub struct TransportBinding<
    MaxTransportAccountLen: Get<u32>,
    MaxCidLen: Get<u32>,
    MaxUriLen: Get<u32>,
> {
    pub binding_id: TransportBindingId,
    pub identity_id: IdentityId,
    pub transport: TransportKind,
    pub account: BoundedVec<u8, MaxTransportAccountLen>,
    pub proof_ref: Option<ContentRef<MaxCidLen, MaxUriLen>>,
    pub status: TransportBindingStatus,
    pub created_at: TimestampMs,
    pub updated_at: TimestampMs,
}

pub trait IdentityAccess<AccountId> {
    fn identity_exists(identity_id: &IdentityId) -> bool;
    fn owner_account(identity_id: &IdentityId) -> Option<AccountId>;
    fn ensure_can_manage_payment(identity_id: &IdentityId, who: &AccountId) -> DispatchResult;
    fn ensure_can_claim_payment(identity_id: &IdentityId, who: &AccountId) -> DispatchResult;
}
