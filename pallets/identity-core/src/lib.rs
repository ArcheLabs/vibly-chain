#![cfg_attr(not(feature = "std"), no_std)]
//! Core identity pallet for vibly-chain.
//!
//! This pallet stores root identities, delegated keys, active content pointers, and
//! external transport bindings. Owner accounts have full authority. Optional recovery
//! accounts can perform owner/recovery lifecycle actions, while delegated keys are
//! constrained by capability bits.

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
        AuthorizedKeyRecord, EvmAddress, IdentityAccess, IdentityId, IdentityStatus, KeyId,
        KeyPurpose, RootIdentity, TransportBinding, TransportBindingId, TransportBindingStatus,
        TransportKind, CAP_ADMIN, CAP_MANAGE_PAYMENT, CAP_MANAGE_POINTERS, CAP_MANAGE_TRANSPORTS,
        CAP_REGISTER_AGENT,
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
        /// Owner or recovery actions, such as owner rotation and freezing.
        OwnerOrRecovery,
        /// Active content pointer management.
        PointerManager,
        /// External transport binding management.
        TransportManager,
        /// Payment intent management for cross-pallet authorization.
        PaymentManager,
        /// Agent registration only.
        AgentRegistrar,
    }

    #[pallet::config]
    /// Runtime configuration for identity storage and authorization.
    pub trait Config: frame_system::Config {
        /// Weight provider for dispatchable calls.
        type WeightInfo: crate::weights::WeightInfo;
        /// Timestamp provider returning milliseconds.
        type TimeProvider: Time<Moment = u64>;
        /// Maximum encoded length for content CIDs.
        #[pallet::constant]
        type MaxCidLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        /// Maximum encoded length for content URIs.
        #[pallet::constant]
        type MaxUriLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        /// Maximum encoded length for external transport account locators.
        #[pallet::constant]
        type MaxTransportAccountLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    /// Root identity records keyed by generated identity id.
    pub type Identities<T: Config> = StorageMap<_, Blake2_128Concat, IdentityId, RootIdentityOf<T>>;
    #[pallet::storage]
    /// Delegated key records keyed by generated key id.
    pub type AuthorizedKeys<T: Config> =
        StorageMap<_, Blake2_128Concat, KeyId, AuthorizedKeyRecordOf<T>>;
    #[pallet::storage]
    /// Reverse lookup from `(identity_id, account)` to delegated key id.
    pub type AuthorizedKeyIdByAccount<T: Config> =
        StorageMap<_, Blake2_128Concat, (IdentityId, T::AccountId), KeyId>;
    #[pallet::storage]
    /// External transport binding records keyed by generated binding id.
    pub type TransportBindings<T: Config> =
        StorageMap<_, Blake2_128Concat, TransportBindingId, TransportBindingOf<T>>;
    #[pallet::storage]
    /// Uniqueness index for `(identity_id, transport, transport_account)`.
    pub type TransportBindingByIdentityAndLocator<T: Config> =
        StorageMap<_, Blake2_128Concat, Hash256, TransportBindingId>;
    #[pallet::storage]
    /// Reverse lookup from an EVM root address to the Vibly identity it controls.
    pub type IdentityIdByEvmAddress<T: Config> =
        StorageMap<_, Blake2_128Concat, EvmAddress, IdentityId>;
    #[pallet::storage]
    /// EVM root address bound to a Vibly identity.
    pub type EvmAddressByIdentityId<T: Config> =
        StorageMap<_, Blake2_128Concat, IdentityId, EvmAddress>;
    #[pallet::storage]
    /// Current restricted AgentRegistrar account for an identity.
    pub type AgentRegistrarByIdentityId<T: Config> =
        StorageMap<_, Blake2_128Concat, IdentityId, T::AccountId>;
    #[pallet::storage]
    /// Monotonic sequence used to derive identity ids.
    pub type NextIdentitySequence<T> = StorageValue<_, u64, ValueQuery>;
    #[pallet::storage]
    /// Monotonic sequence used to derive transport binding ids.
    pub type NextTransportSequence<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    /// Identity lifecycle and pointer events.
    pub enum Event<T: Config> {
        /// A new root identity was registered.
        IdentityRegistered {
            identity_id: IdentityId,
            owner: T::AccountId,
        },
        /// The owner account was rotated.
        OwnerKeyRotated {
            identity_id: IdentityId,
            old_owner: T::AccountId,
            new_owner: T::AccountId,
        },
        /// The optional recovery account was changed.
        RecoveryKeySet { identity_id: IdentityId },
        /// A delegated key was added.
        IdentityKeyAdded {
            identity_id: IdentityId,
            key_id: KeyId,
            purpose: KeyPurpose,
        },
        /// A delegated key was removed.
        IdentityKeyRevoked {
            identity_id: IdentityId,
            key_id: KeyId,
        },
        /// The active profile pointer changed.
        ActiveProfileSet { identity_id: IdentityId },
        /// The active agent registry pointer changed.
        ActiveAgentRegistrySet { identity_id: IdentityId },
        /// The active authorization registry pointer changed.
        ActiveAuthRegistrySet { identity_id: IdentityId },
        /// The active relation policy pointer changed.
        ActiveRelationPolicySet { identity_id: IdentityId },
        /// A transport binding was created in pending state.
        TransportBound {
            identity_id: IdentityId,
            binding_id: TransportBindingId,
            transport: TransportKind,
        },
        /// A pending transport binding was verified.
        TransportVerified {
            identity_id: IdentityId,
            binding_id: TransportBindingId,
        },
        /// A transport binding was revoked.
        TransportRevoked {
            identity_id: IdentityId,
            binding_id: TransportBindingId,
        },
        /// An identity was frozen.
        IdentityFrozen { identity_id: IdentityId },
        /// A frozen identity was reactivated.
        IdentityUnfrozen { identity_id: IdentityId },
        /// An identity was permanently disabled.
        IdentityDisabled { identity_id: IdentityId },
        /// An EVM address was bound as an external identity root.
        EvmRootBound {
            identity_id: IdentityId,
            evm_address: EvmAddress,
        },
        /// An AgentRegistrar account was authorized for an identity.
        AgentRegistrarSet {
            identity_id: IdentityId,
            agent_registrar: T::AccountId,
        },
        /// An AgentRegistrar account was revoked for an identity.
        AgentRegistrarRevoked { identity_id: IdentityId },
    }

    #[pallet::error]
    /// Identity pallet errors.
    pub enum Error<T> {
        /// The requested identity id is already in use.
        IdentityAlreadyExists,
        /// No identity exists for the requested id.
        IdentityNotFound,
        /// The identity or binding is not in a valid state for the operation.
        InvalidState,
        /// The identity is already frozen.
        AlreadyFrozen,
        /// The identity is not frozen.
        NotFrozen,
        /// The identity is already disabled.
        AlreadyDisabled,
        /// Caller does not have the required owner, recovery, or delegated capability.
        Unauthorized,
        /// Owner-only key material was required.
        OwnerKeyRequired,
        /// No recovery account is configured.
        RecoveryNotConfigured,
        /// Recovery account cannot perform this operation.
        RecoveryNotAllowed,
        /// A delegated key already exists for the account.
        KeyAlreadyExists,
        /// The requested delegated key does not exist.
        KeyNotFound,
        /// The key is structurally invalid for this identity.
        KeyInvalid,
        /// The delegated key has expired.
        KeyExpired,
        /// The delegated key has been revoked.
        KeyRevoked,
        /// A content pointer is invalid.
        PointerInvalid,
        /// This transport account is already bound for the identity.
        TransportBindingAlreadyExists,
        /// The requested transport binding does not exist.
        TransportBindingNotFound,
        /// Transport verification proof failed validation.
        TransportVerificationFailed,
        /// Transport kind or account is not allowed.
        TransportNotAllowed,
        /// The supplied nonce is invalid.
        NonceInvalid,
        /// A sequence or counter overflowed.
        Overflow,
        /// Generic invalid input.
        InvalidInput,
        /// The EVM root address is already bound to an identity.
        EvmAddressAlreadyBound,
        /// The EVM root address is not bound to an identity.
        EvmAddressNotBound,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new root identity owned by the signed origin.
        ///
        /// The identity id is derived from a pallet-local sequence. Optional active pointers can
        /// be supplied at registration time, and all later mutations bump the identity nonce.
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

        /// Rotate the owner account for an active or frozen identity.
        ///
        /// The current owner or configured recovery account may call this. Delegated keys cannot
        /// rotate the owner.
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

        /// Set or clear the recovery account.
        ///
        /// Recovery is intentionally limited to owner/recovery lifecycle actions and cannot manage
        /// pointers, transports, or payment authority.
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

        /// Add a delegated key with explicit capability bits.
        ///
        /// Owner and recovery purposes are reserved for the root identity fields and cannot be
        /// added as delegated keys. The key id is derived from `(identity_id, account)`.
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

        /// Revoke a delegated key by removing its authorization record.
        ///
        /// The caller must be the owner or hold pointer-management authority. Removing the key also
        /// removes the account reverse lookup.
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

        /// Set or clear the active profile content pointer.
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

        /// Set or clear the active agent registry content pointer.
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

        /// Set or clear the active authorization registry content pointer.
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

        /// Set or clear the active relation policy content pointer.
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

        /// Create a pending external transport binding for an identity.
        ///
        /// Bindings are unique per `(identity_id, transport, account)` locator. A separate
        /// verification step lets owner or recovery confirm the binding proof.
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

        /// Verify a pending transport binding.
        ///
        /// Only the owner or recovery account may verify, because verification asserts control over
        /// the identity rather than over delegated transport-management authority.
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

        /// Revoke an existing transport binding.
        ///
        /// Revocation keeps the binding record for auditability and marks it as no longer valid.
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

        /// Freeze an identity.
        ///
        /// Frozen identities reject delegated pointer, transport, and payment-management actions,
        /// but owner/recovery can still unfreeze, rotate owner, or disable.
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

        /// Reactivate a frozen identity.
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

        /// Permanently disable an identity.
        ///
        /// Disabled identities cannot be mutated or used for payment authorization.
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
        /// Return the runtime timestamp in milliseconds.
        fn now() -> u64 {
            T::TimeProvider::now()
        }
        /// Generate the next identity id from a domain-separated sequence hash.
        fn next_identity_id() -> Result<IdentityId, DispatchError> {
            let seq = NextIdentitySequence::<T>::get();
            NextIdentitySequence::<T>::put(seq.checked_add(1).ok_or(Error::<T>::Overflow)?);
            Ok(BlakeTwo256::hash_of(&(b"vibly/identity", seq)))
        }
        /// Generate the next transport binding id from a domain-separated sequence hash.
        fn next_transport_id() -> Result<TransportBindingId, DispatchError> {
            let seq = NextTransportSequence::<T>::get();
            NextTransportSequence::<T>::put(seq.checked_add(1).ok_or(Error::<T>::Overflow)?);
            Ok(BlakeTwo256::hash_of(&(b"vibly/transport", seq)))
        }
        /// Derive a delegated key id for an identity/account pair.
        fn key_id(identity_id: IdentityId, account: &T::AccountId) -> KeyId {
            BlakeTwo256::hash_of(&(b"vibly/key", identity_id, account))
        }
        /// Register a Vibly identity from a relayer-verified EVM onboarding flow.
        pub fn register_evm_identity_from_relayer(
            evm_address: EvmAddress,
            owner: T::AccountId,
            agent_registrar: T::AccountId,
        ) -> Result<IdentityId, DispatchError> {
            ensure!(
                !IdentityIdByEvmAddress::<T>::contains_key(evm_address),
                Error::<T>::EvmAddressAlreadyBound
            );
            let identity_id = Self::next_identity_id()?;
            let now = Self::now();
            Identities::<T>::insert(
                identity_id,
                RootIdentity {
                    identity_id,
                    owner: owner.clone(),
                    recovery: None,
                    active_profile: None,
                    active_agent_registry: None,
                    active_auth_registry: None,
                    active_relation_policy: None,
                    status: IdentityStatus::Active,
                    nonce: 0,
                    created_at: now,
                    updated_at: now,
                },
            );
            IdentityIdByEvmAddress::<T>::insert(evm_address, identity_id);
            EvmAddressByIdentityId::<T>::insert(identity_id, evm_address);
            Self::set_agent_registrar_record(identity_id, agent_registrar.clone(), now)?;
            Self::deposit_event(Event::IdentityRegistered { identity_id, owner });
            Self::deposit_event(Event::EvmRootBound {
                identity_id,
                evm_address,
            });
            Self::deposit_event(Event::AgentRegistrarSet {
                identity_id,
                agent_registrar,
            });
            Ok(identity_id)
        }
        /// Rotate the Vibly owner account after an off-chain EVM authorization was verified.
        pub fn rotate_owner_for_evm_root(
            evm_address: EvmAddress,
            new_owner: T::AccountId,
        ) -> DispatchResult {
            let identity_id = IdentityIdByEvmAddress::<T>::get(evm_address)
                .ok_or(Error::<T>::EvmAddressNotBound)?;
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, AccessScope::OwnerOrRecovery)?;
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
        /// Set or replace the restricted AgentRegistrar for an identity.
        pub fn set_agent_registrar_from_owner(
            identity_id: IdentityId,
            owner: &T::AccountId,
            agent_registrar: T::AccountId,
        ) -> DispatchResult {
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, AccessScope::OwnerOrRecovery)?;
                Self::ensure_actor(identity, owner, AccessScope::OwnerOrRecovery)?;
                Self::set_agent_registrar_record(
                    identity_id,
                    agent_registrar.clone(),
                    Self::now(),
                )?;
                Self::bump_identity(identity);
                Self::deposit_event(Event::AgentRegistrarSet {
                    identity_id,
                    agent_registrar,
                });
                Ok(())
            })
        }
        /// Revoke the restricted AgentRegistrar for an identity.
        pub fn revoke_agent_registrar_from_owner(
            identity_id: IdentityId,
            owner: &T::AccountId,
        ) -> DispatchResult {
            Identities::<T>::try_mutate(identity_id, |maybe_identity| -> DispatchResult {
                let identity = maybe_identity
                    .as_mut()
                    .ok_or(Error::<T>::IdentityNotFound)?;
                Self::ensure_identity_state_for_mutation(identity, AccessScope::OwnerOrRecovery)?;
                Self::ensure_actor(identity, owner, AccessScope::OwnerOrRecovery)?;
                if let Some(account) = AgentRegistrarByIdentityId::<T>::take(identity_id) {
                    let key_id = Self::key_id(identity_id, &account);
                    AuthorizedKeys::<T>::remove(key_id);
                    AuthorizedKeyIdByAccount::<T>::remove((identity_id, account));
                }
                Self::bump_identity(identity);
                Self::deposit_event(Event::AgentRegistrarRevoked { identity_id });
                Ok(())
            })
        }
        fn set_agent_registrar_record(
            identity_id: IdentityId,
            agent_registrar: T::AccountId,
            now: u64,
        ) -> DispatchResult {
            if let Some(previous) = AgentRegistrarByIdentityId::<T>::get(identity_id) {
                let previous_key_id = Self::key_id(identity_id, &previous);
                AuthorizedKeys::<T>::remove(previous_key_id);
                AuthorizedKeyIdByAccount::<T>::remove((identity_id, previous));
            }
            let key_id = Self::key_id(identity_id, &agent_registrar);
            AuthorizedKeys::<T>::insert(
                key_id,
                AuthorizedKeyRecord {
                    key_id,
                    identity_id,
                    account: agent_registrar.clone(),
                    purpose: KeyPurpose::AgentRegistrar,
                    capability_mask: CAP_REGISTER_AGENT,
                    expires_at: None,
                    revoked_at: None,
                    created_at: now,
                },
            );
            AuthorizedKeyIdByAccount::<T>::insert((identity_id, agent_registrar.clone()), key_id);
            AgentRegistrarByIdentityId::<T>::insert(identity_id, agent_registrar);
            Ok(())
        }
        /// Derive the uniqueness locator for a transport binding.
        fn transport_locator(
            identity_id: IdentityId,
            transport: TransportKind,
            account: &BoundedVec<u8, T::MaxTransportAccountLen>,
        ) -> Hash256 {
            BlakeTwo256::hash_of(&(b"vibly/transport-locator", identity_id, transport, account))
        }
        /// Bump the identity nonce and update timestamp after a mutation.
        fn bump_identity(identity: &mut RootIdentityOf<T>) {
            identity.nonce = identity.nonce.saturating_add(1);
            identity.updated_at = Self::now();
        }
        /// Enforce lifecycle restrictions before mutating identity-owned state.
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
        /// Enforce owner, recovery, or delegated capability authorization.
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
                AccessScope::AgentRegistrar => CAP_REGISTER_AGENT,
            };
            ensure!(
                record.capability_mask & (required | CAP_ADMIN) != 0,
                Error::<T>::Unauthorized
            );
            Ok(())
        }
        /// Ensure a delegated key has not been revoked or expired.
        fn ensure_record_active(record: &AuthorizedKeyRecordOf<T>) -> Result<(), DispatchError> {
            ensure!(record.revoked_at.is_none(), Error::<T>::KeyRevoked);
            if let Some(expires_at) = record.expires_at {
                ensure!(Self::now() < expires_at, Error::<T>::KeyExpired);
            }
            Ok(())
        }
        /// Shared pointer mutation path that applies authorization and identity nonce updates.
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

    /// Identity authorization implementation consumed by other pallets.
    ///
    /// Payment checks require an active identity and either the owner or a delegated key with
    /// payment capability. Recovery accounts are not allowed to manage or claim payments.
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
        fn ensure_can_register_agent(
            identity_id: &IdentityId,
            who: &T::AccountId,
        ) -> DispatchResult {
            let identity = Identities::<T>::get(identity_id).ok_or(Error::<T>::IdentityNotFound)?;
            ensure!(
                identity.status == IdentityStatus::Active,
                Error::<T>::InvalidState
            );
            Self::ensure_actor(&identity, who, AccessScope::AgentRegistrar)
        }
    }
}
