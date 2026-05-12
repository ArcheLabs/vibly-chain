use super::*;
use frame_support::{assert_noop, assert_ok};
use pallet_vibly_emergency::{EmergencyStatus, Error as EmergencyError, StatusByScope};
use sp_core::sr25519;
use sp_runtime::{
    traits::IdentifyAccount,
    BuildStorage, DispatchError,
    MultiSigner,
};

fn account(seed: u8) -> AccountId {
    MultiSigner::from(sr25519::Public::from_raw([seed; 32])).into_account()
}

fn new_test_ext() -> sp_io::TestExternalities {
    let alice = account(1);
    let bob = account(2);
    let charlie = account(3);
    let dave = account(4);

    let storage = RuntimeGenesisConfig {
        system: Default::default(),
        balances: pallet_balances::GenesisConfig {
            balances: vec![
                (alice.clone(), 1_000 * UNIT),
                (bob.clone(), 1_000 * UNIT),
                (charlie.clone(), 1_000 * UNIT),
                (dave, 1_000 * UNIT),
            ],
            ..Default::default()
        },
        sudo: pallet_sudo::GenesisConfig { key: Some(alice.clone()) },
        aura: pallet_aura::GenesisConfig {
            authorities: vec![AuraId::from(sp_core::sr25519::Public::from_raw([1; 32]))],
        },
        grandpa: pallet_grandpa::GenesisConfig {
            authorities: vec![(
                fg_primitives::AuthorityId::from(sp_core::ed25519::Public::from_raw([1; 32])),
                1,
            )],
            ..Default::default()
        },
        guardian_membership: pallet_membership::GenesisConfig {
            members: vec![alice, bob, charlie].try_into().unwrap(),
            ..Default::default()
        },
        guardian_collective: Default::default(),
        transaction_payment: Default::default(),
    }
    .build_storage()
    .unwrap();

    let mut ext: sp_io::TestExternalities = storage.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn guardian_member_can_pause_proposal() {
    new_test_ext().execute_with(|| {
        let alice = account(1);
        let scope = EmergencyScope::Proposal(1);

        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(alice), scope.clone(), None));
        assert_eq!(StatusByScope::<Runtime>::get(scope), EmergencyStatus::Paused);
    });
}

#[test]
fn non_guardian_cannot_pause_proposal() {
    new_test_ext().execute_with(|| {
        let dave = account(4);
        assert_noop!(
            ViblyEmergency::pause(RuntimeOrigin::signed(dave), EmergencyScope::Proposal(1), None),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn collective_two_thirds_can_resume_paused_proposal() {
    new_test_ext().execute_with(|| {
        let alice = account(1);
        let scope = EmergencyScope::Proposal(1);

        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(alice), scope.clone(), None));
        assert_ok!(ViblyEmergency::resume(
            pallet_collective::RawOrigin::Members(2, 3).into(),
            scope.clone(),
            None,
        ));
        assert_eq!(StatusByScope::<Runtime>::get(scope), EmergencyStatus::Active);
    });
}

#[test]
fn collective_two_thirds_can_cancel_paused_proposal() {
    new_test_ext().execute_with(|| {
        let alice = account(1);
        let scope = EmergencyScope::Proposal(1);

        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(alice), scope.clone(), None));
        assert_ok!(ViblyEmergency::cancel(
            pallet_collective::RawOrigin::Members(2, 3).into(),
            scope.clone(),
            None,
        ));
        assert_eq!(StatusByScope::<Runtime>::get(scope), EmergencyStatus::Cancelled);
    });
}

#[test]
fn cancelled_proposal_cannot_resume() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(1);

        assert_ok!(ViblyEmergency::cancel(
            pallet_collective::RawOrigin::Members(2, 3).into(),
            scope.clone(),
            None,
        ));
        assert_noop!(
            ViblyEmergency::resume(
                pallet_collective::RawOrigin::Members(2, 3).into(),
                scope,
                None,
            ),
            EmergencyError::<Runtime>::AlreadyCancelled
        );
    });
}
