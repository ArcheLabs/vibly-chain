#![cfg_attr(not(feature = "std"), no_std)]
//! Cumulative Merkle claim pallet for Get VIB allocations.

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame::pallet]
pub mod pallet {
    use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
    use frame::{
        prelude::*,
        traits::{
            tokens::{fungible::Mutate, Preservation},
            EnsureOrigin,
        },
    };
    use scale_info::TypeInfo;
    use sp_io::hashing::blake2_256;
    use vibly_primitives_common::Amount;

    const LEAF_DOMAIN: &[u8] = b"VIB_CLAIM_LEAF_V1";
    const NODE_DOMAIN: &[u8] = b"VIB_CLAIM_NODE_V1";

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct RootInfo<BoundedNetworkId, BlockNumber> {
        pub network_id: BoundedNetworkId,
        pub root_version: u32,
        pub merkle_root: [u8; 32],
        pub total_cumulative_amount: Amount,
        pub metadata_hash: [u8; 32],
        pub updated_at: BlockNumber,
    }

    #[derive(
        Clone,
        Encode,
        Decode,
        DecodeWithMemTracking,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    pub enum ProofPosition {
        Left,
        Right,
    }

    #[derive(
        Clone,
        Encode,
        Decode,
        DecodeWithMemTracking,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    pub struct MerkleProofItem {
        pub position: ProofPosition,
        pub hash: [u8; 32],
    }

    #[derive(Encode)]
    struct ClaimLeafV1<AccountId, BoundedIdentityId> {
        account_id: AccountId,
        identity_id: BoundedIdentityId,
        cumulative_amount: Amount,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Mutate<Self::AccountId, Balance = Amount>;
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        #[pallet::constant]
        type ClaimReserveAccount: Get<Self::AccountId>;
        #[pallet::constant]
        type MaxNetworkIdLen: Get<u32>;
        #[pallet::constant]
        type MaxIdentityIdLen: Get<u32>;
        #[pallet::constant]
        type MaxProofLen: Get<u32>;
    }

    pub type BoundedNetworkIdOf<T> = BoundedVec<u8, <T as Config>::MaxNetworkIdLen>;
    pub type BoundedIdentityIdOf<T> = BoundedVec<u8, <T as Config>::MaxIdentityIdLen>;
    pub type BoundedProofOf<T> = BoundedVec<MerkleProofItem, <T as Config>::MaxProofLen>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type ClaimRoot<T: Config> =
        StorageValue<_, RootInfo<BoundedNetworkIdOf<T>, BlockNumberFor<T>>, OptionQuery>;

    #[pallet::storage]
    pub type ClaimedAmount<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Amount, ValueQuery>;

