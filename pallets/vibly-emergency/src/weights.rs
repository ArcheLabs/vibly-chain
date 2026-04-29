use frame::deps::frame_support::weights::Weight;

pub trait WeightInfo {
    fn pause() -> Weight;
    fn resume() -> Weight;
    fn cancel() -> Weight;
}

impl WeightInfo for () {
    fn pause() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn resume() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn cancel() -> Weight {
        Weight::from_parts(10_000, 0)
    }
}

pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);
impl<T> WeightInfo for SubstrateWeight<T> {
    fn pause() -> Weight {
        <() as WeightInfo>::pause()
    }
    fn resume() -> Weight {
        <() as WeightInfo>::resume()
    }
    fn cancel() -> Weight {
        <() as WeightInfo>::cancel()
    }
}
