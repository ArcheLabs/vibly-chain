#![cfg_attr(not(feature = "std"), no_std)]

use frame::prelude::*;
use vibly_primitives_common::{
    ActionCode, Amount, AssetId, ContentRef, Hash256, PaymentNonce, TimestampMs,
};
use vibly_primitives_identity::ActorId;

/// Stable identifier for a payment intent.
pub type PaymentIntentId = Hash256;

/// Application action that a payment intent is meant to settle.
///
/// The chain stores the namespace and action code directly, while larger or private payloads
/// should be referenced through `payload_ref`.
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
    /// Application namespace, such as a service or protocol name.
    pub namespace: BoundedVec<u8, MaxNamespaceLen>,
    /// Application-defined action code within the namespace.
    pub action_code: ActionCode,
    /// Optional off-chain payload pointer.
    pub payload_ref: Option<ContentRef<MaxCidLen, MaxUriLen>>,
}

/// How funds move when an intent is funded.
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
    /// Transfer immediately from the caller to the payee owner account.
    Direct,
    /// Hold funds from the caller until the intent is claimed or refunded.
    Hold,
}

/// Lifecycle state for a payment intent.
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
    /// Intent exists but has not moved funds yet.
    Requested,
    /// Funds are held and can be claimed or refunded.
    Funded,
    /// Funds have been transferred to the payee owner.
    Claimed,
    /// Held funds have been released back to the funding account.
    Refunded,
    /// Requested intent was cancelled before funding.
    Cancelled,
    /// Requested intent reached its expiry and was marked expired.
    Expired,
}

/// On-chain payment intent record.
///
/// Payment intents connect two identity-backed actors, a native asset amount, and an
/// application action. The pallet currently supports native asset `asset_id = 0`.
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
    /// Stable intent identifier supplied by the caller.
    pub intent_id: PaymentIntentId,
    /// Payer identity that authorizes intent creation and funding.
    pub payer: ActorId,
    /// Payee identity that authorizes claiming held funds.
    pub payee: ActorId,
    /// Asset identifier; v1 accepts only native asset `0`.
    pub asset_id: AssetId,
    /// Amount denominated in the native balance type.
    pub amount: Amount,
    /// Application action being settled.
    pub action: PaymentAction<MaxNamespaceLen, MaxCidLen, MaxUriLen>,
    /// Optional off-chain memo pointer.
    pub memo_ref: Option<ContentRef<MaxCidLen, MaxUriLen>>,
    /// Settlement behavior used when funding the intent.
    pub settlement_mode: SettlementMode,
    /// Optional expiration timestamp in milliseconds.
    pub expires_at: Option<TimestampMs>,
    /// Payer nonce reserved for replay protection in future signed flows.
    pub payer_nonce: PaymentNonce,
    /// Current lifecycle status.
    pub status: PaymentIntentStatus,
    /// Creation timestamp in milliseconds.
    pub created_at: TimestampMs,
    /// Last mutation timestamp in milliseconds.
    pub updated_at: TimestampMs,
}
