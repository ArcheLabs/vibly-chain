#![cfg_attr(not(feature = "std"), no_std)]
//! Trusted MVP onboarding and distribution pallet.
//!
//! Coordinator-owned relayers submit EVM onboarding, airdrop issuance, DOT conversion issuance,
//! and EVM-authorized root rotations. This pallet records uniqueness and cap checks on-chain,
//! while DOT payment observation and EVM signature verification remain off-chain Coordinator
//! responsibilities.

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

#[frame::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame::{
        prelude::*,
        traits::{
            tokens::{fungible::Mutate},
            Time,
        },
    };
    use sp_runtime::traits::{BlakeTwo256, Hash as HashT};
    use vibly_primitives_common::{Amount, ContentRef, Hash256};
    use vibly_primitives_identity::{EvmAddress, IdentityAccess, IdentityId};

    type AgentRefOf<T> = ContentRef<<T as Config>::MaxAgentRefCidLen, <T as Config>::MaxAgentRefUriLen>;

    #[derive(Clone, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(AccountId))]
    pub struct AirdropClaim<AccountId> {
        pub identity_id: IdentityId,
        pub root_account: AccountId,
        pub agent_registrar: AccountId,
        pub root_amount: Amount,
        pub registrar_amount: Amount,
        pub relayer: AccountId,
        pub claimed_at: u64,
    }

    #[derive(Clone, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(AccountId))]
    pub struct DotConversion<AccountId> {
        pub identity_id: IdentityId,
        pub payment_id: Hash256,
        pub dot_amount: Amount,
        pub vib_amount: Amount,
        pub recipient: AccountId,
        pub relayer: AccountId,
        pub issued_at: u64,
    }

    #[derive(Clone, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(AccountId, MaxCidLen, MaxUriLen))]
    pub struct AgentRegistration<AccountId, MaxCidLen: Get<u32>, MaxUriLen: Get<u32>> {
        pub identity_id: IdentityId,
        pub agent_id: Hash256,
        pub registrar: AccountId,
        pub agent_ref: ContentRef<MaxCidLen, MaxUriLen>,
        pub registered_at: u64,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> + pallet_identity_core::Config {
        type WeightInfo: WeightInfo;
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        type TimeProvider: Time<Moment = u64>;
        type IdentityProvider: IdentityAccess<Self::AccountId>;
        type Currency: Mutate<Self::AccountId, Balance = Amount>;
        #[pallet::constant]
        type MaxAgentRefCidLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        #[pallet::constant]
        type MaxAgentRefUriLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type AuthorizedRelayers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;
    #[pallet::storage]
    pub type AirdropClaims<T: Config> =
        StorageMap<_, Blake2_128Concat, EvmAddress, AirdropClaim<T::AccountId>>;
    #[pallet::storage]
    pub type DotConversions<T: Config> =
        StorageMap<_, Blake2_128Concat, Hash256, DotConversion<T::AccountId>>;
    #[pallet::storage]
    pub type AgentRegistrations<T: Config> =
        StorageMap<_, Blake2_128Concat, (IdentityId, Hash256), AgentRegistration<T::AccountId, T::MaxAgentRefCidLen, T::MaxAgentRefUriLen>>;
    #[pallet::storage]
    pub type AirdropTotalIssued<T> = StorageValue<_, Amount, ValueQuery>;
    #[pallet::storage]
    pub type ConversionTotalIssued<T> = StorageValue<_, Amount, ValueQuery>;
    #[pallet::storage]
    pub type AirdropTotalCap<T> = StorageValue<_, Amount, ValueQuery>;
    #[pallet::storage]
    pub type ConversionTotalCap<T> = StorageValue<_, Amount, ValueQuery>;
    #[pallet::storage]
    pub type AirdropMaxPerClaim<T> = StorageValue<_, Amount, ValueQuery>;
    #[pallet::storage]
    pub type ConversionMaxPerClaim<T> = StorageValue<_, Amount, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RelayerAuthorizationSet { relayer: T::AccountId, authorized: bool },
        DistributionLimitsSet {
            airdrop_total_cap: Amount,
            conversion_total_cap: Amount,
            airdrop_max_per_claim: Amount,
            conversion_max_per_claim: Amount,
        },
        EvmAirdropIssued {
            evm_address: EvmAddress,
            identity_id: IdentityId,
            root_account: T::AccountId,
            agent_registrar: T::AccountId,
            root_amount: Amount,
            registrar_amount: Amount,
            relayer: T::AccountId,
        },
        EvmRootRotationApplied {
            evm_address: EvmAddress,
            identity_id: IdentityId,
            new_root: T::AccountId,
            relayer: T::AccountId,
        },
        DotConversionIssued {
            identity_id: IdentityId,
            payment_id: Hash256,
            dot_amount: Amount,
            vib_amount: Amount,
            recipient: T::AccountId,
            relayer: T::AccountId,
        },
        AgentRegistered {
            identity_id: IdentityId,
            agent_id: Hash256,
            registrar: T::AccountId,
        },
        AgentRegistrarSet {
            identity_id: IdentityId,
            agent_registrar: T::AccountId,
        },
        AgentRegistrarRevoked { identity_id: IdentityId },
    }

    #[pallet::error]
    pub enum Error<T> {
        UnauthorizedRelayer,
        AirdropAlreadyClaimed,
        DotPaymentAlreadyIssued,
        AirdropCapExceeded,
        ConversionCapExceeded,
        AirdropClaimTooLarge,
        ConversionClaimTooLarge,
        IdentityNotFound,
        AgentAlreadyRegistered,
        Overflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::set_relayer())]
        pub fn set_relayer(
            origin: OriginFor<T>,
            relayer: T::AccountId,
            authorized: bool,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            if authorized {
                AuthorizedRelayers::<T>::insert(&relayer, ());
            } else {
                AuthorizedRelayers::<T>::remove(&relayer);
            }
            Self::deposit_event(Event::RelayerAuthorizationSet { relayer, authorized });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::set_distribution_limits())]
        pub fn set_distribution_limits(
            origin: OriginFor<T>,
            airdrop_total_cap: Amount,
            conversion_total_cap: Amount,
            airdrop_max_per_claim: Amount,
            conversion_max_per_claim: Amount,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            AirdropTotalCap::<T>::put(airdrop_total_cap);
            ConversionTotalCap::<T>::put(conversion_total_cap);
            AirdropMaxPerClaim::<T>::put(airdrop_max_per_claim);
            ConversionMaxPerClaim::<T>::put(conversion_max_per_claim);
            Self::deposit_event(Event::DistributionLimitsSet {
                airdrop_total_cap,
                conversion_total_cap,
                airdrop_max_per_claim,
                conversion_max_per_claim,
            });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::register_evm_airdrop())]
        pub fn register_evm_airdrop(
            origin: OriginFor<T>,
            evm_address: EvmAddress,
            root_account: T::AccountId,
            agent_registrar: T::AccountId,
            root_amount: Amount,
            registrar_amount: Amount,
        ) -> DispatchResult {
            let relayer = ensure_signed(origin)?;
            Self::ensure_relayer(&relayer)?;
            ensure!(!AirdropClaims::<T>::contains_key(evm_address), Error::<T>::AirdropAlreadyClaimed);
            let total_amount = root_amount.checked_add(registrar_amount).ok_or(Error::<T>::Overflow)?;
            Self::ensure_airdrop_limits(total_amount)?;
            let identity_id = pallet_identity_core::Pallet::<T>::register_evm_identity_from_relayer(
                evm_address,
                root_account.clone(),
                agent_registrar.clone(),
            )?;
            if root_amount > 0 {
                T::Currency::mint_into(&root_account, root_amount)?;
            }
            if registrar_amount > 0 {
                T::Currency::mint_into(&agent_registrar, registrar_amount)?;
            }
            AirdropTotalIssued::<T>::mutate(|issued| {
                *issued = issued.saturating_add(total_amount);
            });
            AirdropClaims::<T>::insert(
                evm_address,
                AirdropClaim {
                    identity_id,
                    root_account: root_account.clone(),
                    agent_registrar: agent_registrar.clone(),
                    root_amount,
                    registrar_amount,
                    relayer: relayer.clone(),
                    claimed_at: Self::now(),
                },
            );
            Self::deposit_event(Event::EvmAirdropIssued {
                evm_address,
                identity_id,
                root_account,
                agent_registrar,
                root_amount,
                registrar_amount,
                relayer,
            });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::rotate_root_for_evm())]
        pub fn rotate_root_for_evm(
            origin: OriginFor<T>,
            evm_address: EvmAddress,
            new_root: T::AccountId,
        ) -> DispatchResult {
            let relayer = ensure_signed(origin)?;
            Self::ensure_relayer(&relayer)?;
            let identity_id = pallet_identity_core::IdentityIdByEvmAddress::<T>::get(evm_address)
                .ok_or(Error::<T>::IdentityNotFound)?;
            pallet_identity_core::Pallet::<T>::rotate_owner_for_evm_root(
                evm_address,
                new_root.clone(),
            )?;
            Self::deposit_event(Event::EvmRootRotationApplied {
                evm_address,
                identity_id,
                new_root,
                relayer,
            });
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::issue_dot_conversion())]
        pub fn issue_dot_conversion(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            payment_id: Hash256,
            dot_amount: Amount,
            vib_amount: Amount,
        ) -> DispatchResult {
            let relayer = ensure_signed(origin)?;
            Self::ensure_relayer(&relayer)?;
            ensure!(!DotConversions::<T>::contains_key(payment_id), Error::<T>::DotPaymentAlreadyIssued);
            Self::ensure_conversion_limits(vib_amount)?;
            let recipient = T::IdentityProvider::owner_account(&identity_id).ok_or(Error::<T>::IdentityNotFound)?;
            T::Currency::mint_into(&recipient, vib_amount)?;
            ConversionTotalIssued::<T>::mutate(|issued| {
                *issued = issued.saturating_add(vib_amount);
            });
            DotConversions::<T>::insert(
                payment_id,
                DotConversion {
                    identity_id,
                    payment_id,
                    dot_amount,
                    vib_amount,
                    recipient: recipient.clone(),
                    relayer: relayer.clone(),
                    issued_at: Self::now(),
                },
            );
            Self::deposit_event(Event::DotConversionIssued {
                identity_id,
                payment_id,
                dot_amount,
                vib_amount,
                recipient,
                relayer,
            });
            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::register_agent())]
        pub fn register_agent(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_ref: AgentRefOf<T>,
        ) -> DispatchResult {
            let registrar = ensure_signed(origin)?;
            T::IdentityProvider::ensure_can_register_agent(&identity_id, &registrar)?;
            let agent_id = BlakeTwo256::hash_of(&(b"vibly/agent", identity_id, &registrar, &agent_ref));
            ensure!(
                !AgentRegistrations::<T>::contains_key((identity_id, agent_id)),
                Error::<T>::AgentAlreadyRegistered
            );
            AgentRegistrations::<T>::insert(
                (identity_id, agent_id),
                AgentRegistration {
                    identity_id,
                    agent_id,
                    registrar: registrar.clone(),
                    agent_ref,
                    registered_at: Self::now(),
                },
            );
            Self::deposit_event(Event::AgentRegistered {
                identity_id,
                agent_id,
                registrar,
            });
            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(<T as Config>::WeightInfo::set_agent_registrar())]
        pub fn set_agent_registrar(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_registrar: T::AccountId,
        ) -> DispatchResult {
            let root = ensure_signed(origin)?;
            pallet_identity_core::Pallet::<T>::set_agent_registrar_from_owner(
                identity_id,
                &root,
                agent_registrar.clone(),
            )?;
            Self::deposit_event(Event::AgentRegistrarSet {
                identity_id,
                agent_registrar,
            });
            Ok(())
        }

        #[pallet::call_index(7)]
        #[pallet::weight(<T as Config>::WeightInfo::revoke_agent_registrar())]
        pub fn revoke_agent_registrar(
            origin: OriginFor<T>,
            identity_id: IdentityId,
        ) -> DispatchResult {
            let root = ensure_signed(origin)?;
            pallet_identity_core::Pallet::<T>::revoke_agent_registrar_from_owner(
                identity_id,
                &root,
            )?;
            Self::deposit_event(Event::AgentRegistrarRevoked { identity_id });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn now() -> u64 {
            <T as Config>::TimeProvider::now()
        }
        fn ensure_relayer(relayer: &T::AccountId) -> DispatchResult {
            ensure!(AuthorizedRelayers::<T>::contains_key(relayer), Error::<T>::UnauthorizedRelayer);
            Ok(())
        }
        fn ensure_airdrop_limits(amount: Amount) -> DispatchResult {
            let max = AirdropMaxPerClaim::<T>::get();
            if max > 0 {
                ensure!(amount <= max, Error::<T>::AirdropClaimTooLarge);
            }
            let cap = AirdropTotalCap::<T>::get();
            if cap > 0 {
                let next = AirdropTotalIssued::<T>::get()
                    .checked_add(amount)
                    .ok_or(Error::<T>::Overflow)?;
                ensure!(next <= cap, Error::<T>::AirdropCapExceeded);
            }
            Ok(())
        }
        fn ensure_conversion_limits(amount: Amount) -> DispatchResult {
            let max = ConversionMaxPerClaim::<T>::get();
            if max > 0 {
                ensure!(amount <= max, Error::<T>::ConversionClaimTooLarge);
            }
            let cap = ConversionTotalCap::<T>::get();
            if cap > 0 {
                let next = ConversionTotalIssued::<T>::get()
                    .checked_add(amount)
                    .ok_or(Error::<T>::Overflow)?;
                ensure!(next <= cap, Error::<T>::ConversionCapExceeded);
            }
            Ok(())
        }
    }
}
