use super::*;
use frame::{deps::frame_benchmarking::v2::*, prelude::*};

#[benchmarks]
mod benchmarks {
    use super::*;
    #[cfg(test)]
    use crate::pallet::Pallet as PaymentIntent;
    use frame_system::RawOrigin;
    use sp_core::H256;

    fn example_action<T: Config>() -> PaymentActionOf<T> {
        PaymentAction {
            namespace: b"service".to_vec().try_into().unwrap(),
            action_code: 1,
            payload_ref: None,
        }
    }

    #[benchmark]
    fn create_payment_intent() {
        let caller: T::AccountId = whitelisted_caller();
        let intent_id = H256::repeat_byte(1);
        #[extrinsic_call]
        create_payment_intent(
            RawOrigin::Signed(caller),
            intent_id,
            H256::repeat_byte(2),
            H256::repeat_byte(3),
            0,
            10,
            example_action::<T>(),
            None,
            SettlementMode::Hold,
            None,
        );
    }

    impl_benchmark_test_suite!(
        PaymentIntent,
        crate::mock::new_test_ext(),
        crate::mock::Test
    );
}
