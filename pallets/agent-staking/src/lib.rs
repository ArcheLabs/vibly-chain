#![cfg_attr(not(feature = "std"), no_std)]
//! Agent staking pallet.
//!
//! Stakes are held against registered Vibly agents. Authorization is delegated to
//! `pallet-identity-core` through `IdentityAccess`, and agent existence is read
//! from `pallet-onboarding-distribution`.

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
            tokens::{
                fungible::hold::Mutate as HoldMutate,
                Precision,
            },
            EnsureOrigin,
        },
    };
    use vibly_primitives_common::{Amount, ContentRef, Hash256};
    use vibly_primitives_identity::{IdentityAccess, IdentityId};

    type ContentRefOf<T> = ContentRef<<T as Config>::MaxReasonCidLen, <T as Config>::MaxReasonUriLen>;
    type BlockNumberFor<T> = frame_system::pallet_prelude::BlockNumberFor<T>;

    #[pallet::composite_enum]
    /// Hold reason used for native balances reserved as agent stake.
    pub enum HoldReason {
        /// Funds held while an agent has active or unbonding stake.
        AgentStake,
    }

    #[derive(Clone, Copy, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum AgentStakeStatus {
        Active,
        Unbonding,
        Released,
    }

    #[derive(Clone, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(AccountId, BlockNumber))]
    pub struct AgentStakeLedger<AccountId, BlockNumber, MaxCidLen: Get<u32>, MaxUriLen: Get<u32>> {
        pub identity_id: IdentityId,
        pub agent_id: Hash256,
        pub active_amount: Amount,
        pub unbonding_amount: Amount,
        pub status: AgentStakeStatus,
        pub unlock_at_block: Option<BlockNumber>,
        pub release_blocked: bool,
        pub release_block_reason: Option<ContentRef<MaxCidLen, MaxUriLen>>,
        pub last_funding_account: Option<AccountId>,
        pub updated_at_block: BlockNumber,
    }

    #[derive(Clone, Default, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(BlockNumber))]
    pub struct AgentStakeHold<BlockNumber> {
        pub active_amount: Amount,
        pub unbonding_amount: Amount,
        pub unlock_at_block: Option<BlockNumber>,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_identity_core::Config + pallet_onboarding_distribution::Config {
        type WeightInfo: WeightInfo;
        type IdentityProvider: IdentityAccess<Self::AccountId>;
        type Currency: HoldMutate<Self::AccountId, Balance = Amount, Reason = Self::RuntimeHoldReason>;
        type RuntimeHoldReason: From<HoldReason>;
        type ReleaseBlockOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        #[pallet::constant]
        type UnbondingPeriod: Get<BlockNumberFor<Self>>;
        #[pallet::constant]
        type MaxReasonCidLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        #[pallet::constant]
        type MaxReasonUriLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type AgentStakeLedgers<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (IdentityId, Hash256),
        AgentStakeLedger<T::AccountId, BlockNumberFor<T>, T::MaxReasonCidLen, T::MaxReasonUriLen>,
    >;

    #[pallet::storage]
    pub type AgentStakeHolds<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (IdentityId, Hash256, T::AccountId),
        AgentStakeHold<BlockNumberFor<T>>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AgentStakeBonded {
            identity_id: IdentityId,
            agent_id: Hash256,
            funding_account: T::AccountId,
            amount: Amount,
            active_amount: Amount,
        },
        AgentStakeUnbondRequested {
            identity_id: IdentityId,
            agent_id: Hash256,
            funding_account: T::AccountId,
            amount: Amount,
            unlock_at_block: BlockNumberFor<T>,
        },
        AgentStakeUnbondCancelled {
            identity_id: IdentityId,
            agent_id: Hash256,
            funding_account: T::AccountId,
            amount: Amount,
        },
        AgentStakeReleaseBlocked {
            identity_id: IdentityId,
            agent_id: Hash256,
            reason_ref: Option<ContentRefOf<T>>,
        },
        AgentStakeReleaseCleared {
            identity_id: IdentityId,
            agent_id: Hash256,
        },
        AgentStakeReleased {
            identity_id: IdentityId,
            agent_id: Hash256,
            funding_account: T::AccountId,
            amount: Amount,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        AgentNotRegistered,
        InvalidAmount,
        InsufficientActiveStake,
        NoUnbondingStake,
        UnbondingNotReady,
        ReleaseBlocked,
        Overflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::bond_agent())]
        pub fn bond_agent(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_id: Hash256,
            amount: Amount,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount > 0, Error::<T>::InvalidAmount);
            Self::ensure_agent_registered(identity_id, agent_id)?;
            <T as Config>::IdentityProvider::ensure_can_register_agent(&identity_id, &who)?;
            <T as Config>::Currency::hold(&HoldReason::AgentStake.into(), &who, amount)?;

            let key = (identity_id, agent_id, who.clone());
            AgentStakeHolds::<T>::try_mutate_exists(key, |maybe_hold| -> DispatchResult {
                let mut hold = maybe_hold.take().unwrap_or_default();
                hold.active_amount = hold.active_amount.checked_add(amount).ok_or(Error::<T>::Overflow)?;
                *maybe_hold = Some(hold);
                Ok(())
            })?;
            let ledger = Self::recompute_ledger(identity_id, agent_id, Some(who.clone()), None, false)?;
            Self::deposit_event(Event::AgentStakeBonded {
                identity_id,
                agent_id,
                funding_account: who,
                amount,
                active_amount: ledger.active_amount,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::request_unbond())]
        pub fn request_unbond(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_id: Hash256,
            amount: Amount,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount > 0, Error::<T>::InvalidAmount);
            Self::ensure_agent_registered(identity_id, agent_id)?;
            <T as Config>::IdentityProvider::ensure_can_register_agent(&identity_id, &who)?;

            let unlock_at = frame_system::Pallet::<T>::block_number()
                .checked_add(&T::UnbondingPeriod::get())
                .ok_or(Error::<T>::Overflow)?;
            let key = (identity_id, agent_id, who.clone());
            AgentStakeHolds::<T>::try_mutate_exists(key, |maybe_hold| -> DispatchResult {
                let mut hold = maybe_hold.take().ok_or(Error::<T>::InsufficientActiveStake)?;
                ensure!(hold.active_amount >= amount, Error::<T>::InsufficientActiveStake);
                hold.active_amount = hold.active_amount.saturating_sub(amount);
                hold.unbonding_amount = hold.unbonding_amount.checked_add(amount).ok_or(Error::<T>::Overflow)?;
                hold.unlock_at_block = Some(unlock_at);
                *maybe_hold = Some(hold);
                Ok(())
            })?;
            Self::recompute_ledger(identity_id, agent_id, Some(who.clone()), Some(unlock_at), false)?;
            Self::deposit_event(Event::AgentStakeUnbondRequested {
                identity_id,
                agent_id,
                funding_account: who,
                amount,
                unlock_at_block: unlock_at,
            });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::cancel_unbond())]
        pub fn cancel_unbond(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_id: Hash256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_agent_registered(identity_id, agent_id)?;
            <T as Config>::IdentityProvider::ensure_can_register_agent(&identity_id, &who)?;
            let key = (identity_id, agent_id, who.clone());
            let mut cancelled = 0;
            AgentStakeHolds::<T>::try_mutate_exists(key, |maybe_hold| -> DispatchResult {
                let mut hold = maybe_hold.take().ok_or(Error::<T>::NoUnbondingStake)?;
                ensure!(hold.unbonding_amount > 0, Error::<T>::NoUnbondingStake);
                cancelled = hold.unbonding_amount;
                hold.active_amount = hold.active_amount.checked_add(hold.unbonding_amount).ok_or(Error::<T>::Overflow)?;
                hold.unbonding_amount = 0;
                hold.unlock_at_block = None;
                *maybe_hold = Some(hold);
                Ok(())
            })?;
            Self::recompute_ledger(identity_id, agent_id, Some(who.clone()), None, false)?;
            Self::deposit_event(Event::AgentStakeUnbondCancelled {
                identity_id,
                agent_id,
                funding_account: who,
                amount: cancelled,
            });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::block_release())]
        pub fn block_release(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_id: Hash256,
            reason_ref: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            T::ReleaseBlockOrigin::ensure_origin(origin)?;
            Self::ensure_agent_registered(identity_id, agent_id)?;
            let mut ledger = AgentStakeLedgers::<T>::get((identity_id, agent_id)).ok_or(Error::<T>::NoUnbondingStake)?;
            ledger.release_blocked = true;
            ledger.release_block_reason = reason_ref.clone();
            ledger.updated_at_block = frame_system::Pallet::<T>::block_number();
            AgentStakeLedgers::<T>::insert((identity_id, agent_id), ledger);
            Self::deposit_event(Event::AgentStakeReleaseBlocked { identity_id, agent_id, reason_ref });
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::clear_release_block())]
        pub fn clear_release_block(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_id: Hash256,
        ) -> DispatchResult {
            T::ReleaseBlockOrigin::ensure_origin(origin)?;
            let mut ledger = AgentStakeLedgers::<T>::get((identity_id, agent_id)).ok_or(Error::<T>::NoUnbondingStake)?;
            ledger.release_blocked = false;
            ledger.release_block_reason = None;
            ledger.updated_at_block = frame_system::Pallet::<T>::block_number();
            AgentStakeLedgers::<T>::insert((identity_id, agent_id), ledger);
            Self::deposit_event(Event::AgentStakeReleaseCleared { identity_id, agent_id });
            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::release_unbond())]
        pub fn release_unbond(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_id: Hash256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let ledger = AgentStakeLedgers::<T>::get((identity_id, agent_id)).ok_or(Error::<T>::NoUnbondingStake)?;
            ensure!(!ledger.release_blocked, Error::<T>::ReleaseBlocked);

            let key = (identity_id, agent_id, who.clone());
            let mut released = 0;
            AgentStakeHolds::<T>::try_mutate_exists(key, |maybe_hold| -> DispatchResult {
                let hold = maybe_hold.as_mut().ok_or(Error::<T>::NoUnbondingStake)?;
                ensure!(hold.unbonding_amount > 0, Error::<T>::NoUnbondingStake);
                let unlock_at = hold.unlock_at_block.ok_or(Error::<T>::NoUnbondingStake)?;
                ensure!(frame_system::Pallet::<T>::block_number() >= unlock_at, Error::<T>::UnbondingNotReady);
                released = hold.unbonding_amount;
                hold.unbonding_amount = 0;
                hold.unlock_at_block = None;
                if hold.active_amount == 0 {
                    *maybe_hold = None;
                }
                Ok(())
            })?;
            let _ = <T as Config>::Currency::release(&HoldReason::AgentStake.into(), &who, released, Precision::Exact)?;
            Self::recompute_ledger(identity_id, agent_id, Some(who.clone()), None, false)?;
            Self::deposit_event(Event::AgentStakeReleased {
                identity_id,
                agent_id,
                funding_account: who,
                amount: released,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn ensure_agent_registered(identity_id: IdentityId, agent_id: Hash256) -> DispatchResult {
            ensure!(
                pallet_onboarding_distribution::AgentRegistrations::<T>::contains_key((identity_id, agent_id)),
                Error::<T>::AgentNotRegistered
            );
            Ok(())
        }

        fn recompute_ledger(
            identity_id: IdentityId,
            agent_id: Hash256,
            last_funding_account: Option<T::AccountId>,
            unlock_hint: Option<BlockNumberFor<T>>,
            preserve_block: bool,
        ) -> Result<AgentStakeLedger<T::AccountId, BlockNumberFor<T>, T::MaxReasonCidLen, T::MaxReasonUriLen>, DispatchError> {
            let mut active_amount: Amount = 0;
            let mut unbonding_amount: Amount = 0;
            let mut unlock_at_block: Option<BlockNumberFor<T>> = unlock_hint;
            for ((hold_identity_id, hold_agent_id, _), hold) in AgentStakeHolds::<T>::iter() {
                if hold_identity_id != identity_id || hold_agent_id != agent_id {
                    continue;
                }
                active_amount = active_amount.checked_add(hold.active_amount).ok_or(Error::<T>::Overflow)?;
                unbonding_amount = unbonding_amount.checked_add(hold.unbonding_amount).ok_or(Error::<T>::Overflow)?;
                if let Some(unlock_at) = hold.unlock_at_block {
                    unlock_at_block = Some(unlock_at_block.map_or(unlock_at, |current| current.max(unlock_at)));
                }
            }
            let existing = AgentStakeLedgers::<T>::get((identity_id, agent_id));
            let status = if active_amount > 0 {
                AgentStakeStatus::Active
            } else if unbonding_amount > 0 {
                AgentStakeStatus::Unbonding
            } else {
                AgentStakeStatus::Released
            };
            let ledger = AgentStakeLedger {
                identity_id,
                agent_id,
                active_amount,
                unbonding_amount,
                status,
                unlock_at_block,
                release_blocked: if preserve_block { existing.as_ref().map(|l| l.release_blocked).unwrap_or(false) } else { existing.as_ref().map(|l| l.release_blocked).unwrap_or(false) },
                release_block_reason: existing.and_then(|l| l.release_block_reason),
                last_funding_account,
                updated_at_block: frame_system::Pallet::<T>::block_number(),
            };
            AgentStakeLedgers::<T>::insert((identity_id, agent_id), ledger.clone());
            Ok(ledger)
        }
    }
}
