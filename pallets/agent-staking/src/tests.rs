use crate::mock::{
    new_test_ext, AgentStaking, Balances, IdentityCore, OnboardingDistribution,
    RuntimeEvent, RuntimeOrigin, System, Test,
};
use crate::{AgentStakeLedgers, AgentStakeStatus};
use frame::deps::frame_support::{assert_noop, assert_ok};
use frame::traits::tokens::fungible::InspectHold;
use sp_runtime::traits::{BlakeTwo256, Hash as HashT};
use vibly_primitives_common::{ContentRef, Hash256};

fn registered_identity() -> Hash256 {
    match System::events().pop().unwrap().event {
        RuntimeEvent::IdentityCore(pallet_identity_core::Event::IdentityRegistered { identity_id, .. }) => identity_id,
        other => panic!("unexpected event: {other:?}"),
    }
}

fn agent_ref(byte: u8) -> ContentRef<<Test as crate::Config>::MaxReasonCidLen, <Test as crate::Config>::MaxReasonUriLen> {
    ContentRef::Hash(Hash256::repeat_byte(byte))
}

fn register_identity_and_agent(registrar: u64) -> (Hash256, Hash256) {
    assert_ok!(IdentityCore::register_identity(RuntimeOrigin::signed(1), None, None, None, None, None));
    let identity_id = registered_identity();
    if registrar != 1 {
        assert_ok!(OnboardingDistribution::set_agent_registrar(
            RuntimeOrigin::signed(1),
            identity_id,
            registrar,
        ));
    }
    let agent_ref = agent_ref(9);
    assert_ok!(OnboardingDistribution::register_agent(
        RuntimeOrigin::signed(registrar),
        identity_id,
        agent_ref.clone(),
    ));
    let agent_id = BlakeTwo256::hash_of(&(b"vibly/agent", identity_id, &registrar, &agent_ref));
    (identity_id, agent_id)
}

#[test]
fn root_can_bond_and_unbond_registered_agent() {
    new_test_ext().execute_with(|| {
        let (identity_id, agent_id) = register_identity_and_agent(1);

        assert_ok!(AgentStaking::bond_agent(RuntimeOrigin::signed(1), identity_id, agent_id, 100));
        let ledger = AgentStakeLedgers::<Test>::get((identity_id, agent_id)).unwrap();
        assert_eq!(ledger.active_amount, 100);
        assert_eq!(ledger.status, AgentStakeStatus::Active);
        assert_eq!(Balances::balance_on_hold(&crate::HoldReason::AgentStake.into(), &1), 100);

        assert_ok!(AgentStaking::request_unbond(RuntimeOrigin::signed(1), identity_id, agent_id, 40));
        let ledger = AgentStakeLedgers::<Test>::get((identity_id, agent_id)).unwrap();
        assert_eq!(ledger.active_amount, 60);
        assert_eq!(ledger.unbonding_amount, 40);
        assert_eq!(ledger.status, AgentStakeStatus::Active);

        System::set_block_number(6);
        assert_ok!(AgentStaking::release_unbond(RuntimeOrigin::signed(1), identity_id, agent_id));
        assert_eq!(Balances::balance_on_hold(&crate::HoldReason::AgentStake.into(), &1), 60);
    });
}

#[test]
fn delegated_agent_registrar_can_bond() {
    new_test_ext().execute_with(|| {
        let (identity_id, agent_id) = register_identity_and_agent(2);
        assert_ok!(AgentStaking::bond_agent(RuntimeOrigin::signed(2), identity_id, agent_id, 125));
        let ledger = AgentStakeLedgers::<Test>::get((identity_id, agent_id)).unwrap();
        assert_eq!(ledger.active_amount, 125);
        assert_eq!(ledger.last_funding_account, Some(2));
    });
}

#[test]
fn unregistered_or_unauthorized_stake_is_rejected() {
    new_test_ext().execute_with(|| {
        let (identity_id, agent_id) = register_identity_and_agent(1);
        assert_noop!(
            AgentStaking::bond_agent(RuntimeOrigin::signed(3), identity_id, agent_id, 10),
            pallet_identity_core::Error::<Test>::Unauthorized,
        );
        assert_noop!(
            AgentStaking::bond_agent(RuntimeOrigin::signed(1), identity_id, Hash256::repeat_byte(1), 10),
            crate::Error::<Test>::AgentNotRegistered,
        );
    });
}

#[test]
fn release_can_be_blocked_and_cleared() {
    new_test_ext().execute_with(|| {
        let (identity_id, agent_id) = register_identity_and_agent(1);
        assert_ok!(AgentStaking::bond_agent(RuntimeOrigin::signed(1), identity_id, agent_id, 100));
        assert_ok!(AgentStaking::request_unbond(RuntimeOrigin::signed(1), identity_id, agent_id, 100));
        System::set_block_number(6);

        assert_ok!(AgentStaking::block_release(RuntimeOrigin::root(), identity_id, agent_id, None));
        assert_noop!(
            AgentStaking::release_unbond(RuntimeOrigin::signed(1), identity_id, agent_id),
            crate::Error::<Test>::ReleaseBlocked,
        );
        assert_ok!(AgentStaking::clear_release_block(RuntimeOrigin::root(), identity_id, agent_id));
        assert_ok!(AgentStaking::release_unbond(RuntimeOrigin::signed(1), identity_id, agent_id));
        let ledger = AgentStakeLedgers::<Test>::get((identity_id, agent_id)).unwrap();
        assert_eq!(ledger.status, AgentStakeStatus::Released);
    });
}
