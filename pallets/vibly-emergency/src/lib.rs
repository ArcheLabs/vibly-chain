#![cfg_attr(not(feature = "std"), no_std)]
//! Vibly Emergency pallet.
//!
//! Records the emergency status (`Active`, `Paused`, `Cancelled`) of Vibly-owned scopes such as
//! proposals, reward batches and settlement batches. This pallet intentionally does NOT transfer
//! funds, trigger governance votes, or execute tasks — it is a pure status registry consumed by
//! off-chain coordinators and other pallets via the [`EmergencyInspect`] helper trait.
//!
//! # Guardian model (scheme B)
//! * **pause** – any single Guardian member (`PauseOrigin`)
//! * **cancel / resume** – Guardian collective m/n (`CancelOrigin` / `ResumeOrigin`)

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

#[frame::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame::prelude::*;
    use sp_core::H256;

    // ── Types ────────────────────────────────────────────────────────────────

    /// A `u64` identifier used for proposals, batches, adapters, etc.
    pub type ScopeId = u64;

    /// The scope of an emergency action.
    ///
    /// In P4 Phase 1 only `Proposal` and `Global` are used; additional variants
    /// can be added in future phases without breaking storage.
    #[derive(
        Clone,
        Eq,
        PartialEq,
        Ord,
        PartialOrd,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    pub enum EmergencyScope {
        /// Affects the entire chain (nuclear option).
        Global,
        /// A single Vibly governance proposal.
        Proposal(ScopeId),
        /// A reward distribution batch.
        RewardBatch(ScopeId),
        /// A settlement batch.
        SettlementBatch(ScopeId),
    }

    /// Emergency status of a scope.
    ///
    /// Absent storage entry is equivalent to [`EmergencyStatus::Active`].
    #[derive(
        Clone,
        Copy,
        Default,
        Eq,
        PartialEq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    pub enum EmergencyStatus {
        /// Normal operation.
        #[default]
        Active,
        /// Operations on this scope are temporarily suspended.
        Paused,
        /// Scope is permanently cancelled; cannot be resumed.
        Cancelled,
    }

    /// Stored with a `Paused` transition to record who triggered it and why.
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
    pub struct PauseRecord<AccountId> {
        /// Account that issued the pause.
        pub by: AccountId,
        /// Optional hash of the off-chain reason document.
        pub reason_hash: Option<H256>,
    }

    // ── Pallet ───────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Weight provider.
        type WeightInfo: WeightInfo;
        /// Origin allowed to pause any scope (any single Guardian member).
        type PauseOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
        /// Origin allowed to cancel a scope (Guardian collective m/n).
        type CancelOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Origin allowed to resume a scope (Guardian collective m/n).
        type ResumeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ── Storage ──────────────────────────────────────────────────────────────

    /// Current emergency status per scope.  Absent ⟺ `Active`.
    #[pallet::storage]
    pub type StatusByScope<T: Config> =
        StorageMap<_, Blake2_128Concat, EmergencyScope, EmergencyStatus, ValueQuery>;

    /// The most recent pause record for a scope (replaced on each new pause).
    #[pallet::storage]
    pub type LastPauseRecord<T: Config> =
        StorageMap<_, Blake2_128Concat, EmergencyScope, PauseRecord<T::AccountId>>;

    // ── Events ───────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A scope was paused.
        Paused {
            scope: EmergencyScope,
            by: T::AccountId,
            reason_hash: Option<H256>,
        },
        /// A scope was resumed.
        Resumed {
            scope: EmergencyScope,
            reason_hash: Option<H256>,
        },
        /// A scope was cancelled.
        Cancelled {
            scope: EmergencyScope,
            reason_hash: Option<H256>,
        },
    }

    // ── Errors ───────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Scope is already cancelled; no further transitions allowed.
        AlreadyCancelled,
        /// Scope is already active; cannot resume.
        AlreadyActive,
        /// Scope is not paused; cannot resume.
        NotPaused,
        /// The requested state transition is not allowed.
        InvalidTransition,
    }

    // ── Dispatchables ────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Pause a scope.
        ///
        /// Callable by any Guardian member (`PauseOrigin`).
        /// Allowed transitions: `Active → Paused`, `Paused → Paused` (replaces record).
        /// Rejected when scope is `Cancelled`.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::pause())]
        pub fn pause(
            origin: OriginFor<T>,
            scope: EmergencyScope,
            reason_hash: Option<H256>,
        ) -> DispatchResult {
            let who = T::PauseOrigin::ensure_origin(origin)?;

            let status = StatusByScope::<T>::get(&scope);
            ensure!(
                status != EmergencyStatus::Cancelled,
                Error::<T>::AlreadyCancelled
            );

            StatusByScope::<T>::insert(&scope, EmergencyStatus::Paused);
            LastPauseRecord::<T>::insert(
                &scope,
                PauseRecord {
                    by: who.clone(),
                    reason_hash,
                },
            );

            Self::deposit_event(Event::Paused {
                scope,
                by: who,
                reason_hash,
            });
            Ok(())
        }

        /// Resume a paused scope.
        ///
        /// Callable by Guardian collective m/n (`ResumeOrigin`).
        /// Allowed transition: `Paused → Active`.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::resume())]
        pub fn resume(
            origin: OriginFor<T>,
            scope: EmergencyScope,
            reason_hash: Option<H256>,
        ) -> DispatchResult {
            T::ResumeOrigin::ensure_origin(origin)?;

            let status = StatusByScope::<T>::get(&scope);
            ensure!(
                status != EmergencyStatus::Cancelled,
                Error::<T>::AlreadyCancelled
            );
            ensure!(status == EmergencyStatus::Paused, Error::<T>::NotPaused);

            StatusByScope::<T>::remove(&scope);
            LastPauseRecord::<T>::remove(&scope);

            Self::deposit_event(Event::Resumed { scope, reason_hash });
            Ok(())
        }

        /// Cancel a scope permanently.
        ///
        /// Callable by Guardian collective m/n (`CancelOrigin`).
        /// Allowed transitions: `Active → Cancelled`, `Paused → Cancelled`.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::cancel())]
        pub fn cancel(
            origin: OriginFor<T>,
            scope: EmergencyScope,
            reason_hash: Option<H256>,
        ) -> DispatchResult {
            T::CancelOrigin::ensure_origin(origin)?;

            let status = StatusByScope::<T>::get(&scope);
            ensure!(
                status != EmergencyStatus::Cancelled,
                Error::<T>::AlreadyCancelled
            );

            StatusByScope::<T>::insert(&scope, EmergencyStatus::Cancelled);

            Self::deposit_event(Event::Cancelled { scope, reason_hash });
            Ok(())
        }
    }

    // ── Runtime helpers ──────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Returns `Ok(())` if the scope is `Active`, or the appropriate error otherwise.
        ///
        /// Intended to be called from other pallets before executing scope-guarded logic.
        pub fn ensure_active(scope: &EmergencyScope) -> DispatchResult {
            match StatusByScope::<T>::get(scope) {
                EmergencyStatus::Active => Ok(()),
                EmergencyStatus::Paused => Err(Error::<T>::NotPaused.into()),
                EmergencyStatus::Cancelled => Err(Error::<T>::AlreadyCancelled.into()),
            }
        }

        /// Returns `true` if the scope is currently `Paused`.
        pub fn is_paused(scope: &EmergencyScope) -> bool {
            StatusByScope::<T>::get(scope) == EmergencyStatus::Paused
        }

        /// Returns `true` if the scope has been `Cancelled`.
        pub fn is_cancelled(scope: &EmergencyScope) -> bool {
            StatusByScope::<T>::get(scope) == EmergencyStatus::Cancelled
        }
    }
}
