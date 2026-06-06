use super::{
    mock::{
        new_test_ext, AgentIncentives, AgentStaking, Balances, IdentityCore,
        OnboardingDistribution, RuntimeEvent, RuntimeOrigin, System, Test, REWARD_RESERVE,
    },
    AgentRef, AgentRewardLedgers, DailyEmissionStates, DifficultyRewardSchedule, Error,
    RewardConfig, RewardCreditKind, TaskDifficulty,
};
use frame::deps::frame_support::{assert_noop, assert_ok};
use vibly_primitives_common::ContentRef;
use vibly_primitives_identity::{IdentityId, KeyPurpose, CAP_MANAGE_PAYMENT};

const BASE_UNIT: u128 = 1_000;

fn reward_config() -> RewardConfig {
    RewardConfig {
        total_reward_pool: 30_000_000 * BASE_UNIT,
        auto_emission_pool: 27_000_000 * BASE_UNIT,
        base_staking_pool: 6_000_000 * BASE_UNIT,
        observer_reviewer_pool: 12_000_000 * BASE_UNIT,
        task_market_pool: 9_000_000 * BASE_UNIT,
        reserve_pool: 3_000_000 * BASE_UNIT,
        emission_start_day: 0,
        planned_emission_days: 365,
        min_stake: 1_000,
        max_effective_stake: 500_000,
        max_passive_apy_bps: 3_000,
        round_duration_seconds: 86_400,
        observer_share_bps: 6_000,
        reviewer_share_bps: 4_000,
        task_max_subsidy: 2_000,
        agent_daily_observer_reviewer_reward_cap: 1_000,
        agent_daily_task_reward_cap: 2_000,
        agent_daily_total_protocol_reward_cap: 3_000,
        difficulty_schedule: DifficultyRewardSchedule {
            easy: 250,
            normal: 500,
            hard: 1_000,
            critical: 1_500,
        },
    }
}

fn register_identity_and_agent(owner: u64, cid: &[u8]) -> (IdentityId, sp_core::H256) {
    assert_ok!(IdentityCore::register_identity(
        RuntimeOrigin::signed(owner),
        None,
        None,
        None,
        None,
        None,
    ));
    let identity_id = System::events()
        .into_iter()
        .rev()
        .find_map(|record| match record.event {
            RuntimeEvent::IdentityCore(pallet_identity_core::Event::IdentityRegistered {
                identity_id,
                owner: event_owner,
            }) if event_owner == owner => Some(identity_id),
            _ => None,
        })
        .expect("identity registered event");

    assert_ok!(OnboardingDistribution::set_agent_registrar(
        RuntimeOrigin::signed(owner),
        identity_id,
        owner,
    ));
    assert_ok!(OnboardingDistribution::register_agent(
        RuntimeOrigin::signed(owner),
        identity_id,
        ContentRef::Cid(cid.to_vec().try_into().expect("cid fits")),
    ));
    let agent_id = System::events()
        .into_iter()
        .rev()
        .find_map(|record| match record.event {
            RuntimeEvent::OnboardingDistribution(
                pallet_onboarding_distribution::Event::AgentRegistered {
                    identity_id: event_identity_id,
                    agent_id,
                    registrar,
                },
            ) if event_identity_id == identity_id && registrar == owner => Some(agent_id),
            _ => None,
        })
        .expect("agent registered event");

    (identity_id, agent_id)
}

fn bond_agent(owner: u64, identity_id: IdentityId, agent_id: sp_core::H256, amount: u128) {
    assert_ok!(AgentStaking::bond_agent(
        RuntimeOrigin::signed(owner),
        identity_id,
        agent_id,
        amount,
    ));
}

