use frame::deps::frame_support::weights::Weight;

pub trait WeightInfo {
    fn register_identity() -> Weight;
    fn rotate_owner_key() -> Weight;
    fn set_recovery_key() -> Weight;
    fn add_key() -> Weight;
    fn revoke_key() -> Weight;
    fn set_active_profile() -> Weight;
    fn set_active_agent_registry() -> Weight;
    fn set_active_auth_registry() -> Weight;
    fn set_active_relation_policy() -> Weight;
    fn bind_transport() -> Weight;
    fn verify_transport() -> Weight;
    fn revoke_transport() -> Weight;
    fn freeze_identity() -> Weight;
    fn unfreeze_identity() -> Weight;
    fn disable_identity() -> Weight;
}

impl WeightInfo for () {
    fn register_identity() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn rotate_owner_key() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn set_recovery_key() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn add_key() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn revoke_key() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn set_active_profile() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn set_active_agent_registry() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn set_active_auth_registry() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn set_active_relation_policy() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn bind_transport() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn verify_transport() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn revoke_transport() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn freeze_identity() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn unfreeze_identity() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn disable_identity() -> Weight {
        Weight::from_parts(10_000, 0)
    }
}

pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);
impl<T> WeightInfo for SubstrateWeight<T> {
    fn register_identity() -> Weight {
        <() as WeightInfo>::register_identity()
    }
    fn rotate_owner_key() -> Weight {
        <() as WeightInfo>::rotate_owner_key()
    }
    fn set_recovery_key() -> Weight {
        <() as WeightInfo>::set_recovery_key()
    }
    fn add_key() -> Weight {
        <() as WeightInfo>::add_key()
    }
    fn revoke_key() -> Weight {
        <() as WeightInfo>::revoke_key()
    }
    fn set_active_profile() -> Weight {
        <() as WeightInfo>::set_active_profile()
    }
    fn set_active_agent_registry() -> Weight {
        <() as WeightInfo>::set_active_agent_registry()
    }
    fn set_active_auth_registry() -> Weight {
        <() as WeightInfo>::set_active_auth_registry()
    }
    fn set_active_relation_policy() -> Weight {
        <() as WeightInfo>::set_active_relation_policy()
    }
    fn bind_transport() -> Weight {
        <() as WeightInfo>::bind_transport()
    }
    fn verify_transport() -> Weight {
        <() as WeightInfo>::verify_transport()
    }
    fn revoke_transport() -> Weight {
        <() as WeightInfo>::revoke_transport()
    }
    fn freeze_identity() -> Weight {
        <() as WeightInfo>::freeze_identity()
    }
    fn unfreeze_identity() -> Weight {
        <() as WeightInfo>::unfreeze_identity()
    }
    fn disable_identity() -> Weight {
        <() as WeightInfo>::disable_identity()
    }
}
