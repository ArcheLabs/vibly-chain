use frame::deps::frame_support::weights::Weight;

pub trait WeightInfo {
    fn set_claim_root() -> Weight;
    fn claim() -> Weight;
    fn set_claim_paused() -> Weight;
    fn set_claim_root_publisher() -> Weight;
}

impl WeightInfo for () {
    fn set_claim_root() -> Weight { Weight::from_parts(10_000, 0) }
    fn claim() -> Weight { Weight::from_parts(10_000, 0) }
    fn set_claim_paused() -> Weight { Weight::from_parts(10_000, 0) }
    fn set_claim_root_publisher() -> Weight { Weight::from_parts(10_000, 0) }
}

pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);

impl<T> WeightInfo for SubstrateWeight<T> {
    fn set_claim_root() -> Weight { <() as WeightInfo>::set_claim_root() }
    fn claim() -> Weight { <() as WeightInfo>::claim() }
    fn set_claim_paused() -> Weight { <() as WeightInfo>::set_claim_paused() }
    fn set_claim_root_publisher() -> Weight { <() as WeightInfo>::set_claim_root_publisher() }
}
