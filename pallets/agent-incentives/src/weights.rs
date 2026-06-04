use frame::deps::frame_support::weights::Weight;

pub trait WeightInfo {
    fn set_reward_config() -> Weight;
    fn set_reward_settlement_publisher() -> Weight;
    fn settle_base_staking_day() -> Weight;
    fn settle_observer_round() -> Weight;
    fn settle_reviewer_round() -> Weight;
    fn settle_task_reward() -> Weight;
    fn claim_agent_rewards() -> Weight;
}

impl WeightInfo for () {
    fn set_reward_config() -> Weight { Weight::from_parts(10_000, 0) }
    fn set_reward_settlement_publisher() -> Weight { Weight::from_parts(10_000, 0) }
    fn settle_base_staking_day() -> Weight { Weight::from_parts(10_000, 0) }
    fn settle_observer_round() -> Weight { Weight::from_parts(10_000, 0) }
    fn settle_reviewer_round() -> Weight { Weight::from_parts(10_000, 0) }
    fn settle_task_reward() -> Weight { Weight::from_parts(10_000, 0) }
    fn claim_agent_rewards() -> Weight { Weight::from_parts(10_000, 0) }
}

pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);

impl<T> WeightInfo for SubstrateWeight<T> {
    fn set_reward_config() -> Weight { <() as WeightInfo>::set_reward_config() }
    fn set_reward_settlement_publisher() -> Weight { <() as WeightInfo>::set_reward_settlement_publisher() }
    fn settle_base_staking_day() -> Weight { <() as WeightInfo>::settle_base_staking_day() }
    fn settle_observer_round() -> Weight { <() as WeightInfo>::settle_observer_round() }
    fn settle_reviewer_round() -> Weight { <() as WeightInfo>::settle_reviewer_round() }
    fn settle_task_reward() -> Weight { <() as WeightInfo>::settle_task_reward() }
    fn claim_agent_rewards() -> Weight { <() as WeightInfo>::claim_agent_rewards() }
}
