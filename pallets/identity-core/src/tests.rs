use crate::mock::{
    new_test_ext, IdentityCore, RuntimeEvent, RuntimeOrigin, System, Test, Timestamp,
};
use frame::deps::frame_support::{assert_noop, assert_ok};
use vibly_primitives_common::ContentRef;
use vibly_primitives_identity::{
    IdentityStatus, KeyPurpose, TransportBindingStatus, TransportKind, CAP_MANAGE_PAYMENT,
    CAP_MANAGE_POINTERS, CAP_MANAGE_TRANSPORTS,
};

fn cid(
    bytes: &[u8],
) -> ContentRef<<Test as crate::Config>::MaxCidLen, <Test as crate::Config>::MaxUriLen> {
    ContentRef::Cid(bytes.to_vec().try_into().unwrap())
}

fn registered_identity() -> vibly_primitives_identity::IdentityId {
    match System::events().pop().unwrap().event {
        RuntimeEvent::IdentityCore(crate::Event::IdentityRegistered { identity_id, .. }) => {
            identity_id
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn register_and_manage_identity_works() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(10);
        assert_ok!(IdentityCore::register_identity(
            RuntimeOrigin::signed(1),
            Some(2),
            Some(cid(b"profile")),
            None,
            None,
            None
        ));
        let identity_id = registered_identity();
        let identity = crate::Identities::<Test>::get(identity_id).unwrap();
        assert_eq!(identity.owner, 1);
        assert_eq!(identity.recovery, Some(2));
        assert_eq!(identity.status, IdentityStatus::Active);
        assert_ok!(IdentityCore::set_active_profile(
            RuntimeOrigin::signed(1),
            identity_id,
            None
        ));
        assert_ok!(IdentityCore::add_key(
            RuntimeOrigin::signed(1),
            identity_id,
            3,
            KeyPurpose::Finance,
            CAP_MANAGE_PAYMENT | CAP_MANAGE_POINTERS | CAP_MANAGE_TRANSPORTS,
            None
        ));
    });
}

#[test]
fn transport_flow_works() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(20);
        assert_ok!(IdentityCore::register_identity(
            RuntimeOrigin::signed(1),
            Some(2),
            None,
            None,
            None,
            None
        ));
        let identity_id = registered_identity();
        assert_ok!(IdentityCore::bind_transport(
            RuntimeOrigin::signed(1),
            identity_id,
            TransportKind::Matrix,
            b"@alice:example.org".to_vec().try_into().unwrap(),
            None
        ));
        let binding_id = match System::events().pop().unwrap().event {
            RuntimeEvent::IdentityCore(crate::Event::TransportBound { binding_id, .. }) => {
                binding_id
            }
            _ => unreachable!(),
        };
        assert_eq!(
            crate::TransportBindings::<Test>::get(binding_id)
                .unwrap()
                .status,
            TransportBindingStatus::Pending
        );
        assert_ok!(IdentityCore::verify_transport(
            RuntimeOrigin::signed(2),
            identity_id,
            binding_id,
            Some(cid(b"proof"))
        ));
        assert_eq!(
            crate::TransportBindings::<Test>::get(binding_id)
                .unwrap()
                .status,
            TransportBindingStatus::Verified
        );
        assert_ok!(IdentityCore::revoke_transport(
            RuntimeOrigin::signed(1),
            identity_id,
            binding_id
        ));
        assert_eq!(
            crate::TransportBindings::<Test>::get(binding_id)
                .unwrap()
                .status,
            TransportBindingStatus::Revoked
        );
    });
}

#[test]
fn frozen_identity_rejects_pointer_updates() {
    new_test_ext().execute_with(|| {
        assert_ok!(IdentityCore::register_identity(
            RuntimeOrigin::signed(1),
            Some(2),
            None,
            None,
            None,
            None
        ));
        let identity_id = registered_identity();
        assert_ok!(IdentityCore::freeze_identity(
            RuntimeOrigin::signed(2),
            identity_id
        ));
        assert_noop!(
            IdentityCore::set_active_profile(
                RuntimeOrigin::signed(1),
                identity_id,
                Some(cid(b"blocked"))
            ),
            crate::Error::<Test>::InvalidState
        );
        assert_ok!(IdentityCore::unfreeze_identity(
            RuntimeOrigin::signed(1),
            identity_id
        ));
        assert_ok!(IdentityCore::disable_identity(
            RuntimeOrigin::signed(2),
            identity_id
        ));
        assert_noop!(
            IdentityCore::freeze_identity(RuntimeOrigin::signed(1), identity_id),
            crate::Error::<Test>::AlreadyDisabled
        );
    });
}

#[test]
fn rotate_owner_and_revoke_key_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(IdentityCore::register_identity(
            RuntimeOrigin::signed(1),
            Some(2),
            None,
            None,
            None,
            None
        ));
        let identity_id = registered_identity();
        assert_ok!(IdentityCore::add_key(
            RuntimeOrigin::signed(1),
            identity_id,
            4,
            KeyPurpose::Admin,
            CAP_MANAGE_POINTERS,
            None
        ));
        let key_id = match System::events().pop().unwrap().event {
            RuntimeEvent::IdentityCore(crate::Event::IdentityKeyAdded { key_id, .. }) => key_id,
            _ => unreachable!(),
        };
        assert_ok!(IdentityCore::revoke_key(
            RuntimeOrigin::signed(1),
            identity_id,
            key_id
        ));
        assert_ok!(IdentityCore::rotate_owner_key(
            RuntimeOrigin::signed(2),
            identity_id,
            5
        ));
        assert_eq!(
            crate::Identities::<Test>::get(identity_id).unwrap().owner,
            5
        );
    });
}
