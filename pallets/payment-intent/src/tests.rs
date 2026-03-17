use crate::mock::{
    new_test_ext, IdentityCore, PaymentIntent, RuntimeEvent, RuntimeOrigin, System, Test, Timestamp,
};
use frame::deps::frame_support::{assert_noop, assert_ok};
use vibly_primitives_payment::{PaymentAction, PaymentIntentStatus, SettlementMode};

fn identity_for(who: u64) -> vibly_primitives_identity::IdentityId {
    assert_ok!(IdentityCore::register_identity(
        RuntimeOrigin::signed(who),
        None,
        None,
        None,
        None,
        None
    ));
    match System::events().pop().unwrap().event {
        RuntimeEvent::IdentityCore(pallet_identity_core::Event::IdentityRegistered {
            identity_id,
            ..
        }) => identity_id,
        other => panic!("unexpected event: {other:?}"),
    }
}

fn action() -> PaymentAction<
    <Test as crate::Config>::MaxNamespaceLen,
    <Test as crate::Config>::MaxCidLen,
    <Test as crate::Config>::MaxUriLen,
> {
    PaymentAction {
        namespace: b"service".to_vec().try_into().unwrap(),
        action_code: 1,
        payload_ref: None,
    }
}

#[test]
fn create_and_direct_fund_works() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(10);
        let payer = identity_for(1);
        let payee = identity_for(2);
        let intent_id = sp_core::H256::repeat_byte(1);
        assert_ok!(PaymentIntent::create_payment_intent(
            RuntimeOrigin::signed(1),
            intent_id,
            payer,
            payee,
            0,
            100,
            action(),
            None,
            SettlementMode::Direct,
            None
        ));
        assert_ok!(PaymentIntent::fund_payment_intent(
            RuntimeOrigin::signed(1),
            intent_id
        ));
        assert_eq!(
            crate::PaymentIntents::<Test>::get(intent_id)
                .unwrap()
                .status,
            PaymentIntentStatus::Claimed
        );
    });
}

#[test]
fn hold_claim_and_refund_state_machine_works() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(20);
        let payer = identity_for(1);
        let payee = identity_for(2);
        let hold_intent = sp_core::H256::repeat_byte(2);
        assert_ok!(PaymentIntent::create_payment_intent(
            RuntimeOrigin::signed(1),
            hold_intent,
            payer,
            payee,
            0,
            150,
            action(),
            None,
            SettlementMode::Hold,
            None
        ));
        assert_ok!(PaymentIntent::fund_payment_intent(
            RuntimeOrigin::signed(1),
            hold_intent
        ));
        assert_eq!(
            crate::PaymentIntents::<Test>::get(hold_intent)
                .unwrap()
                .status,
            PaymentIntentStatus::Funded
        );
        assert_ok!(PaymentIntent::claim_payment_intent(
            RuntimeOrigin::signed(2),
            hold_intent,
            None
        ));
        assert_eq!(
            crate::PaymentIntents::<Test>::get(hold_intent)
                .unwrap()
                .status,
            PaymentIntentStatus::Claimed
        );

        let refund_intent = sp_core::H256::repeat_byte(3);
        assert_ok!(PaymentIntent::create_payment_intent(
            RuntimeOrigin::signed(1),
            refund_intent,
            payer,
            payee,
            0,
            200,
            action(),
            None,
            SettlementMode::Hold,
            None
        ));
        assert_ok!(PaymentIntent::fund_payment_intent(
            RuntimeOrigin::signed(1),
            refund_intent
        ));
        assert_ok!(PaymentIntent::refund_payment_intent(
            RuntimeOrigin::signed(1),
            refund_intent,
            None
        ));
        assert_eq!(
            crate::PaymentIntents::<Test>::get(refund_intent)
                .unwrap()
                .status,
            PaymentIntentStatus::Refunded
        );
    });
}

#[test]
fn cancel_and_expire_only_work_from_requested() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(30);
        let payer = identity_for(1);
        let payee = identity_for(2);
        let cancel_intent = sp_core::H256::repeat_byte(4);
        assert_ok!(PaymentIntent::create_payment_intent(
            RuntimeOrigin::signed(1),
            cancel_intent,
            payer,
            payee,
            0,
            50,
            action(),
            None,
            SettlementMode::Hold,
            None
        ));
        assert_ok!(PaymentIntent::cancel_payment_intent(
            RuntimeOrigin::signed(1),
            cancel_intent
        ));
        assert_eq!(
            crate::PaymentIntents::<Test>::get(cancel_intent)
                .unwrap()
                .status,
            PaymentIntentStatus::Cancelled
        );

        let expired_intent = sp_core::H256::repeat_byte(5);
        assert_ok!(PaymentIntent::create_payment_intent(
            RuntimeOrigin::signed(1),
            expired_intent,
            payer,
            payee,
            0,
            75,
            action(),
            None,
            SettlementMode::Hold,
            Some(40)
        ));
        assert_noop!(
            PaymentIntent::expire_payment_intent(RuntimeOrigin::signed(3), expired_intent),
            crate::Error::<Test>::NotYetExpired
        );
        Timestamp::set_timestamp(41);
        assert_ok!(PaymentIntent::expire_payment_intent(
            RuntimeOrigin::signed(3),
            expired_intent
        ));
        assert_eq!(
            crate::PaymentIntents::<Test>::get(expired_intent)
                .unwrap()
                .status,
            PaymentIntentStatus::Expired
        );
    });
}

#[test]
fn invalid_asset_and_terminal_reentry_fail() {
    new_test_ext().execute_with(|| {
        let payer = identity_for(1);
        let payee = identity_for(2);
        let intent_id = sp_core::H256::repeat_byte(6);
        assert_noop!(
            PaymentIntent::create_payment_intent(
                RuntimeOrigin::signed(1),
                intent_id,
                payer,
                payee,
                1,
                10,
                action(),
                None,
                SettlementMode::Hold,
                None
            ),
            crate::Error::<Test>::InvalidAsset
        );
        assert_ok!(PaymentIntent::create_payment_intent(
            RuntimeOrigin::signed(1),
            intent_id,
            payer,
            payee,
            0,
            10,
            action(),
            None,
            SettlementMode::Hold,
            None
        ));
        assert_ok!(PaymentIntent::fund_payment_intent(
            RuntimeOrigin::signed(1),
            intent_id
        ));
        assert_ok!(PaymentIntent::claim_payment_intent(
            RuntimeOrigin::signed(2),
            intent_id,
            None
        ));
        assert_noop!(
            PaymentIntent::refund_payment_intent(RuntimeOrigin::signed(1), intent_id, None),
            crate::Error::<Test>::InvalidState
        );
    });
}