    #[pallet::storage]
    pub type ClaimPaused<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::storage]
    pub type ClaimRootPublisher<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ClaimRootUpdated {
            network_id: BoundedNetworkIdOf<T>,
            root_version: u32,
            merkle_root: [u8; 32],
            total_cumulative_amount: Amount,
            metadata_hash: [u8; 32],
        },
        VibClaimed {
            account_id: T::AccountId,
            identity_id: BoundedIdentityIdOf<T>,
            root_version: u32,
            cumulative_amount: Amount,
            claimed_delta: Amount,
        },
        ClaimPausedUpdated {
            paused: bool,
        },
        ClaimRootPublisherUpdated {
            publisher: Option<T::AccountId>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ClaimRootNotSet,
        InvalidNetworkId,
        InvalidMerkleProof,
        NothingToClaim,
        ClaimPaused,
        AmountOverflow,
        UnauthorizedRootUpdate,
        InvalidRootVersion,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn set_claim_root(
            origin: OriginFor<T>,
            network_id: BoundedNetworkIdOf<T>,
            root_version: u32,
            merkle_root: [u8; 32],
            total_cumulative_amount: Amount,
            metadata_hash: [u8; 32],
        ) -> DispatchResult {
            Self::ensure_root_update_origin(origin)?;
            if let Some(current) = ClaimRoot::<T>::get() {
                ensure!(
                    root_version > current.root_version,
                    Error::<T>::InvalidRootVersion
                );
            }
            let info = RootInfo {
                network_id: network_id.clone(),
                root_version,
                merkle_root,
                total_cumulative_amount,
                metadata_hash,
                updated_at: frame_system::Pallet::<T>::block_number(),
            };
            ClaimRoot::<T>::put(info);
            Self::deposit_event(Event::ClaimRootUpdated {
                network_id,
                root_version,
                merkle_root,
                total_cumulative_amount,
                metadata_hash,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn claim(
            origin: OriginFor<T>,
            network_id: BoundedNetworkIdOf<T>,
            root_version: u32,
            identity_id: BoundedIdentityIdOf<T>,
            cumulative_amount: Amount,
            proof: BoundedProofOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!ClaimPaused::<T>::get(), Error::<T>::ClaimPaused);
            let root = ClaimRoot::<T>::get().ok_or(Error::<T>::ClaimRootNotSet)?;
            ensure!(root.network_id == network_id, Error::<T>::InvalidNetworkId);
            ensure!(
                root.root_version == root_version,
                Error::<T>::InvalidRootVersion
            );
            let leaf = Self::hash_leaf(&who, &identity_id, cumulative_amount);
            let computed = Self::apply_proof(leaf, proof);
            ensure!(computed == root.merkle_root, Error::<T>::InvalidMerkleProof);

            let already_claimed = ClaimedAmount::<T>::get(&who);
            ensure!(
                cumulative_amount > already_claimed,
                Error::<T>::NothingToClaim
            );
            let delta = cumulative_amount
                .checked_sub(already_claimed)
                .ok_or(Error::<T>::AmountOverflow)?;
            T::Currency::transfer(
                &T::ClaimReserveAccount::get(),
                &who,
                delta,
                Preservation::Expendable,
            )?;
            ClaimedAmount::<T>::insert(&who, cumulative_amount);
            Self::deposit_event(Event::VibClaimed {
                account_id: who,
                identity_id,
                root_version,
                cumulative_amount,
                claimed_delta: delta,
            });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn set_claim_paused(origin: OriginFor<T>, paused: bool) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)
                .map_err(|_| Error::<T>::UnauthorizedRootUpdate)?;
            ClaimPaused::<T>::put(paused);
            Self::deposit_event(Event::ClaimPausedUpdated { paused });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(10_000)]
        pub fn set_claim_root_publisher(
            origin: OriginFor<T>,
            publisher: Option<T::AccountId>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)
                .map_err(|_| Error::<T>::UnauthorizedRootUpdate)?;
            match publisher.clone() {
                Some(account) => ClaimRootPublisher::<T>::put(account),
                None => ClaimRootPublisher::<T>::kill(),
            }
            Self::deposit_event(Event::ClaimRootPublisherUpdated { publisher });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn ensure_root_update_origin(origin: OriginFor<T>) -> DispatchResult {
            match T::AdminOrigin::try_origin(origin) {
                Ok(_) => Ok(()),
                Err(origin) => {
                    let who =
                        ensure_signed(origin).map_err(|_| Error::<T>::UnauthorizedRootUpdate)?;
                    ensure!(
                        ClaimRootPublisher::<T>::get().as_ref() == Some(&who),
                        Error::<T>::UnauthorizedRootUpdate
                    );
                    Ok(())
                }
            }
        }

        pub fn hash_leaf(
            account_id: &T::AccountId,
            identity_id: &BoundedIdentityIdOf<T>,
            cumulative_amount: Amount,
        ) -> [u8; 32] {
            let leaf = ClaimLeafV1 {
                account_id,
                identity_id,
                cumulative_amount,
            };
            let mut encoded = LEAF_DOMAIN.to_vec();
            encoded.extend(leaf.encode());
            blake2_256(&encoded)
        }

        pub fn hash_node(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
            let mut encoded = NODE_DOMAIN.to_vec();
            encoded.extend(left);
            encoded.extend(right);
            blake2_256(&encoded)
        }

        fn apply_proof(mut hash: [u8; 32], proof: BoundedProofOf<T>) -> [u8; 32] {
            for item in proof {
                hash = match item.position {
                    ProofPosition::Left => Self::hash_node(item.hash, hash),
                    ProofPosition::Right => Self::hash_node(hash, item.hash),
                };
            }
            hash
        }
    }
}
