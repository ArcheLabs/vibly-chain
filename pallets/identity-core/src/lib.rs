#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame::{prelude::*, traits::Time};
    use sp_runtime::traits::{BlakeTwo256, Hash as HashT};
    use vibly_primitives_common::{CapabilityMask, ContentRef, Hash256};
    use vibly_primitives_identity::{
        AuthorizedKeyRecord, IdentityAccess, IdentityId, IdentityStatus, KeyId, KeyPurpose,
        RootIdentity, TransportBinding, TransportBindingId, TransportBindingStatus, TransportKind,
        CAP_ADMIN, CAP_MANAGE_PAYMENT, CAP_MANAGE_POINTERS, CAP_MANAGE_TRANSPORTS,
    };

    type RootIdentityOf<T> = RootIdentity<
        <T as frame_system::Config>::AccountId,
        <T as Config>::MaxCidLen,
        <T as Config>::MaxUriLen,
    >;
    type AuthorizedKeyRecordOf<T> = AuthorizedKeyRecord<<T as frame_system::Config>::AccountId>;
    type TransportBindingOf<T> = TransportBinding<
        <T as Config>::MaxTransportAccountLen,
        <T as Config>::MaxCidLen,
        <T as Config>::MaxUriLen,
    >;
    type ContentRefOf<T> = ContentRef<<T as Config>::MaxCidLen, <T as Config>::MaxUriLen>;

    #[derive(Clone, Copy, Eq, PartialEq)]
    enum AccessScope {
        OwnerOrRecovery,
        PointerManager,
        TransportManager,
        PaymentManager,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type WeightInfo: crate::weights::WeightInfo;
        type TimeProvider: Time<Moment = u64>;
        #[pallet::constant]
        type MaxCidLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        #[pallet::constant]
        type MaxUriLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        #[pallet::constant]
        type MaxTransportAccountLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type Identities<T: Config> = StorageMap<_, Blake2_128Concat, IdentityId, RootIdentityOf<T>>;
    #[pallet::storage]
    pub type AuthorizedKeys<T: Config> =
        StorageMap<_, Blake2_128Concat, KeyId, AuthorizedKeyRecordOf<T>>;
    #[pallet::storage]
    pub type AuthorizedKeyIdByAccount<T: Config> =
        StorageMap<_, Blake2_128Concat, (IdentityId, T::AccountId), KeyId>;
    #[pallet::storage]
    pub type TransportBindings<T: Config> =
        StorageMap<_, Blake2_128Concat, TransportBindingId, TransportBindingOf<T>>;
    #[pallet::storage]
    pub type TransportBindingByIdentityAndLocator<T: Config> =
        StorageMap<_, Blake2_128Concat, Hash256, TransportBindingId>;
    #[pallet::storage]
    pub type NextIdentitySequence<T> = StorageValue<_, u64, ValueQuery>;
    #[pallet::storage]
    pub type NextTransportSequence<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        IdentityRegistered {
            identity_id: IdentityId,
            owner: T::AccountId,
        },
        OwnerKeyRotated {
            identity_id: IdentityId,
            old_owner: T::AccountId,
            new_owner: T::AccountId,
        },
        RecoveryKeySet {
            identity_id: IdentityId,
        },
        IdentityKeyAdded {
            identity_id: IdentityId,
            key_id: KeyId,
            purpose: KeyPurpose,
        },
        IdentityKeyRevoked {
            identity_id: IdentityId,
            key_id: KeyId,
        },
        ActiveProfileSet {
            identity_id: IdentityId,
        },
        ActiveAgentRegistrySet {
            identity_id: IdentityId,
        },
        ActiveAuthRegistrySet {
            identity_id: IdentityId,
        },
        ActiveRelationPolicySet {
            identity_id: IdentityId,
        },
        TransportBound {
            identity_id: IdentityId,
            binding_id: TransportBindingId,
            transport: TransportKind,
        },
        TransportVerified {
            identity_id: IdentityId,
            binding_id: TransportBindingId,
        },
        TransportRevoked {
            identity_id: IdentityId,
            binding_id: TransportBindingId,
        },
        IdentityFrozen {
            identity_id: IdentityId,
        },
        IdentityUnfrozen {
            identity_id: IdentityId,
        },
        IdentityDisabled {
            identity_id: IdentityId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        IdentityAlreadyExists,
        IdentityNotFound,
        InvalidState,
        AlreadyFrozen,
        NotFrozen,
        AlreadyDisabled,
        Unauthorized,
        OwnerKeyRequired,
        RecoveryNotConfigured,
        RecoveryNotAllowed,
        KeyAlreadyExists,
        KeyNotFound,
        KeyInvalid,
        KeyExpired,
        KeyRevoked,
        PointerInvalid,
        TransportBindingAlreadyExists,
        TransportBindingNotFound,
        TransportVerificationFailed,
        TransportNotAllowed,
        NonceInvalid,
        Overflow,
        InvalidInput,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_identity())]
        pub fn register_identity(
            origin: OriginFor<T>,
            recovery: Option<T::AccountId>,
            active_profile: Option<ContentRefOf<T>>,
            active_agent_registry: Option<ContentRefOf<T>>,
            active_auth_registry: Option<ContentRefOf<T>>,
            active_relation_policy: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            let identity_id = Self::next_identity_id()?;
            let now = Self::now();
            Identities::<T>::insert(
                identity_id,
                RootIdentity {
                    identity_id,
                    owner: owner.clone(),
                    recovery,
                    active_profile,
                    active_agent_registry,
                    active_auth_registry,
                    active_relation_policy,
                    status: IdentityStatus::Active,
                    nonce: 0,
                    created_at: now,
                    updated_at: now,
                },
            );
            Self::deposit_event(Event::IdentityRegistered { identity_id, owner });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::rotate_owner_key())]
        pub fn rotate_owner_key(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            new_owner: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, AccessScope::OwnerOrRecovery)?;
                Self::ensure_actor(identity, &who, AccessScope::OwnerOrRecovery)?;
                let old_owner = identity.owner.clone();
                identity.owner = new_owner.clone();
                Self::bump_identity(identity);
                Self::deposit_event(Event::OwnerKeyRotated {
                    identity_id,
                    old_owner,
                    new_owner,
                });
                Ok(())
            })
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_recovery_key())]
        pub fn set_recovery_key(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            new_recovery: Option<T::AccountId>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, AccessScope::OwnerOrRecovery)?;
                Self::ensure_actor(identity, &who, AccessScope::OwnerOrRecovery)?;
                identity.recovery = new_recovery;
                Self::bump_identity(identity);
                Self::deposit_event(Event::RecoveryKeySet { identity_id });
                Ok(())
            })
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::add_key())]
        pub fn add_key(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            account: T::AccountId,
            purpose: KeyPurpose,
            capability_mask: CapabilityMask,
            expires_at: Option<u64>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !matches!(purpose, KeyPurpose::Owner | KeyPurpose::Recovery),
                Error::<T>::KeyInvalid
            );
            let key_id = Self::key_id(identity_id, &account);
            ensure!(
                !AuthorizedKeys::<T>::contains_key(key_id),
                Error::<T>::KeyAlreadyExists
            );
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, AccessScope::PointerManager)?;
                Self::ensure_actor(identity, &who, AccessScope::PointerManager)?;
                ensure!(account != identity.owner, Error::<T>::KeyInvalid);
                ensure!(
                    identity.recovery.as_ref() != Some(&account),
                    Error::<T>::KeyInvalid
                );
                AuthorizedKeys::<T>::insert(
                    key_id,
                    AuthorizedKeyRecord {
                        key_id,
                        identity_id,
                        account: account.clone(),
                        purpose,
                        capability_mask,
                        expires_at,
                        revoked_at: None,
                        created_at: Self::now(),
                    },
                );
                AuthorizedKeyIdByAccount::<T>::insert((identity_id, account), key_id);
                Self::bump_identity(identity);
                Self::deposit_event(Event::IdentityKeyAdded {
                    identity_id,
                    key_id,
                    purpose,
                });
                Ok(())
            })
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::revoke_key())]
        pub fn revoke_key(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            key_id: KeyId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, AccessScope::PointerManager)?;
                Self::ensure_actor(identity, &who, AccessScope::PointerManager)?;
                let record = AuthorizedKeys::<T>::get(key_id).ok_or(Error::<T>::KeyNotFound)?;
                ensure!(record.identity_id == identity_id, Error::<T>::KeyNotFound);
                AuthorizedKeys::<T>::remove(key_id);
                AuthorizedKeyIdByAccount::<T>::remove((identity_id, record.account));
                Self::bump_identity(identity);
                Self::deposit_event(Event::IdentityKeyRevoked {
                    identity_id,
                    key_id,
                });
                Ok(())
            })
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::set_active_profile())]
        pub fn set_active_profile(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            profile: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            Self::mutate_pointer(
                origin,
                identity_id,
                AccessScope::PointerManager,
                |identity| identity.active_profile = profile,
                Event::ActiveProfileSet { identity_id },
            )
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::set_active_agent_registry())]
        pub fn set_active_agent_registry(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            registry: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            Self::mutate_pointer(
                origin,
                identity_id,
                AccessScope::PointerManager,
                |identity| identity.active_agent_registry = registry,
                Event::ActiveAgentRegistrySet { identity_id },
            )
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::set_active_auth_registry())]
        pub fn set_active_auth_registry(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            registry: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            Self::mutate_pointer(
                origin,
                identity_id,
                AccessScope::PointerManager,
                |identity| identity.active_auth_registry = registry,
                Event::ActiveAuthRegistrySet { identity_id },
            )
        }

        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::set_active_relation_policy())]
        pub fn set_active_relation_policy(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            policy: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            Self::mutate_pointer(
                origin,
                identity_id,
                AccessScope::PointerManager,
                |identity| identity.active_relation_policy = policy,
                Event::ActiveRelationPolicySet { identity_id },
            )
        }

        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::bind_transport())]
        pub fn bind_transport(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            transport: TransportKind,
            account: BoundedVec<u8, T::MaxTransportAccountLen>,
            proof_ref: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let locator = Self::transport_locator(identity_id, transport, &account);
            ensure!(
                !TransportBindingByIdentityAndLocator::<T>::contains_key(locator),
                Error::<T>::TransportBindingAlreadyExists
            );
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, AccessScope::TransportManager)?;
                Self::ensure_actor(identity, &who, AccessScope::TransportManager)?;
                let binding_id = Self::next_transport_id()?;
                let now = Self::now();
                TransportBindings::<T>::insert(
                    binding_id,
                    TransportBinding {
                        binding_id,
                        identity_id,
                        transport,
                        account: account.clone(),
                        proof_ref,
                        status: TransportBindingStatus::Pending,
                        created_at: now,
                        updated_at: now,
                    },
                );
                TransportBindingByIdentityAndLocator::<T>::insert(locator, binding_id);
                Self::bump_identity(identity);
                Self::deposit_event(Event::TransportBound {
                    identity_id,
                    binding_id,
                    transport,
                });
                Ok(())
            })
        }

        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::verify_transport())]
        pub fn verify_transport(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            binding_id: TransportBindingId,
            proof_ref: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut identity =
                Identities::<T>::get(identity_id).ok_or(Error::<T>::IdentityNotFound)?;
            Self::ensure_identity_state_for_mutation(&identity, AccessScope::OwnerOrRecovery)?;
            Self::ensure_actor(&identity, &who, AccessScope::OwnerOrRecovery)?;
            TransportBindings::<T>::try_mutate(binding_id, |maybe_binding| -> DispatchResult {
                let binding = maybe_binding
                    .as_mut()
                    .ok_or(Error::<T>::TransportBindingNotFound)?;
                ensure!(
                    binding.identity_id == identity_id,
                    Error::<T>::TransportBindingNotFound
                );
                ensure!(
                    binding.status == TransportBindingStatus::Pending,
                    Error::<T>::InvalidState
                );
                binding.status = TransportBindingStatus::Verified;
                binding.proof_ref = proof_ref;
                binding.updated_at = Self::now();
                Self::bump_identity(&mut identity);
                Identities::<T>::insert(identity_id, identity);
                Self::deposit_event(Event::TransportVerified {
                    identity_id,
                    binding_id,
                });
                Ok(())
            })
        }

        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::revoke_transport())]
        pub fn revoke_transport(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            binding_id: TransportBindingId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut identity =
                Identities::<T>::get(identity_id).ok_or(Error::<T>::IdentityNotFound)?;
            Self::ensure_identity_state_for_mutation(&identity, AccessScope::TransportManager)?;
            Self::ensure_actor(&identity, &who, AccessScope::TransportManager)?;
            TransportBindings::<T>::try_mutate(binding_id, |maybe_binding| -> DispatchResult {
                let binding = maybe_binding
                    .as_mut()
                    .ok_or(Error::<T>::TransportBindingNotFound)?;
                ensure!(
                    binding.identity_id == identity_id,
                    Error::<T>::TransportBindingNotFound
                );
                ensure!(
                    binding.status != TransportBindingStatus::Revoked,
                    Error::<T>::InvalidState
                );
                binding.status = TransportBindingStatus::Revoked;
                binding.updated_at = Self::now();
                Self::bump_identity(&mut identity);
                Identities::<T>::insert(identity_id, identity);
                Self::deposit_event(Event::TransportRevoked {
                    identity_id,
                    binding_id,
                });
                Ok(())
            })
        }

        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::freeze_identity())]
        pub fn freeze_identity(origin: OriginFor<T>, identity_id: IdentityId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                ensure!(
                    identity.status != IdentityStatus::Disabled,
                    Error::<T>::AlreadyDisabled
                );
                ensure!(
                    identity.status != IdentityStatus::Frozen,
                    Error::<T>::AlreadyFrozen
                );
                Self::ensure_actor(identity, &who, AccessScope::OwnerOrRecovery)?;
                identity.status = IdentityStatus::Frozen;
                Self::bump_identity(identity);
                Self::deposit_event(Event::IdentityFrozen { identity_id });
                Ok(())
            })
        }

        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::unfreeze_identity())]
        pub fn unfreeze_identity(origin: OriginFor<T>, identity_id: IdentityId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                ensure!(
                    identity.status != IdentityStatus::Disabled,
                    Error::<T>::AlreadyDisabled
                );
                ensure!(
                    identity.status == IdentityStatus::Frozen,
                    Error::<T>::NotFrozen
                );
                Self::ensure_actor(identity, &who, AccessScope::OwnerOrRecovery)?;
                identity.status = IdentityStatus::Active;
                Self::bump_identity(identity);
                Self::deposit_event(Event::IdentityUnfrozen { identity_id });
                Ok(())
            })
        }

        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::disable_identity())]
        pub fn disable_identity(origin: OriginFor<T>, identity_id: IdentityId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                ensure!(
                    identity.status != IdentityStatus::Disabled,
                    Error::<T>::AlreadyDisabled
                );
                Self::ensure_actor(identity, &who, AccessScope::OwnerOrRecovery)?;
                identity.status = IdentityStatus::Disabled;
                Self::bump_identity(identity);
                Self::deposit_event(Event::IdentityDisabled { identity_id });
                Ok(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        fn now() -> u64 {
            T::TimeProvider::now()
        }
        fn next_identity_id() -> Result<IdentityId, DispatchError> {
            let seq = NextIdentitySequence::<T>::get();
            NextIdentitySequence::<T>::put(seq.checked_add(1).ok_or(Error::<T>::Overflow)?);
            Ok(BlakeTwo256::hash_of(&(b"vibly/identity", seq)))
        }
        fn next_transport_id() -> Result<TransportBindingId, DispatchError> {
            let seq = NextTransportSequence::<T>::get();
            NextTransportSequence::<T>::put(seq.checked_add(1).ok_or(Error::<T>::Overflow)?);
            Ok(BlakeTwo256::hash_of(&(b"vibly/transport", seq)))
        }
        fn key_id(identity_id: IdentityId, account: &T::AccountId) -> KeyId {
            BlakeTwo256::hash_of(&(b"vibly/key", identity_id, account))
        }
        fn transport_locator(
            identity_id: IdentityId,
            transport: TransportKind,
            account: &BoundedVec<u8, T::MaxTransportAccountLen>,
        ) -> Hash256 {
            BlakeTwo256::hash_of(&(b"vibly/transport-locator", identity_id, transport, account))
        }
        fn bump_identity(identity: &mut RootIdentityOf<T>) {
            identity.nonce = identity.nonce.saturating_add(1);
            identity.updated_at = Self::now();
        }
        fn ensure_identity_state_for_mutation(
            identity: &RootIdentityOf<T>,
            scope: AccessScope,
        ) -> Result<(), DispatchError> {
            match identity.status {
                IdentityStatus::Disabled => Err(Error::<T>::AlreadyDisabled.into()),
                IdentityStatus::Frozen => match scope {
                    AccessScope::OwnerOrRecovery => Ok(()),
                    _ => Err(Error::<T>::InvalidState.into()),
                },
                IdentityStatus::Active => Ok(()),
            }
        }
        fn ensure_actor(
            identity: &RootIdentityOf<T>,
            who: &T::AccountId,
            scope: AccessScope,
        ) -> Result<(), DispatchError> {
            if *who == identity.owner {
                return Ok(());
            }
            if identity.recovery.as_ref() == Some(who) {
                return match scope {
                    AccessScope::OwnerOrRecovery => Ok(()),
                    _ => Err(Error::<T>::RecoveryNotAllowed.into()),
                };
            }
            let key_id = AuthorizedKeyIdByAccount::<T>::get((identity.identity_id, who.clone()))
                .ok_or(Error::<T>::Unauthorized)?;
            let record = AuthorizedKeys::<T>::get(key_id).ok_or(Error::<T>::KeyNotFound)?;
            Self::ensure_record_active(&record)?;
            let required = match scope {
                AccessScope::OwnerOrRecovery => return Err(Error::<T>::Unauthorized.into()),
                AccessScope::PointerManager => CAP_MANAGE_POINTERS,
                AccessScope::TransportManager => CAP_MANAGE_TRANSPORTS,
                AccessScope::PaymentManager => CAP_MANAGE_PAYMENT,
            };
            ensure!(
                record.capability_mask & (required | CAP_ADMIN) != 0,
                Error::<T>::Unauthorized
            );
            Ok(())
        }
        fn ensure_record_active(record: &AuthorizedKeyRecordOf<T>) -> Result<(), DispatchError> {
            ensure!(record.revoked_at.is_none(), Error::<T>::KeyRevoked);
            if let Some(expires_at) = record.expires_at {
                ensure!(Self::now() < expires_at, Error::<T>::KeyExpired);
            }
            Ok(())
        }
        fn mutate_pointer(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            scope: AccessScope,
            mutate: impl FnOnce(&mut RootIdentityOf<T>),
            event: Event<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, scope)?;
                Self::ensure_actor(identity, &who, scope)?;
                mutate(identity);
                Self::bump_identity(identity);
                Self::deposit_event(event);
                Ok(())
            })
        }
    }

    impl<T: Config> IdentityAccess<T::AccountId> for Pallet<T> {
        fn identity_exists(identity_id: &IdentityId) -> bool {
            Identities::<T>::contains_key(identity_id)
        }
        fn owner_account(identity_id: &IdentityId) -> Option<T::AccountId> {
            Identities::<T>::get(identity_id).map(|identity| identity.owner)
        }
        fn ensure_can_manage_payment(
            identity_id: &IdentityId,
            who: &T::AccountId,
        ) -> DispatchResult {
            let identity = Identities::<T>::get(identity_id).ok_or(Error::<T>::IdentityNotFound)?;
            ensure!(
                identity.status == IdentityStatus::Active,
                Error::<T>::InvalidState
            );
            Self::ensure_actor(&identity, who, AccessScope::PaymentManager)
        }
        fn ensure_can_claim_payment(
            identity_id: &IdentityId,
            who: &T::AccountId,
        ) -> DispatchResult {
            Self::ensure_can_manage_payment(identity_id, who)
        }
    }
}
