use frame::deps::frame_support::weights::Weight;

pub trait WeightInfo {
    fn bond_agent() -> Weight;
    fn request_unbond() -> Weight;
    fn cancel_unbond() -> Weight;
    fn block_release() -> Weight;
    fn clear_release_block() -> Weight;
    fn release_unbond() -> Weight;
}

impl WeightInfo for () {
    fn bond_agent() -> Weight { Weight::from_parts(10_000, 0) }
    fn request_unbond() -> Weight { Weight::from_parts(10_000, 0) }
    fn cancel_unbond() -> Weight { Weight::from_parts(10_000, 0) }
    fn block_release() -> Weight { Weight::from_parts(10_000, 0) }
    fn clear_release_block() -> Weight { Weight::from_parts(10_000, 0) }
    fn release_unbond() -> Weight { Weight::from_parts(10_000, 0) }
}

pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);

impl<T> WeightInfo for SubstrateWeight<T> {
    fn bond_agent() -> Weight { <() as WeightInfo>::bond_agent() }
    fn request_unbond() -> Weight { <() as WeightInfo>::request_unbond() }
    fn cancel_unbond() -> Weight { <() as WeightInfo>::cancel_unbond() }
    fn block_release() -> Weight { <() as WeightInfo>::block_release() }
    fn clear_release_block() -> Weight { <() as WeightInfo>::clear_release_block() }
    fn release_unbond() -> Weight { <() as WeightInfo>::release_unbond() }
}
