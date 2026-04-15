#![cfg_attr(not(feature = "std"), no_std)]
//! Payment intent pallet for vibly-chain.
//!
//! This pallet records identity-backed payment intents for the native asset. Intents can settle
//! immediately with a direct transfer or hold payer funds until the payee claims or the payer
//! refunds. Identity authorization is delegated to `pallet-identity-core` through the
//! `IdentityAccess` trait.

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
    /// Hold reason used for native balances reserved by payment intents.
    pub enum HoldReason {
        /// Funds held while a payment intent is in `Funded` state.
        PaymentIntent,
    }

    #[pallet::config]
    /// Runtime configuration for payment intent settlement.
    pub trait Config: frame_system::Config {
        /// Weight provider for dispatchable calls.
        type WeightInfo: crate::weights::WeightInfo;
        /// Timestamp provider returning milliseconds.
        type TimeProvider: Time<Moment = u64>;
        /// Identity lookup and authorization provider.
        type IdentityProvider: IdentityAccess<Self::AccountId>;
        /// Native currency used for direct transfers and holds.
        type Currency: Mutate<Self::AccountId, Balance = Amount>
            + HoldMutate<Self::AccountId, Balance = Amount, Reason = Self::RuntimeHoldReason>;
        /// Runtime-wide hold reason type.
        type RuntimeHoldReason: From<HoldReason>;
        /// Maximum encoded length for payment action namespaces.
        #[pallet::constant]
        type MaxNamespaceLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        /// Maximum encoded length for content CIDs.
        #[pallet::constant]
        type MaxCidLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
        /// Maximum encoded length for content URIs.
        #[pallet::constant]
        type MaxUriLen: Get<u32> + core::fmt::Debug + Clone + Eq + PartialEq + TypeInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    /// Payment intent records keyed by caller-supplied intent id.
    pub type PaymentIntents<T: Config> =
        StorageMap<_, Blake2_128Concat, PaymentIntentId, PaymentIntentOf<T>>;
    #[pallet::storage]
    /// Funding account for held intents, present only while status is `Funded`.
    pub type IntentFundingAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, PaymentIntentId, T::AccountId>;
    #[pallet::storage]
    /// Sparse index of payment intents by payer identity.
    pub type PaymentIntentsByPayer<T: Config> =
        StorageMap<_, Blake2_128Concat, (IdentityId, PaymentIntentId), (), OptionQuery>;
    #[pallet::storage]
    /// Sparse index of payment intents by payee identity.
    pub type PaymentIntentsByPayee<T: Config> =
        StorageMap<_, Blake2_128Concat, (IdentityId, PaymentIntentId), (), OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    /// Payment intent lifecycle events.
    pub enum Event<T: Config> {
        /// A requested payment intent was created.
        PaymentIntentCreated {
            intent_id: PaymentIntentId,
            payer: IdentityId,
            payee: IdentityId,
            asset_id: AssetId,
            amount: Amount,
            action: PaymentActionOf<T>,
        },
        /// Funds moved or were held according to the settlement mode.
        PaymentIntentFunded {
            intent_id: PaymentIntentId,
            settlement_mode: SettlementMode,
        },
        /// Funds were transferred to the payee owner.
        PaymentIntentClaimed { intent_id: PaymentIntentId },
        /// Held funds were released back to the funding account.
        PaymentIntentRefunded { intent_id: PaymentIntentId },
        /// A requested intent was cancelled before funding.
        PaymentIntentCancelled { intent_id: PaymentIntentId },
        /// A requested intent was marked expired.
        PaymentIntentExpired { intent_id: PaymentIntentId },
    }

    #[pallet::error]
    /// Payment intent pallet errors.
    pub enum Error<T> {
        /// The intent id already exists.
        IntentAlreadyExists,
        /// No intent exists for the requested id.
        IntentNotFound,
        /// The intent state does not allow the requested transition.
        InvalidState,
        /// Caller is not authorized by the relevant identity.
        Unauthorized,
        /// Amount must be non-zero.
        InvalidAmount,
        /// Only native asset `0` is currently supported.
        InvalidAsset,
        /// Payment action is invalid.
        InvalidAction,
        /// Settlement mode is invalid or unsupported.
        InvalidSettlementMode,
        /// Held funding account is missing.
        FundingUnavailable,
        /// The funding account cannot cover the requested amount.
        InsufficientBalance,
        /// Intent is already expired for the requested operation.
        AlreadyExpired,
        /// Intent has not reached its expiry time.
        NotYetExpired,
        /// Claim transition is not allowed.
        ClaimNotAllowed,
        /// Refund transition is not allowed.
        RefundNotAllowed,
        /// Cancel transition is not allowed.
        CancelNotAllowed,
        /// Expire transition is not allowed.
        ExpireNotAllowed,
        /// Evidence reference is invalid.
        EvidenceInvalid,
        /// The supplied nonce is invalid.
        NonceInvalid,
        /// Arithmetic overflow.
        Overflow,
        /// Generic invalid input.
        InvalidInput,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a requested payment intent between two identities.
        ///
        /// The signed origin must be authorized to manage payments for the payer identity. The
        /// initial v1 implementation accepts only native asset `asset_id = 0`.
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

        /// Fund an intent according to its settlement mode.
        ///
        /// Direct settlement immediately transfers from the caller to the payee owner and marks the
        /// intent claimed. Hold settlement reserves funds from the caller and records that funding
        /// account until claim or refund.
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

        /// Claim a funded hold-settlement intent.
        ///
        /// The signed origin must be authorized for the payee identity. Held funds move from the
        /// recorded funding account to the current payee owner account.
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

        /// Refund a funded hold-settlement intent to its funding account.
        ///
        /// The signed origin must still be authorized for the payer identity.
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

        /// Cancel a requested intent before funds move.
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

        /// Mark a requested intent as expired after its expiry timestamp.
        ///
        /// Any signed account may submit the expiration transaction because the timestamp and state
        /// transition are deterministic.
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
        /// Return the runtime timestamp in milliseconds.
        fn now() -> u64 {
            T::TimeProvider::now()
        }
        /// Returns whether the current timestamp is at or past the intent expiry.
        fn is_expired(intent: &PaymentIntentOf<T>) -> bool {
            intent
                .expires_at
                .map(|expires_at| Self::now() >= expires_at)
                .unwrap_or(false)
        }
    }
}
