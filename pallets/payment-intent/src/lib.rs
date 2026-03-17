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
    use frame::traits::tokens::fungible::hold::Mutate as HoldMutate;
    use frame::{
        prelude::*,
        traits::{
            tokens::{fungible::Mutate, Fortitude, Precision, Preservation, Restriction},
            Time,
        },
    };
    use vibly_primitives_common::{Amount, AssetId, ContentRef};
    use vibly_primitives_identity::{IdentityAccess, IdentityId};
    use vibly_primitives_payment::{
        PaymentAction, PaymentIntent, PaymentIntentId, PaymentIntentStatus, SettlementMode,
    };

    type PaymentActionOf<T> = PaymentAction<
        <T as Config>::MaxNamespaceLen,
        <T as Config>::MaxCidLen,
        <T as Config>::MaxUriLen,
    >;
    type ContentRefOf<T> = ContentRef<<T as Config>::MaxCidLen, <T as Config>::MaxUriLen>;
    type PaymentIntentOf<T> = PaymentIntent<
        <T as Config>::MaxNamespaceLen,
        <T as Config>::MaxCidLen,
        <T as Config>::MaxUriLen,
    >;

    #[pallet::composite_enum]
    pub enum HoldReason {
        PaymentIntent,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type WeightInfo: crate::weights::WeightInfo;
        type TimeProvider: Time<Moment = u64>;
        type IdentityProvider: IdentityAccess<Self::AccountId>;
        type Currency: Mutate<Self::AccountId, Balance = Amount>
            + HoldMutate<Self::AccountId, Balance = Amount, Reason = Self::RuntimeHoldReason>;
        type RuntimeHoldReason: From<HoldReason>;
        #[pallet::constant]
        type MaxNamespaceLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        #[pallet::constant]
        type MaxCidLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        #[pallet::constant]
        type MaxUriLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type PaymentIntents<T: Config> =
        StorageMap<_, Blake2_128Concat, PaymentIntentId, PaymentIntentOf<T>>;
    #[pallet::storage]
    pub type IntentFundingAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, PaymentIntentId, T::AccountId>;
    #[pallet::storage]
    pub type PaymentIntentsByPayer<T: Config> =
        StorageMap<_, Blake2_128Concat, (IdentityId, PaymentIntentId), (), OptionQuery>;
    #[pallet::storage]
    pub type PaymentIntentsByPayee<T: Config> =
        StorageMap<_, Blake2_128Concat, (IdentityId, PaymentIntentId), (), OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        PaymentIntentCreated {
            intent_id: PaymentIntentId,
            payer: IdentityId,
            payee: IdentityId,
            asset_id: AssetId,
            amount: Amount,
            action: PaymentActionOf<T>,
        },
        PaymentIntentFunded {
            intent_id: PaymentIntentId,
            settlement_mode: SettlementMode,
        },
        PaymentIntentClaimed {
            intent_id: PaymentIntentId,
        },
        PaymentIntentRefunded {
            intent_id: PaymentIntentId,
        },
        PaymentIntentCancelled {
            intent_id: PaymentIntentId,
        },
        PaymentIntentExpired {
            intent_id: PaymentIntentId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        IntentAlreadyExists,
        IntentNotFound,
        InvalidState,
        Unauthorized,
        InvalidAmount,
        InvalidAsset,
        InvalidAction,
        InvalidSettlementMode,
        FundingUnavailable,
        InsufficientBalance,
        AlreadyExpired,
        NotYetExpired,
        ClaimNotAllowed,
        RefundNotAllowed,
        CancelNotAllowed,
        ExpireNotAllowed,
        EvidenceInvalid,
        NonceInvalid,
        Overflow,
        InvalidInput,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_payment_intent())]
        pub fn create_payment_intent(
            origin: OriginFor<T>,
            intent_id: PaymentIntentId,
            payer: IdentityId,
            payee: IdentityId,
            asset_id: AssetId,
            amount: Amount,
            action: PaymentActionOf<T>,
            memo_ref: Option<ContentRefOf<T>>,
            settlement_mode: SettlementMode,
            expires_at: Option<u64>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !PaymentIntents::<T>::contains_key(intent_id),
                Error::<T>::IntentAlreadyExists
            );
            ensure!(amount > 0, Error::<T>::InvalidAmount);
            ensure!(asset_id == 0, Error::<T>::InvalidAsset);
            ensure!(!action.namespace.is_empty(), Error::<T>::InvalidAction);
            ensure!(
                T::IdentityProvider::identity_exists(&payer),
                Error::<T>::InvalidInput
            );
            ensure!(
                T::IdentityProvider::identity_exists(&payee),
                Error::<T>::InvalidInput
            );
            T::IdentityProvider::ensure_can_manage_payment(&payer, &who)?;
            let now = Self::now();
            let payer_nonce: u64 = 0;
            let payment_intent = PaymentIntent {
                intent_id,
                payer,
                payee,
                asset_id,
                amount,
                action: action.clone(),
                memo_ref,
                settlement_mode,
                expires_at,
                payer_nonce,
                status: PaymentIntentStatus::Requested,
                created_at: now,
                updated_at: now,
            };
            PaymentIntents::<T>::insert(intent_id, payment_intent);
            PaymentIntentsByPayer::<T>::insert((payer, intent_id), ());
            PaymentIntentsByPayee::<T>::insert((payee, intent_id), ());
            Self::deposit_event(Event::PaymentIntentCreated {
                intent_id,
                payer,
                payee,
                asset_id,
                amount,
                action,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::fund_payment_intent())]
        pub fn fund_payment_intent(
            origin: OriginFor<T>,
            intent_id: PaymentIntentId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            PaymentIntents::<T>::try_mutate(intent_id, |maybe_intent| -> DispatchResult {
                let intent = maybe_intent.as_mut().ok_or(Error::<T>::IntentNotFound)?;
                ensure!(
                    intent.status == PaymentIntentStatus::Requested,
                    Error::<T>::InvalidState
                );
                ensure!(!Self::is_expired(intent), Error::<T>::AlreadyExpired);
                T::IdentityProvider::ensure_can_manage_payment(&intent.payer, &who)?;
                let payee_owner = T::IdentityProvider::owner_account(&intent.payee)
                    .ok_or(Error::<T>::InvalidInput)?;
                match intent.settlement_mode {
                    SettlementMode::Direct => {
                        T::Currency::transfer(
                            &who,
                            &payee_owner,
                            intent.amount,
                            Preservation::Preserve,
                        )?;
                        intent.status = PaymentIntentStatus::Claimed;
                        intent.updated_at = Self::now();
                        Self::deposit_event(Event::PaymentIntentFunded {
                            intent_id,
                            settlement_mode: SettlementMode::Direct,
                        });
                        Self::deposit_event(Event::PaymentIntentClaimed { intent_id });
                    }
                    SettlementMode::Hold => {
                        T::Currency::hold(&HoldReason::PaymentIntent.into(), &who, intent.amount)?;
                        IntentFundingAccounts::<T>::insert(intent_id, who);
                        intent.status = PaymentIntentStatus::Funded;
                        intent.updated_at = Self::now();
                        Self::deposit_event(Event::PaymentIntentFunded {
                            intent_id,
                            settlement_mode: SettlementMode::Hold,
                        });
                    }
                }
                Ok(())
            })
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::claim_payment_intent())]
        pub fn claim_payment_intent(
            origin: OriginFor<T>,
            intent_id: PaymentIntentId,
            _evidence_ref: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            PaymentIntents::<T>::try_mutate(intent_id, |maybe_intent| -> DispatchResult {
                let intent = maybe_intent.as_mut().ok_or(Error::<T>::IntentNotFound)?;
                ensure!(
                    intent.status == PaymentIntentStatus::Funded,
                    Error::<T>::InvalidState
                );
                T::IdentityProvider::ensure_can_claim_payment(&intent.payee, &who)?;
                let source = IntentFundingAccounts::<T>::get(intent_id)
                    .ok_or(Error::<T>::FundingUnavailable)?;
                let payee_owner = T::IdentityProvider::owner_account(&intent.payee)
                    .ok_or(Error::<T>::InvalidInput)?;
                T::Currency::transfer_on_hold(
                    &HoldReason::PaymentIntent.into(),
                    &source,
                    &payee_owner,
                    intent.amount,
                    Precision::Exact,
                    Restriction::Free,
                    Fortitude::Polite,
                )?;
                IntentFundingAccounts::<T>::remove(intent_id);
                intent.status = PaymentIntentStatus::Claimed;
                intent.updated_at = Self::now();
                Self::deposit_event(Event::PaymentIntentClaimed { intent_id });
                Ok(())
            })
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::refund_payment_intent())]
        pub fn refund_payment_intent(
            origin: OriginFor<T>,
            intent_id: PaymentIntentId,
            _evidence_ref: Option<ContentRefOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            PaymentIntents::<T>::try_mutate(intent_id, |maybe_intent| -> DispatchResult {
                let intent = maybe_intent.as_mut().ok_or(Error::<T>::IntentNotFound)?;
                ensure!(
                    intent.status == PaymentIntentStatus::Funded,
                    Error::<T>::InvalidState
                );
                T::IdentityProvider::ensure_can_manage_payment(&intent.payer, &who)?;
                let source = IntentFundingAccounts::<T>::get(intent_id)
                    .ok_or(Error::<T>::FundingUnavailable)?;
                let _ = T::Currency::release(
                    &HoldReason::PaymentIntent.into(),
                    &source,
                    intent.amount,
                    Precision::Exact,
                )?;
                IntentFundingAccounts::<T>::remove(intent_id);
                intent.status = PaymentIntentStatus::Refunded;
                intent.updated_at = Self::now();
                Self::deposit_event(Event::PaymentIntentRefunded { intent_id });
                Ok(())
            })
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::cancel_payment_intent())]
        pub fn cancel_payment_intent(
            origin: OriginFor<T>,
            intent_id: PaymentIntentId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            PaymentIntents::<T>::try_mutate(intent_id, |maybe_intent| -> DispatchResult {
                let intent = maybe_intent.as_mut().ok_or(Error::<T>::IntentNotFound)?;
                ensure!(
                    intent.status == PaymentIntentStatus::Requested,
                    Error::<T>::CancelNotAllowed
                );
                T::IdentityProvider::ensure_can_manage_payment(&intent.payer, &who)?;
                intent.status = PaymentIntentStatus::Cancelled;
                intent.updated_at = Self::now();
                Self::deposit_event(Event::PaymentIntentCancelled { intent_id });
                Ok(())
            })
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::expire_payment_intent())]
        pub fn expire_payment_intent(
            origin: OriginFor<T>,
            intent_id: PaymentIntentId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            PaymentIntents::<T>::try_mutate(intent_id, |maybe_intent| -> DispatchResult {
                let intent = maybe_intent.as_mut().ok_or(Error::<T>::IntentNotFound)?;
                ensure!(
                    intent.status == PaymentIntentStatus::Requested,
                    Error::<T>::ExpireNotAllowed
                );
                ensure!(Self::is_expired(intent), Error::<T>::NotYetExpired);
                intent.status = PaymentIntentStatus::Expired;
                intent.updated_at = Self::now();
                Self::deposit_event(Event::PaymentIntentExpired { intent_id });
                Ok(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        fn now() -> u64 {
            T::TimeProvider::now()
        }
        fn is_expired(intent: &PaymentIntentOf<T>) -> bool {
            intent
                .expires_at
                .map(|expires_at| Self::now() >= expires_at)
                .unwrap_or(false)
        }
    }
}