#[test]
fn base_staking_settlement_and_claim_flow_work() {
    new_test_ext().execute_with(|| {
        let (identity_id, agent_id) = register_identity_and_agent(1, b"agent-base");
        bond_agent(1, identity_id, agent_id, 100_000);

        assert_ok!(AgentIncentives::set_reward_config(
            RuntimeOrigin::root(),
            reward_config(),
        ));
        assert_ok!(AgentIncentives::set_reward_settlement_publisher(
            RuntimeOrigin::root(),
            Some(99),
        ));

        let before_owner_balance = Balances::free_balance(1);
        let before_reserve_balance = Balances::free_balance(REWARD_RESERVE);
        assert_ok!(AgentIncentives::settle_base_staking_day(
            RuntimeOrigin::signed(99),
            0,
            vec![AgentRef {
                identity_id,
                agent_id
            }]
            .try_into()
            .unwrap(),
        ));

        let ledger = AgentRewardLedgers::<Test>::get((identity_id, agent_id));
        assert_eq!(ledger.claimable_base, 82);
        assert_eq!(ledger.claimable_total, 82);
        assert!(System::events().iter().any(|record| matches!(
            record.event,
            RuntimeEvent::AgentIncentives(super::Event::AgentRewardCredited {
                identity_id: event_identity_id,
                agent_id: event_agent_id,
                day_index: 0,
                kind: RewardCreditKind::Base,
                amount: 82,
            }) if event_identity_id == identity_id && event_agent_id == agent_id
        )));

        assert_ok!(AgentIncentives::claim_agent_rewards(
            RuntimeOrigin::signed(1),
            identity_id,
            agent_id,
        ));

        let updated = AgentRewardLedgers::<Test>::get((identity_id, agent_id));
        assert_eq!(updated.claimable_total, 0);
        assert_eq!(updated.claimed_total, 82);
        assert_eq!(updated.claimed_base, 82);
        assert_eq!(Balances::free_balance(1), before_owner_balance + 82);
        assert_eq!(
            Balances::free_balance(REWARD_RESERVE),
            before_reserve_balance - 82
        );
    });
}

#[test]
fn payment_capability_key_can_claim_but_other_keys_cannot() {
    new_test_ext().execute_with(|| {
        let (identity_id, agent_id) = register_identity_and_agent(1, b"agent-payment-key");
        bond_agent(1, identity_id, agent_id, 100_000);

        assert_ok!(IdentityCore::add_key(
            RuntimeOrigin::signed(1),
            identity_id,
            2,
            KeyPurpose::Finance,
            CAP_MANAGE_PAYMENT,
            None,
        ));
        assert_ok!(IdentityCore::add_key(
            RuntimeOrigin::signed(1),
            identity_id,
            3,
            KeyPurpose::Session,
            0,
            None,
        ));

        assert_ok!(AgentIncentives::set_reward_config(
            RuntimeOrigin::root(),
            reward_config(),
        ));
        assert_ok!(AgentIncentives::set_reward_settlement_publisher(
            RuntimeOrigin::root(),
            Some(99),
        ));
        assert_ok!(AgentIncentives::settle_base_staking_day(
            RuntimeOrigin::signed(99),
            0,
            vec![AgentRef {
                identity_id,
                agent_id
            }]
            .try_into()
            .unwrap(),
        ));

        assert_noop!(
            AgentIncentives::claim_agent_rewards(RuntimeOrigin::signed(3), identity_id, agent_id,),
            pallet_identity_core::Error::<Test>::Unauthorized
        );
        assert_ok!(AgentIncentives::claim_agent_rewards(
            RuntimeOrigin::signed(2),
            identity_id,
            agent_id,
        ));
    });
}

#[test]
fn settlement_publisher_is_enforced() {
    new_test_ext().execute_with(|| {
        let (identity_id, agent_id) = register_identity_and_agent(1, b"agent-auth");
        bond_agent(1, identity_id, agent_id, 10_000);

        assert_ok!(AgentIncentives::set_reward_config(
            RuntimeOrigin::root(),
            reward_config(),
        ));
        assert_ok!(AgentIncentives::set_reward_settlement_publisher(
            RuntimeOrigin::root(),
            Some(99),
        ));

        assert_noop!(
            AgentIncentives::settle_base_staking_day(
                RuntimeOrigin::signed(1),
                0,
                vec![AgentRef {
                    identity_id,
                    agent_id
                }]
                .try_into()
                .unwrap(),
            ),
            Error::<Test>::UnauthorizedSettlementPublisher
        );
    });
}

