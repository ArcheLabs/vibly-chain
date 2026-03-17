#![cfg_attr(not(feature = "std"), no_std)]

use frame::prelude::*;
use sp_core::H256;

pub type Hash256 = H256;
pub type TimestampMs = u64;
pub type Amount = u128;
pub type AssetId = u32;
pub type IdentityNonce = u64;
pub type PaymentNonce = u64;
pub type CapabilityMask = u128;
pub type ActionCode = u32;
pub type BoundedBytes<MaxLen> = BoundedVec<u8, MaxLen>;

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
#[scale_info(skip_type_params(MaxCidLen, MaxUriLen))]
pub enum ContentRef<MaxCidLen: Get<u32>, MaxUriLen: Get<u32>> {
    Hash(Hash256),
    Cid(BoundedVec<u8, MaxCidLen>),
    UriHash {
        uri: BoundedVec<u8, MaxUriLen>,
        hash: Hash256,
    },
}
