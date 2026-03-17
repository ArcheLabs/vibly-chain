use frame::deps::frame_support::weights::Weight;

pub trait WeightInfo {
    fn create_payment_intent() -> Weight;
    fn fund_payment_intent() -> Weight;
    fn claim_payment_intent() -> Weight;
    fn refund_payment_intent() -> Weight;
    fn cancel_payment_intent() -> Weight;
    fn expire_payment_intent() -> Weight;
}

impl WeightInfo for () {
    fn create_payment_intent() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn fund_payment_intent() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn claim_payment_intent() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn refund_payment_intent() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn cancel_payment_intent() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn expire_payment_intent() -> Weight {
        Weight::from_parts(10_000, 0)
    }
}

pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);
impl<T> WeightInfo for SubstrateWeight<T> {
    fn create_payment_intent() -> Weight {
        <() as WeightInfo>::create_payment_intent()
    }
    fn fund_payment_intent() -> Weight {
        <() as WeightInfo>::fund_payment_intent()
    }
    fn claim_payment_intent() -> Weight {
        <() as WeightInfo>::claim_payment_intent()
    }
    fn refund_payment_intent() -> Weight {
        <() as WeightInfo>::refund_payment_intent()
    }
    fn cancel_payment_intent() -> Weight {
        <() as WeightInfo>::cancel_payment_intent()
    }
    fn expire_payment_intent() -> Weight {
        <() as WeightInfo>::expire_payment_intent()
    }
}