#[test]
fn observer_and_task_caps_are_applied() {
    new_test_ext().execute_with(|| {
        let mut config = reward_config();
        config.observer_reviewer_pool = 365_000;
        config.task_market_pool = 365_000;
        config.auto_emission_pool =
            config.base_staking_pool + config.observer_reviewer_pool + config.task_market_pool;
        config.total_reward_pool = config.auto_emission_pool + config.reserve_pool;
        config.agent_daily_total_protocol_reward_cap = 1_500;
        config.agent_daily_task_reward_cap = 2_000;
        config.task_max_subsidy = 2_000;
        config.difficulty_schedule.critical = 2_000;

        let (identity_id, agent_id) = register_identity_and_agent(1, b"agent-caps");
        bond_agent(1, identity_id, agent_id, 20_000);

        assert_ok!(AgentIncentives::set_reward_config(
            RuntimeOrigin::root(),
            config,
        ));
        assert_ok!(AgentIncentives::set_reward_settlement_publisher(
            RuntimeOrigin::root(),
            Some(99),
        ));

        let participants = vec![AgentRef {
            identity_id,
            agent_id,
        }]
        .try_into()
        .unwrap();
        assert_ok!(AgentIncentives::settle_observer_round(
            RuntimeOrigin::signed(99),
            b"round-1".to_vec().try_into().unwrap(),
            0,
            participants,
        ));
        let after_observer = AgentRewardLedgers::<Test>::get((identity_id, agent_id));
        assert_eq!(after_observer.claimable_observer, 600);

        assert_ok!(AgentIncentives::settle_task_reward(
            RuntimeOrigin::signed(99),
            b"task-1".to_vec().try_into().unwrap(),
            0,
            AgentRef {
                identity_id,
                agent_id
            },
            TaskDifficulty::Critical,
        ));

        let ledger = AgentRewardLedgers::<Test>::get((identity_id, agent_id));
        assert_eq!(ledger.claimable_observer, 600);
        assert_eq!(ledger.claimable_task, 900);
        assert_eq!(ledger.claimable_total, 1_500);

        let usage = super::AgentDailyUsages::<Test>::get((0, identity_id, agent_id));
        assert_eq!(usage.observer_reviewer_amount, 600);
        assert_eq!(usage.task_amount, 900);
        assert_eq!(usage.total_protocol_amount, 1_500);
    });
}

#[test]
fn duplicate_task_settlement_is_rejected() {
    new_test_ext().execute_with(|| {
        let (identity_id, agent_id) = register_identity_and_agent(1, b"agent-task");
        bond_agent(1, identity_id, agent_id, 10_000);

        assert_ok!(AgentIncentives::set_reward_config(
            RuntimeOrigin::root(),
            reward_config(),
        ));
        assert_ok!(AgentIncentives::set_reward_settlement_publisher(
            RuntimeOrigin::root(),
            Some(99),
        ));

        let task_id: frame::prelude::BoundedVec<u8, <Test as super::Config>::MaxExternalIdLen> =
            b"task-dup".to_vec().try_into().unwrap();
        assert_ok!(AgentIncentives::settle_task_reward(
            RuntimeOrigin::signed(99),
            task_id.clone(),
            0,
            AgentRef {
                identity_id,
                agent_id
            },
            TaskDifficulty::Hard,
        ));
        assert_noop!(
            AgentIncentives::settle_task_reward(
                RuntimeOrigin::signed(99),
                task_id,
                0,
                AgentRef {
                    identity_id,
                    agent_id
                },
                TaskDifficulty::Hard,
            ),
            Error::<Test>::AlreadySettled
        );
    });
}

