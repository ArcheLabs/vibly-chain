use crate::mock::{
    new_test_ext, Balances, IdentityCore, OnboardingDistribution, RuntimeOrigin, Test, Timestamp,
};
use frame::deps::frame_support::{assert_noop, assert_ok};
use sp_core::H256;
use vibly_primitives_common::ContentRef;

fn evm(byte: u8) -> [u8; 20] {
    [byte; 20]
}

fn agent_ref(
    bytes: &[u8],
) -> ContentRef<<Test as crate::Config>::MaxAgentRefCidLen, <Test as crate::Config>::MaxAgentRefUriLen>
{
    ContentRef::Cid(bytes.to_vec().try_into().unwrap())
}

#[test]
fn evm_airdrop_registers_identity_and_prevents_duplicates() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(10);
        assert_ok!(OnboardingDistribution::set_distribution_limits(
            RuntimeOrigin::root(),
            1_000,
            1_000,
            200,
            200,
        ));
        assert_ok!(OnboardingDistribution::set_relayer(RuntimeOrigin::root(), 9, true));

        assert_noop!(
            OnboardingDistribution::register_evm_airdrop(
                RuntimeOrigin::signed(8),
                evm(1),
                1,
                2,
                100,
                10,
            ),
            crate::Error::<Test>::UnauthorizedRelayer
        );

        assert_ok!(OnboardingDistribution::register_evm_airdrop(
            RuntimeOrigin::signed(9),
            evm(1),
            1,
            2,
            100,
            10,
        ));
        assert_eq!(Balances::free_balance(1), 100);
        assert_eq!(Balances::free_balance(2), 10);

        assert_noop!(
            OnboardingDistribution::register_evm_airdrop(
                RuntimeOrigin::signed(9),
                evm(1),
                3,
                4,
                100,
                10,
            ),
            crate::Error::<Test>::AirdropAlreadyClaimed
        );
    });
}

#[test]
fn caps_and_dot_payment_uniqueness_are_enforced() {
    new_test_ext().execute_with(|| {
        assert_ok!(OnboardingDistribution::set_distribution_limits(
            RuntimeOrigin::root(),
            1_000,
            50,
            200,
            40,
        ));
        assert_ok!(OnboardingDistribution::set_relayer(RuntimeOrigin::root(), 9, true));
        assert_ok!(OnboardingDistribution::register_evm_airdrop(
            RuntimeOrigin::signed(9),
            evm(1),
            1,
            2,
            100,
            10,
        ));
        let identity_id = pallet_identity_core::IdentityIdByEvmAddress::<Test>::get(evm(1)).unwrap();
        let payment_id = H256::repeat_byte(7);

        assert_noop!(
            OnboardingDistribution::issue_dot_conversion(
                RuntimeOrigin::signed(9),
                identity_id,
                payment_id,
                5,
                41,
            ),
            crate::Error::<Test>::ConversionClaimTooLarge
        );

        assert_ok!(OnboardingDistribution::issue_dot_conversion(
            RuntimeOrigin::signed(9),
            identity_id,
            payment_id,
            5,
            40,
        ));
        assert_noop!(
            OnboardingDistribution::issue_dot_conversion(
                RuntimeOrigin::signed(9),
                identity_id,
                payment_id,
                5,
                1,
            ),
            crate::Error::<Test>::DotPaymentAlreadyIssued
        );
    });
}

#[test]
fn agent_registrar_can_register_agent_but_not_rotate_root() {
    new_test_ext().execute_with(|| {
        assert_ok!(OnboardingDistribution::set_relayer(RuntimeOrigin::root(), 9, true));
        assert_ok!(OnboardingDistribution::register_evm_airdrop(
            RuntimeOrigin::signed(9),
            evm(1),
            1,
            2,
            100,
            10,
        ));
        let identity_id = pallet_identity_core::IdentityIdByEvmAddress::<Test>::get(evm(1)).unwrap();

        assert_ok!(OnboardingDistribution::register_agent(
            RuntimeOrigin::signed(2),
            identity_id,
            agent_ref(b"agent-a"),
        ));
        assert_noop!(
            IdentityCore::rotate_owner_key(
                RuntimeOrigin::signed(2),
                identity_id,
                3,
            ),
            pallet_identity_core::Error::<Test>::Unauthorized
        );
    });
}

#[test]
fn root_can_replace_and_revoke_agent_registrar() {
    new_test_ext().execute_with(|| {
        assert_ok!(OnboardingDistribution::set_relayer(RuntimeOrigin::root(), 9, true));
        assert_ok!(OnboardingDistribution::register_evm_airdrop(
            RuntimeOrigin::signed(9),
            evm(1),
            1,
            2,
            100,
            10,
        ));
        let identity_id = pallet_identity_core::IdentityIdByEvmAddress::<Test>::get(evm(1)).unwrap();

        assert_ok!(OnboardingDistribution::set_agent_registrar(
            RuntimeOrigin::signed(1),
            identity_id,
            4,
        ));
        assert_noop!(
            OnboardingDistribution::register_agent(
                RuntimeOrigin::signed(2),
                identity_id,
                agent_ref(b"old-registrar"),
            ),
            pallet_identity_core::Error::<Test>::Unauthorized
        );
        assert_ok!(OnboardingDistribution::register_agent(
            RuntimeOrigin::signed(4),
            identity_id,
            agent_ref(b"new-registrar"),
        ));

        assert_ok!(OnboardingDistribution::revoke_agent_registrar(
            RuntimeOrigin::signed(1),
            identity_id,
        ));
        assert_noop!(
            OnboardingDistribution::register_agent(
                RuntimeOrigin::signed(4),
                identity_id,
                agent_ref(b"revoked-registrar"),
            ),
            pallet_identity_core::Error::<Test>::Unauthorized
        );
    });
}

#[test]
fn root_rotation_keeps_identity_and_balances_in_place() {
    new_test_ext().execute_with(|| {
        assert_ok!(OnboardingDistribution::set_relayer(RuntimeOrigin::root(), 9, true));
        assert_ok!(OnboardingDistribution::register_evm_airdrop(
            RuntimeOrigin::signed(9),
            evm(1),
            1,
            2,
            100,
            10,
        ));
        let identity_id = pallet_identity_core::IdentityIdByEvmAddress::<Test>::get(evm(1)).unwrap();
        assert_ok!(OnboardingDistribution::rotate_root_for_evm(
            RuntimeOrigin::signed(9),
            evm(1),
            3,
        ));
        assert_eq!(pallet_identity_core::IdentityIdByEvmAddress::<Test>::get(evm(1)), Some(identity_id));
        assert_eq!(pallet_identity_core::Identities::<Test>::get(identity_id).unwrap().owner, 3);
        assert_eq!(Balances::free_balance(1), 100);
        assert_eq!(Balances::free_balance(3), 0);
    });
}
