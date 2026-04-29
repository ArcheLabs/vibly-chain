use crate::{
    mock::{new_test_ext, RuntimeOrigin, System, Test, ViblyEmergency},
    pallet::{EmergencyScope, EmergencyStatus, Error, Event},
    LastPauseRecord, StatusByScope,
};
use frame::deps::frame_support::{assert_noop, assert_ok};

// ── pause ─────────────────────────────────────────────────────────────────────

#[test]
fn any_guardian_member_can_pause_active_scope() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(1);
        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(1), scope.clone(), None));
        assert_eq!(StatusByScope::<Test>::get(&scope), EmergencyStatus::Paused);
        assert!(LastPauseRecord::<Test>::contains_key(&scope));
        System::assert_last_event(
            Event::Paused { scope, by: 1, reason_hash: None }.into(),
        );
    });
}

#[test]
fn pause_overwrites_existing_paused_record() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(2);
        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(1), scope.clone(), None));
        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(2), scope.clone(), None));
        // Still paused, record updated to account 2.
        assert_eq!(StatusByScope::<Test>::get(&scope), EmergencyStatus::Paused);
        let rec = LastPauseRecord::<Test>::get(&scope).unwrap();
        assert_eq!(rec.by, 2u64);
    });
}

#[test]
fn pause_cancelled_scope_is_rejected() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(3);
        assert_ok!(ViblyEmergency::cancel(RuntimeOrigin::root(), scope.clone(), None));
        assert_noop!(
            ViblyEmergency::pause(RuntimeOrigin::signed(1), scope, None),
            Error::<Test>::AlreadyCancelled
        );
    });
}

// ── resume ────────────────────────────────────────────────────────────────────

#[test]
fn collective_can_resume_paused_scope() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(10);
        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(1), scope.clone(), None));
        assert_ok!(ViblyEmergency::resume(RuntimeOrigin::root(), scope.clone(), None));
        // Absent entry ≡ Active.
        assert_eq!(StatusByScope::<Test>::get(&scope), EmergencyStatus::Active);
        assert!(!LastPauseRecord::<Test>::contains_key(&scope));
        System::assert_last_event(
            Event::Resumed { scope, reason_hash: None }.into(),
        );
    });
}

#[test]
fn resume_active_scope_is_rejected() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(11);
        assert_noop!(
            ViblyEmergency::resume(RuntimeOrigin::root(), scope, None),
            Error::<Test>::NotPaused
        );
    });
}

#[test]
fn resume_cancelled_scope_is_rejected() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(12);
        assert_ok!(ViblyEmergency::cancel(RuntimeOrigin::root(), scope.clone(), None));
        assert_noop!(
            ViblyEmergency::resume(RuntimeOrigin::root(), scope, None),
            Error::<Test>::AlreadyCancelled
        );
    });
}

// ── cancel ────────────────────────────────────────────────────────────────────

#[test]
fn collective_can_cancel_active_scope() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(20);
        assert_ok!(ViblyEmergency::cancel(RuntimeOrigin::root(), scope.clone(), None));
        assert_eq!(StatusByScope::<Test>::get(&scope), EmergencyStatus::Cancelled);
        System::assert_last_event(
            Event::Cancelled { scope, reason_hash: None }.into(),
        );
    });
}

#[test]
fn collective_can_cancel_paused_scope() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(21);
        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(1), scope.clone(), None));
        assert_ok!(ViblyEmergency::cancel(RuntimeOrigin::root(), scope.clone(), None));
        assert_eq!(StatusByScope::<Test>::get(&scope), EmergencyStatus::Cancelled);
    });
}

#[test]
fn cancel_already_cancelled_scope_is_rejected() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(22);
        assert_ok!(ViblyEmergency::cancel(RuntimeOrigin::root(), scope.clone(), None));
        assert_noop!(
            ViblyEmergency::cancel(RuntimeOrigin::root(), scope, None),
            Error::<Test>::AlreadyCancelled
        );
    });
}

// ── helper methods ────────────────────────────────────────────────────────────

#[test]
fn ensure_active_returns_ok_when_active() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(30);
        assert_ok!(ViblyEmergency::ensure_active(&scope));
    });
}

#[test]
fn ensure_active_returns_err_when_paused() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(31);
        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(1), scope.clone(), None));
        assert!(ViblyEmergency::ensure_active(&scope).is_err());
        assert!(ViblyEmergency::is_paused(&scope));
    });
}

#[test]
fn ensure_active_returns_err_when_cancelled() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Proposal(32);
        assert_ok!(ViblyEmergency::cancel(RuntimeOrigin::root(), scope.clone(), None));
        assert!(ViblyEmergency::ensure_active(&scope).is_err());
        assert!(ViblyEmergency::is_cancelled(&scope));
    });
}

// ── different scope variants ──────────────────────────────────────────────────

#[test]
fn global_scope_can_be_paused_and_cancelled() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::Global;
        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(1), scope.clone(), None));
        assert_ok!(ViblyEmergency::cancel(RuntimeOrigin::root(), scope.clone(), None));
        assert_eq!(StatusByScope::<Test>::get(&scope), EmergencyStatus::Cancelled);
    });
}

#[test]
fn reward_batch_scope_works() {
    new_test_ext().execute_with(|| {
        let scope = EmergencyScope::RewardBatch(99);
        assert_ok!(ViblyEmergency::pause(RuntimeOrigin::signed(2), scope.clone(), None));
        assert!(ViblyEmergency::is_paused(&scope));
        assert_ok!(ViblyEmergency::resume(RuntimeOrigin::root(), scope.clone(), None));
        assert!(!ViblyEmergency::is_paused(&scope));
    });
}