#[test]
fn day_state_rollover_is_recorded() {
    new_test_ext().execute_with(|| {
        let config = reward_config();
        assert_ok!(AgentIncentives::set_reward_config(
            RuntimeOrigin::root(),
            config.clone(),
        ));
        assert_ok!(AgentIncentives::set_reward_settlement_publisher(
            RuntimeOrigin::root(),
            Some(99),
        ));

        let (identity_id, agent_id) = register_identity_and_agent(1, b"agent-rollover");
        bond_agent(1, identity_id, agent_id, 1_000);
        assert_ok!(AgentIncentives::settle_base_staking_day(
            RuntimeOrigin::signed(99),
            0,
            vec![AgentRef {
                identity_id,
                agent_id
            }]
            .try_into()
            .unwrap(),
        ));

        let day_zero = DailyEmissionStates::<Test>::get(0).expect("day zero");
        let day_one = DailyEmissionStates::<Test>::get(1).unwrap_or_else(|| {
            AgentIncentives::settle_task_reward(
                RuntimeOrigin::signed(99),
                b"task-rollover".to_vec().try_into().unwrap(),
                1,
                AgentRef {
                    identity_id,
                    agent_id,
                },
                TaskDifficulty::Easy,
            )
            .ok();
            DailyEmissionStates::<Test>::get(1).expect("day one")
        });

        assert_eq!(
            day_one.base_staking_budget,
            (config.base_staking_pool / config.planned_emission_days as u128)
                + day_zero.rollover_base_staking
        );
    });
}

#[test]
fn reward_days_must_be_settled_in_sequence() {
    new_test_ext().execute_with(|| {
        let mut config = reward_config();
        config.emission_start_day = 10;
        assert_ok!(AgentIncentives::set_reward_config(
            RuntimeOrigin::root(),
            config,
        ));
        assert_ok!(AgentIncentives::set_reward_settlement_publisher(
            RuntimeOrigin::root(),
            Some(99),
        ));

        let (identity_id, agent_id) = register_identity_and_agent(1, b"agent-day-sequence");
        bond_agent(1, identity_id, agent_id, 1_000);
        let participants = || {
            vec![AgentRef {
                identity_id,
                agent_id,
            }]
            .try_into()
            .unwrap()
        };

        assert_noop!(
            AgentIncentives::settle_base_staking_day(RuntimeOrigin::signed(99), 9, participants()),
            Error::<Test>::RewardEmissionEnded
        );
        assert_noop!(
            AgentIncentives::settle_base_staking_day(RuntimeOrigin::signed(99), 11, participants()),
            Error::<Test>::PreviousRewardDayMissing
        );
        assert_ok!(AgentIncentives::settle_base_staking_day(
            RuntimeOrigin::signed(99),
            10,
            participants(),
        ));
    });
}

#[test]
fn reward_emission_end_is_enforced() {
    new_test_ext().execute_with(|| {
        let mut config = reward_config();
        config.planned_emission_days = 1;
        assert_ok!(AgentIncentives::set_reward_config(
            RuntimeOrigin::root(),
            config,
        ));
        assert_ok!(AgentIncentives::set_reward_settlement_publisher(
            RuntimeOrigin::root(),
            Some(99),
        ));

        let (identity_id, agent_id) = register_identity_and_agent(1, b"agent-ended");
        bond_agent(1, identity_id, agent_id, 1_000);
        assert_noop!(
            AgentIncentives::settle_task_reward(
                RuntimeOrigin::signed(99),
                b"task-ended".to_vec().try_into().unwrap(),
                1,
                AgentRef {
                    identity_id,
                    agent_id
                },
                TaskDifficulty::Easy,
            ),
            Error::<Test>::RewardEmissionEnded
        );
    });
}

#[test]
fn invalid_pool_config_is_rejected() {
    new_test_ext().execute_with(|| {
        let mut config = reward_config();
        config.task_market_pool = config.task_market_pool.saturating_sub(1);
        assert_noop!(
            AgentIncentives::set_reward_config(RuntimeOrigin::root(), config),
            Error::<Test>::InvalidRewardConfig
        );

        let mut config = reward_config();
        config.reviewer_share_bps = 3_000;
        assert_noop!(
            AgentIncentives::set_reward_config(RuntimeOrigin::root(), config),
            Error::<Test>::InvalidRewardConfig
        );
    });
}
