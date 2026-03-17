use super::*;
use frame::{deps::frame_benchmarking::v2::*, prelude::*};

#[benchmarks]
mod benchmarks {
    use super::*;
    #[cfg(test)]
    use crate::pallet::Pallet as IdentityCore;
    use frame_system::RawOrigin;

    #[benchmark]
    fn register_identity() {
        let caller: T::AccountId = whitelisted_caller();
        #[extrinsic_call]
        register_identity(RawOrigin::Signed(caller), None, None, None, None, None);
    }

    impl_benchmark_test_suite!(IdentityCore, crate::mock::new_test_ext(), crate::mock::Test);
}
