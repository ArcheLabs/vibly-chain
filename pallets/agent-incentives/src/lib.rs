#![cfg_attr(not(feature = "std"), no_std)]
//! Agent incentive pallet for the incentivized testnet.

extern crate alloc;

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
            tokens::{fungible::Mutate, Preservation},
            EnsureOrigin,
        },
    };
    use alloc::{collections::BTreeSet, vec::Vec};
    use core::cmp::min;
    use pallet_agent_staking::{AgentStakeLedgers, AgentStakeStatus};
    use vibly_primitives_common::{Amount, Hash256};
    use vibly_primitives_identity::{IdentityAccess, IdentityId};

    type ExternalIdOf<T> = BoundedVec<u8, <T as Config>::MaxExternalIdLen>;
    type ParticipantsOf<T> = BoundedVec<AgentRef, <T as Config>::MaxSettlementParticipants>;
    type BlockNumberFor<T> = frame_system::pallet_prelude::BlockNumberFor<T>;

    const BPS_DENOMINATOR: Amount = 10_000;
    const DAYS_PER_YEAR: Amount = 365;
    const SECONDS_PER_DAY: Amount = 86_400;

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
    pub enum TaskDifficulty {
        #[default]
        Easy,
        Normal,
        Hard,
        Critical,
    }

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
    pub enum RoundRole {
        #[default]
        Observer,
        Reviewer,
    }

    #[derive(
        Clone,
        Copy,
        Default,
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
    pub struct AgentRef {
        pub identity_id: IdentityId,
        pub agent_id: Hash256,
    }

    #[derive(
        Clone,
        Eq,
        PartialEq,
        Default,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        serde::Serialize,
        serde::Deserialize,
    )]
    pub struct DifficultyRewardSchedule {
        pub easy: Amount,
        pub normal: Amount,
        pub hard: Amount,
        pub critical: Amount,
    }

    #[derive(
        Clone,
        Eq,
        PartialEq,
        Default,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        serde::Serialize,
        serde::Deserialize,
    )]
    pub struct RewardConfig {
        pub total_reward_pool: Amount,
        pub auto_emission_pool: Amount,
        pub base_staking_pool: Amount,
        pub observer_reviewer_pool: Amount,
        pub task_market_pool: Amount,
        pub reserve_pool: Amount,
        pub emission_start_day: u32,
        pub planned_emission_days: u32,
        pub min_stake: Amount,
        pub max_effective_stake: Amount,
        pub max_passive_apy_bps: u32,
        pub round_duration_seconds: u32,
        pub observer_share_bps: u32,
        pub reviewer_share_bps: u32,
        pub task_max_subsidy: Amount,
        pub agent_daily_observer_reviewer_reward_cap: Amount,
        pub agent_daily_task_reward_cap: Amount,
        pub agent_daily_total_protocol_reward_cap: Amount,
        pub difficulty_schedule: DifficultyRewardSchedule,
    }

    #[derive(
        Clone,
        Eq,
        PartialEq,
        Default,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    pub struct DailyEmissionState {
        pub day_index: u32,
        pub base_staking_budget: Amount,
        pub observer_reviewer_budget: Amount,
        pub task_market_budget: Amount,
        pub base_staking_released: Amount,
        pub observer_reviewer_released: Amount,
        pub task_market_released: Amount,
        pub rollover_base_staking: Amount,
        pub rollover_observer_reviewer: Amount,
        pub rollover_task_market: Amount,
        pub base_staking_settled: bool,
        pub observer_rounds_settled: u32,
        pub reviewer_rounds_settled: u32,
        pub task_rewards_settled: u32,
    }

    #[derive(
        Clone,
        Eq,
        PartialEq,
        Default,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    pub struct AgentRewardLedger<BlockNumber> {
        pub claimable_total: Amount,
        pub claimed_total: Amount,
        pub claimable_base: Amount,
        pub claimable_observer: Amount,
        pub claimable_reviewer: Amount,
        pub claimable_task: Amount,
        pub claimed_base: Amount,
        pub claimed_observer: Amount,
        pub claimed_reviewer: Amount,
        pub claimed_task: Amount,
        pub updated_at_block: BlockNumber,
    }

    #[derive(
        Clone,
        Eq,
        PartialEq,
        Default,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    #[scale_info(skip_type_params(ExternalId))]
    pub struct RoundSettlement<ExternalId> {
        pub round_id: ExternalId,
        pub day_index: u32,
        pub role: RoundRole,
        pub participant_count: u32,
        pub total_effective_stake: Amount,
        pub released: Amount,
    }

    #[derive(
        Clone,
        Eq,
        PartialEq,
        Default,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    #[scale_info(skip_type_params(ExternalId))]
    pub struct TaskRewardSettlement<ExternalId> {
        pub task_id: ExternalId,
        pub day_index: u32,
        pub identity_id: IdentityId,
        pub agent_id: Hash256,
        pub difficulty: TaskDifficulty,
        pub released: Amount,
    }

    #[derive(
        Clone,
        Eq,
        PartialEq,
        Default,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    pub struct AgentDailyUsage {
        pub observer_reviewer_amount: Amount,
        pub task_amount: Amount,
        pub total_protocol_amount: Amount,
    }

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_identity_core::Config + pallet_agent_staking::Config
    {
        type WeightInfo: WeightInfo;
        type IdentityProvider: IdentityAccess<Self::AccountId>;
        type Currency: Mutate<Self::AccountId, Balance = Amount>;
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        #[pallet::constant]
        type RewardReserveAccount: Get<Self::AccountId>;
        #[pallet::constant]
        type MaxExternalIdLen: Get<u32>;
        #[pallet::constant]
        type MaxSettlementParticipants: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type RewardConfigs<T: Config> = StorageValue<_, RewardConfig, OptionQuery>;

    #[pallet::storage]
    pub type RewardSettlementPublisher<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::storage]
    pub type DailyEmissionStates<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, DailyEmissionState, OptionQuery>;

    #[pallet::storage]
    pub type AgentRewardLedgers<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (IdentityId, Hash256),
        AgentRewardLedger<BlockNumberFor<T>>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type RoundSettlements<T: Config> =
        StorageMap<_, Blake2_128Concat, ExternalIdOf<T>, RoundSettlement<ExternalIdOf<T>>, OptionQuery>;

    #[pallet::storage]
    pub type TaskRewardSettlements<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ExternalIdOf<T>,
        TaskRewardSettlement<ExternalIdOf<T>>,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type AgentDailyUsages<T: Config> =
        StorageMap<_, Blake2_128Concat, (u32, IdentityId, Hash256), AgentDailyUsage, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub reward_config: Option<RewardConfig>,
        pub reward_settlement_publisher: Option<T::AccountId>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                reward_config: None,
                reward_settlement_publisher: None,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            if let Some(config) = &self.reward_config {
                RewardConfigs::<T>::put(config.clone());
            }
            if let Some(publisher) = &self.reward_settlement_publisher {
                RewardSettlementPublisher::<T>::put(publisher.clone());
            }
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RewardConfigUpdated {
            config: RewardConfig,
        },
        RewardSettlementPublisherUpdated {
            publisher: Option<T::AccountId>,
        },
        BaseStakingDaySettled {
            day_index: u32,
            eligible_agents: u32,
            total_effective_stake: Amount,
            released: Amount,
            rollover: Amount,
        },
        ObserverRoundSettled {
            round_id: ExternalIdOf<T>,
            day_index: u32,
            participant_count: u32,
            total_effective_stake: Amount,
            released: Amount,
            rollover: Amount,
        },
        ReviewerRoundSettled {
            round_id: ExternalIdOf<T>,
            day_index: u32,
            participant_count: u32,
            total_effective_stake: Amount,
            released: Amount,
            rollover: Amount,
        },
        TaskRewardSettled {
            task_id: ExternalIdOf<T>,
            day_index: u32,
            identity_id: IdentityId,
            agent_id: Hash256,
            difficulty: TaskDifficulty,
            released: Amount,
            remaining_budget: Amount,
        },
        AgentRewardClaimed {
            identity_id: IdentityId,
            agent_id: Hash256,
            owner_account: T::AccountId,
            amount: Amount,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        RewardConfigNotSet,
        UnauthorizedSettlementPublisher,
        DuplicateAgent,
        AlreadySettled,
        NoEligibleStake,
        NoClaimableReward,
        IdentityOwnerMissing,
        ArithmeticOverflow,
        InvalidRewardConfig,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::set_reward_config())]
        pub fn set_reward_config(origin: OriginFor<T>, config: RewardConfig) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)
                .map_err(|_| Error::<T>::UnauthorizedSettlementPublisher)?;
            Self::validate_reward_config(&config)?;
            RewardConfigs::<T>::put(config.clone());
            Self::deposit_event(Event::RewardConfigUpdated { config });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::set_reward_settlement_publisher())]
        pub fn set_reward_settlement_publisher(
            origin: OriginFor<T>,
            publisher: Option<T::AccountId>,
        ) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)
                .map_err(|_| Error::<T>::UnauthorizedSettlementPublisher)?;
            match publisher.clone() {
                Some(account) => RewardSettlementPublisher::<T>::put(account),
                None => RewardSettlementPublisher::<T>::kill(),
            }
            Self::deposit_event(Event::RewardSettlementPublisherUpdated { publisher });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::settle_base_staking_day())]
        pub fn settle_base_staking_day(
            origin: OriginFor<T>,
            day_index: u32,
            agents: ParticipantsOf<T>,
        ) -> DispatchResult {
            Self::ensure_settlement_publisher(origin)?;
            let config = Self::reward_config()?;
            let mut state = Self::ensure_day_state(day_index, &config)?;
            ensure!(!state.base_staking_settled, Error::<T>::AlreadySettled);

            let weights = Self::eligible_weights(&agents, config.min_stake, config.max_effective_stake)?;
            ensure!(!weights.is_empty(), Error::<T>::NoEligibleStake);
            let total_effective_stake = weights
                .iter()
                .fold(0u128, |acc: Amount, (_, weight)| acc.saturating_add(*weight));
            ensure!(total_effective_stake != 0u128, Error::<T>::NoEligibleStake);

            let remaining_budget = state
                .base_staking_budget
                .saturating_sub(state.base_staking_released);
            let apy_cap_release = total_effective_stake
                .saturating_mul(config.max_passive_apy_bps as Amount)
                / BPS_DENOMINATOR
                / DAYS_PER_YEAR;
            let released = min(remaining_budget, apy_cap_release);

            Self::distribute_rewards(
                &weights,
                released,
                RewardKind::Base,
                None,
            )?;

            state.base_staking_released = state
                .base_staking_released
                .checked_add(released)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            state.rollover_base_staking = state
                .base_staking_budget
                .saturating_sub(state.base_staking_released);
            state.base_staking_settled = true;
            DailyEmissionStates::<T>::insert(day_index, state.clone());

            Self::deposit_event(Event::BaseStakingDaySettled {
                day_index,
                eligible_agents: weights.len() as u32,
                total_effective_stake,
                released,
                rollover: state.rollover_base_staking,
            });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::settle_observer_round())]
        pub fn settle_observer_round(
            origin: OriginFor<T>,
            round_id: ExternalIdOf<T>,
            day_index: u32,
            participants: ParticipantsOf<T>,
        ) -> DispatchResult {
            Self::ensure_settlement_publisher(origin)?;
            ensure!(
                !RoundSettlements::<T>::contains_key(&round_id),
                Error::<T>::AlreadySettled
            );
            let config = Self::reward_config()?;
            let mut state = Self::ensure_day_state(day_index, &config)?;
            let weights = Self::eligible_weights(&participants, config.min_stake, config.max_effective_stake)?;
            ensure!(!weights.is_empty(), Error::<T>::NoEligibleStake);

            let released = Self::settle_round(
                day_index,
                &config,
                &mut state,
                &weights,
                RoundRole::Observer,
            )?;
            let total_effective_stake = weights
                .iter()
                .fold(0u128, |acc: Amount, (_, weight)| acc.saturating_add(*weight));
            RoundSettlements::<T>::insert(
                &round_id,
                RoundSettlement {
                    round_id: round_id.clone(),
                    day_index,
                    role: RoundRole::Observer,
                    participant_count: weights.len() as u32,
                    total_effective_stake,
                    released,
                },
            );
            DailyEmissionStates::<T>::insert(day_index, state.clone());

            Self::deposit_event(Event::ObserverRoundSettled {
                round_id,
                day_index,
                participant_count: weights.len() as u32,
                total_effective_stake,
                released,
                rollover: state.rollover_observer_reviewer,
            });
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::settle_reviewer_round())]
        pub fn settle_reviewer_round(
            origin: OriginFor<T>,
            round_id: ExternalIdOf<T>,
            day_index: u32,
            participants: ParticipantsOf<T>,
        ) -> DispatchResult {
            Self::ensure_settlement_publisher(origin)?;
            ensure!(
                !RoundSettlements::<T>::contains_key(&round_id),
                Error::<T>::AlreadySettled
            );
            let config = Self::reward_config()?;
            let mut state = Self::ensure_day_state(day_index, &config)?;
            let weights = Self::eligible_weights(&participants, config.min_stake, config.max_effective_stake)?;
            ensure!(!weights.is_empty(), Error::<T>::NoEligibleStake);

            let released = Self::settle_round(
                day_index,
                &config,
                &mut state,
                &weights,
                RoundRole::Reviewer,
            )?;
            let total_effective_stake = weights
                .iter()
                .fold(0u128, |acc: Amount, (_, weight)| acc.saturating_add(*weight));
            RoundSettlements::<T>::insert(
                &round_id,
                RoundSettlement {
                    round_id: round_id.clone(),
                    day_index,
                    role: RoundRole::Reviewer,
                    participant_count: weights.len() as u32,
                    total_effective_stake,
                    released,
                },
            );
            DailyEmissionStates::<T>::insert(day_index, state.clone());

            Self::deposit_event(Event::ReviewerRoundSettled {
                round_id,
                day_index,
                participant_count: weights.len() as u32,
                total_effective_stake,
                released,
                rollover: state.rollover_observer_reviewer,
            });
            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::settle_task_reward())]
        pub fn settle_task_reward(
            origin: OriginFor<T>,
            task_id: ExternalIdOf<T>,
            day_index: u32,
            executor: AgentRef,
            difficulty: TaskDifficulty,
        ) -> DispatchResult {
            Self::ensure_settlement_publisher(origin)?;
            ensure!(
                !TaskRewardSettlements::<T>::contains_key(&task_id),
                Error::<T>::AlreadySettled
            );
            let config = Self::reward_config()?;
            let mut state = Self::ensure_day_state(day_index, &config)?;

            let remaining_daily_budget = state
                .task_market_budget
                .saturating_sub(state.task_market_released);
            let scheduled = min(
                Self::difficulty_reward(&config.difficulty_schedule, difficulty),
                config.task_max_subsidy,
            );
            let usage_key = (day_index, executor.identity_id, executor.agent_id);
            let usage = AgentDailyUsages::<T>::get(&usage_key);
            let reward: Amount = scheduled
                .min(remaining_daily_budget)
                .min(config.agent_daily_task_reward_cap.saturating_sub(usage.task_amount))
                .min(config.agent_daily_total_protocol_reward_cap.saturating_sub(usage.total_protocol_amount));

            if reward != 0u128 {
                Self::credit_reward(executor.identity_id, executor.agent_id, reward, RewardKind::Task, Some(day_index))?;
                state.task_market_released = state
                    .task_market_released
                    .checked_add(reward)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
            }
            state.rollover_task_market = state
                .task_market_budget
                .saturating_sub(state.task_market_released);
            state.task_rewards_settled = state.task_rewards_settled.saturating_add(1);
            DailyEmissionStates::<T>::insert(day_index, state.clone());
            TaskRewardSettlements::<T>::insert(
                &task_id,
                TaskRewardSettlement {
                    task_id: task_id.clone(),
                    day_index,
                    identity_id: executor.identity_id,
                    agent_id: executor.agent_id,
                    difficulty,
                    released: reward,
                },
            );

            Self::deposit_event(Event::TaskRewardSettled {
                task_id,
                day_index,
                identity_id: executor.identity_id,
                agent_id: executor.agent_id,
                difficulty,
                released: reward,
                remaining_budget: state
                    .task_market_budget
                    .saturating_sub(state.task_market_released),
            });
            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(<T as Config>::WeightInfo::claim_agent_rewards())]
        pub fn claim_agent_rewards(
            origin: OriginFor<T>,
            identity_id: IdentityId,
            agent_id: Hash256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            <T as Config>::IdentityProvider::ensure_can_claim_payment(&identity_id, &who)?;

            let mut ledger = AgentRewardLedgers::<T>::get((identity_id, agent_id));
            ensure!(ledger.claimable_total > 0, Error::<T>::NoClaimableReward);
            let owner = <T as Config>::IdentityProvider::owner_account(&identity_id)
                .ok_or(Error::<T>::IdentityOwnerMissing)?;
            let amount = ledger.claimable_total;
            <T as Config>::Currency::transfer(
                &T::RewardReserveAccount::get(),
                &owner,
                amount,
                Preservation::Expendable,
            )?;

            ledger.claimable_total = 0;
            ledger.claimed_total = ledger
                .claimed_total
                .checked_add(amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ledger.claimed_base = ledger
                .claimed_base
                .checked_add(ledger.claimable_base)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ledger.claimed_observer = ledger
                .claimed_observer
                .checked_add(ledger.claimable_observer)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ledger.claimed_reviewer = ledger
                .claimed_reviewer
                .checked_add(ledger.claimable_reviewer)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ledger.claimed_task = ledger
                .claimed_task
                .checked_add(ledger.claimable_task)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ledger.claimable_base = 0;
            ledger.claimable_observer = 0;
            ledger.claimable_reviewer = 0;
            ledger.claimable_task = 0;
            ledger.updated_at_block = frame_system::Pallet::<T>::block_number();
            AgentRewardLedgers::<T>::insert((identity_id, agent_id), ledger);

            Self::deposit_event(Event::AgentRewardClaimed {
                identity_id,
                agent_id,
                owner_account: owner,
                amount,
            });
            Ok(())
        }
    }

    #[derive(Clone, Copy)]
    enum RewardKind {
        Base,
        Observer,
        Reviewer,
        Task,
    }

    impl<T: Config> Pallet<T> {
        fn reward_config() -> Result<RewardConfig, DispatchError> {
            RewardConfigs::<T>::get().ok_or(Error::<T>::RewardConfigNotSet.into())
        }

        fn validate_reward_config(config: &RewardConfig) -> DispatchResult {
            ensure!(config.planned_emission_days > 0, Error::<T>::InvalidRewardConfig);
            ensure!(config.round_duration_seconds > 0, Error::<T>::InvalidRewardConfig);
            ensure!(
                config.observer_share_bps.saturating_add(config.reviewer_share_bps) <= BPS_DENOMINATOR as u32,
                Error::<T>::InvalidRewardConfig,
            );
            Ok(())
        }

        fn ensure_settlement_publisher(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                RewardSettlementPublisher::<T>::get().as_ref() == Some(&who),
                Error::<T>::UnauthorizedSettlementPublisher,
            );
            Ok(())
        }

        fn ensure_day_state(day_index: u32, config: &RewardConfig) -> Result<DailyEmissionState, DispatchError> {
            if let Some(existing) = DailyEmissionStates::<T>::get(day_index) {
                return Ok(existing);
            }

            let base_daily = config.base_staking_pool / config.planned_emission_days as Amount;
            let observer_daily = config.observer_reviewer_pool / config.planned_emission_days as Amount;
            let task_daily = config.task_market_pool / config.planned_emission_days as Amount;
            let previous = if day_index == 0 {
                None
            } else {
                Some(Self::ensure_day_state(day_index - 1, config)?)
            };
            let state = DailyEmissionState {
                day_index,
                base_staking_budget: base_daily
                    .checked_add(previous.as_ref().map(|item| item.rollover_base_staking).unwrap_or(0))
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
                observer_reviewer_budget: observer_daily
                    .checked_add(previous.as_ref().map(|item| item.rollover_observer_reviewer).unwrap_or(0))
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
                task_market_budget: task_daily
                    .checked_add(previous.as_ref().map(|item| item.rollover_task_market).unwrap_or(0))
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
                base_staking_released: 0,
                observer_reviewer_released: 0,
                task_market_released: 0,
                rollover_base_staking: 0,
                rollover_observer_reviewer: 0,
                rollover_task_market: 0,
                base_staking_settled: false,
                observer_rounds_settled: 0,
                reviewer_rounds_settled: 0,
                task_rewards_settled: 0,
            };
            DailyEmissionStates::<T>::insert(day_index, state.clone());
            Ok(state)
        }

        fn eligible_weights(
            participants: &ParticipantsOf<T>,
            min_stake: Amount,
            max_effective_stake: Amount,
        ) -> Result<Vec<(AgentRef, Amount)>, DispatchError> {
            let mut unique = BTreeSet::new();
            let mut weights = Vec::with_capacity(participants.len());
            for participant in participants.iter() {
                ensure!(unique.insert(*participant), Error::<T>::DuplicateAgent);
                if let Some(ledger) = AgentStakeLedgers::<T>::get((participant.identity_id, participant.agent_id)) {
                    if ledger.status == AgentStakeStatus::Released {
                        continue;
                    }
                    if ledger.active_amount < min_stake {
                        continue;
                    }
                    let effective: Amount = min(ledger.active_amount, max_effective_stake);
                    if effective != 0u128 {
                        weights.push((*participant, effective));
                    }
                }
            }
            Ok(weights)
        }

        fn settle_round(
            day_index: u32,
            config: &RewardConfig,
            state: &mut DailyEmissionState,
            weights: &[(AgentRef, Amount)],
            role: RoundRole,
        ) -> Result<Amount, DispatchError> {
            let total_effective_stake = weights
                .iter()
                .fold(0u128, |acc: Amount, (_, weight)| acc.saturating_add(*weight));
            ensure!(total_effective_stake > 0, Error::<T>::NoEligibleStake);

            let role_bps = match role {
                RoundRole::Observer => config.observer_share_bps as Amount,
                RoundRole::Reviewer => config.reviewer_share_bps as Amount,
            };
            let planned_round_budget = state
                .observer_reviewer_budget
                .saturating_mul(role_bps)
                .saturating_mul(config.round_duration_seconds as Amount)
                / BPS_DENOMINATOR
                / SECONDS_PER_DAY;
            let remaining_budget = state
                .observer_reviewer_budget
                .saturating_sub(state.observer_reviewer_released);
            let allocatable = min(planned_round_budget, remaining_budget);
            let kind = match role {
                RoundRole::Observer => RewardKind::Observer,
                RoundRole::Reviewer => RewardKind::Reviewer,
            };

            let released = Self::distribute_rewards(weights, allocatable, kind, Some(day_index))?;
            state.observer_reviewer_released = state
                .observer_reviewer_released
                .checked_add(released)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            state.rollover_observer_reviewer = state
                .observer_reviewer_budget
                .saturating_sub(state.observer_reviewer_released);
            match role {
                RoundRole::Observer => {
                    state.observer_rounds_settled = state.observer_rounds_settled.saturating_add(1);
                }
                RoundRole::Reviewer => {
                    state.reviewer_rounds_settled = state.reviewer_rounds_settled.saturating_add(1);
                }
            }
            Ok(released)
        }

        fn distribute_rewards(
            weights: &[(AgentRef, Amount)],
            allocatable: Amount,
            kind: RewardKind,
            day_index: Option<u32>,
        ) -> Result<Amount, DispatchError> {
            if allocatable == 0 || weights.is_empty() {
                return Ok(0);
            }
            let total_weight = weights
                .iter()
                .fold(0u128, |acc: Amount, (_, weight)| acc.saturating_add(*weight));
            ensure!(total_weight > 0, Error::<T>::NoEligibleStake);

            let mut distributed = 0u128;
            for (index, (agent, weight)) in weights.iter().enumerate() {
                let mut reward = if index + 1 == weights.len() {
                    allocatable.saturating_sub(distributed)
                } else {
                    allocatable.saturating_mul(*weight) / total_weight
                };
                if let Some(day) = day_index {
                    reward = Self::apply_daily_caps(day, agent.identity_id, agent.agent_id, reward, kind)?;
                }
                if reward == 0 {
                    continue;
                }
                Self::credit_reward(agent.identity_id, agent.agent_id, reward, kind, day_index)?;
                distributed = distributed
                    .checked_add(reward)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
            }
            Ok(distributed)
        }

        fn apply_daily_caps(
            day_index: u32,
            identity_id: IdentityId,
            agent_id: Hash256,
            reward: Amount,
            kind: RewardKind,
        ) -> Result<Amount, DispatchError> {
            let config = Self::reward_config()?;
            let usage = AgentDailyUsages::<T>::get((day_index, identity_id, agent_id));
            let capped = match kind {
                RewardKind::Observer | RewardKind::Reviewer => reward
                    .min(
                        config
                            .agent_daily_observer_reviewer_reward_cap
                            .saturating_sub(usage.observer_reviewer_amount),
                    )
                    .min(
                        config
                            .agent_daily_total_protocol_reward_cap
                            .saturating_sub(usage.total_protocol_amount),
                    ),
                RewardKind::Task => reward
                    .min(config.agent_daily_task_reward_cap.saturating_sub(usage.task_amount))
                    .min(
                        config
                            .agent_daily_total_protocol_reward_cap
                            .saturating_sub(usage.total_protocol_amount),
                    ),
                RewardKind::Base => reward,
            };
            Ok(capped)
        }

        fn credit_reward(
            identity_id: IdentityId,
            agent_id: Hash256,
            amount: Amount,
            kind: RewardKind,
            day_index: Option<u32>,
        ) -> DispatchResult {
            if amount == 0 {
                return Ok(());
            }

            AgentRewardLedgers::<T>::try_mutate((identity_id, agent_id), |ledger| -> DispatchResult {
                ledger.claimable_total = ledger
                    .claimable_total
                    .checked_add(amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                match kind {
                    RewardKind::Base => {
                        ledger.claimable_base = ledger
                            .claimable_base
                            .checked_add(amount)
                            .ok_or(Error::<T>::ArithmeticOverflow)?;
                    }
                    RewardKind::Observer => {
                        ledger.claimable_observer = ledger
                            .claimable_observer
                            .checked_add(amount)
                            .ok_or(Error::<T>::ArithmeticOverflow)?;
                    }
                    RewardKind::Reviewer => {
                        ledger.claimable_reviewer = ledger
                            .claimable_reviewer
                            .checked_add(amount)
                            .ok_or(Error::<T>::ArithmeticOverflow)?;
                    }
                    RewardKind::Task => {
                        ledger.claimable_task = ledger
                            .claimable_task
                            .checked_add(amount)
                            .ok_or(Error::<T>::ArithmeticOverflow)?;
                    }
                }
                ledger.updated_at_block = frame_system::Pallet::<T>::block_number();
                Ok(())
            })?;

            if let Some(day) = day_index {
                AgentDailyUsages::<T>::try_mutate((day, identity_id, agent_id), |usage| -> DispatchResult {
                    match kind {
                        RewardKind::Observer | RewardKind::Reviewer => {
                            usage.observer_reviewer_amount = usage
                                .observer_reviewer_amount
                                .checked_add(amount)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                            usage.total_protocol_amount = usage
                                .total_protocol_amount
                                .checked_add(amount)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                        }
                        RewardKind::Task => {
                            usage.task_amount = usage
                                .task_amount
                                .checked_add(amount)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                            usage.total_protocol_amount = usage
                                .total_protocol_amount
                                .checked_add(amount)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                        }
                        RewardKind::Base => {}
                    }
                    Ok(())
                })?;
            }
            Ok(())
        }

        fn difficulty_reward(schedule: &DifficultyRewardSchedule, difficulty: TaskDifficulty) -> Amount {
            match difficulty {
                TaskDifficulty::Easy => schedule.easy,
                TaskDifficulty::Normal => schedule.normal,
                TaskDifficulty::Hard => schedule.hard,
                TaskDifficulty::Critical => schedule.critical,
            }
        }
    }
}
