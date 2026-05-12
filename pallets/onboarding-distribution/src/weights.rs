use frame::deps::frame_support::weights::Weight;

pub trait WeightInfo {
    fn set_relayer() -> Weight;
    fn set_distribution_limits() -> Weight;
    fn register_evm_airdrop() -> Weight;
    fn rotate_root_for_evm() -> Weight;
    fn issue_dot_conversion() -> Weight;
    fn register_agent() -> Weight;
    fn set_agent_registrar() -> Weight;
    fn revoke_agent_registrar() -> Weight;
}

impl WeightInfo for () {
    fn set_relayer() -> Weight { Weight::from_parts(10_000, 0) }
    fn set_distribution_limits() -> Weight { Weight::from_parts(10_000, 0) }
    fn register_evm_airdrop() -> Weight { Weight::from_parts(20_000, 0) }
    fn rotate_root_for_evm() -> Weight { Weight::from_parts(10_000, 0) }
    fn issue_dot_conversion() -> Weight { Weight::from_parts(15_000, 0) }
    fn register_agent() -> Weight { Weight::from_parts(10_000, 0) }
    fn set_agent_registrar() -> Weight { Weight::from_parts(10_000, 0) }
    fn revoke_agent_registrar() -> Weight { Weight::from_parts(10_000, 0) }
}

pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);
impl<T> WeightInfo for SubstrateWeight<T> {
    fn set_relayer() -> Weight { <() as WeightInfo>::set_relayer() }
    fn set_distribution_limits() -> Weight { <() as WeightInfo>::set_distribution_limits() }
    fn register_evm_airdrop() -> Weight { <() as WeightInfo>::register_evm_airdrop() }
    fn rotate_root_for_evm() -> Weight { <() as WeightInfo>::rotate_root_for_evm() }
    fn issue_dot_conversion() -> Weight { <() as WeightInfo>::issue_dot_conversion() }
    fn register_agent() -> Weight { <() as WeightInfo>::register_agent() }
    fn set_agent_registrar() -> Weight { <() as WeightInfo>::set_agent_registrar() }
    fn revoke_agent_registrar() -> Weight { <() as WeightInfo>::revoke_agent_registrar() }
}
