#![cfg_attr(not(feature = "std"), no_std)]

use frame::prelude::*;
use vibly_primitives_common::{
    ActionCode, Amount, AssetId, ContentRef, Hash256, PaymentNonce, TimestampMs,
};
use vibly_primitives_identity::ActorId;

pub type PaymentIntentId = Hash256;

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
#[scale_info(skip_type_params(MaxNamespaceLen, MaxCidLen, MaxUriLen))]
pub struct PaymentAction<MaxNamespaceLen: Get<u32>, MaxCidLen: Get<u32>, MaxUriLen: Get<u32>> {
    pub namespace: BoundedVec<u8, MaxNamespaceLen>,
    pub action_code: ActionCode,
    pub payload_ref: Option<ContentRef<MaxCidLen, MaxUriLen>>,
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
pub enum SettlementMode {
    Direct,
    Hold,
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
pub enum PaymentIntentStatus {
    Requested,
    Funded,
    Claimed,
    Refunded,
    Cancelled,
    Expired,
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
#[scale_info(skip_type_params(MaxNamespaceLen, MaxCidLen, MaxUriLen))]
pub struct PaymentIntent<MaxNamespaceLen: Get<u32>, MaxCidLen: Get<u32>, MaxUriLen: Get<u32>> {
    pub intent_id: PaymentIntentId,
    pub payer: ActorId,
    pub payee: ActorId,
    pub asset_id: AssetId,
    pub amount: Amount,
    pub action: PaymentAction<MaxNamespaceLen, MaxCidLen, MaxUriLen>,
    pub memo_ref: Option<ContentRef<MaxCidLen, MaxUriLen>>,
    pub settlement_mode: SettlementMode,
    pub expires_at: Option<TimestampMs>,
    pub payer_nonce: PaymentNonce,
    pub status: PaymentIntentStatus,
    pub created_at: TimestampMs,
    pub updated_at: TimestampMs,
}
