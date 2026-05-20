use crate::{
    mock::{identity_id, network_id, new_test_ext, Balances, RuntimeOrigin, Test, VibClaim, RESERVE},
    BoundedProofOf, ClaimRoot, ClaimedAmount, Error,
};
use frame::deps::frame_support::{assert_noop, assert_ok};

fn empty_proof() -> BoundedProofOf<Test> {
    Default::default()
}

fn root_for(account: u64, cumulative: u128) -> [u8; 32] {
    VibClaim::hash_leaf(&account, &identity_id(), cumulative)
}

#[test]
fn root_origin_and_version_are_enforced() {
    new_test_ext().execute_with(|| {
        let root = root_for(1, 100);
        assert_noop!(
            VibClaim::set_claim_root(RuntimeOrigin::signed(1), network_id(), 1, root, 100, [1; 32]),
            Error::<Test>::UnauthorizedRootUpdate
        );
        assert_ok!(VibClaim::set_claim_root(RuntimeOrigin::root(), network_id(), 1, root, 100, [1; 32]));
        assert_eq!(ClaimRoot::<Test>::get().unwrap().root_version, 1);
        assert_noop!(
            VibClaim::set_claim_root(RuntimeOrigin::root(), network_id(), 1, root, 100, [1; 32]),
            Error::<Test>::InvalidRootVersion
        );
    });
}

#[test]
fn valid_claim_transfers_delta_and_rejects_duplicate() {
    new_test_ext().execute_with(|| {
        let root = root_for(1, 100);
        assert_ok!(VibClaim::set_claim_root(RuntimeOrigin::root(), network_id(), 1, root, 100, [1; 32]));
        assert_ok!(VibClaim::claim(
            RuntimeOrigin::signed(1),
            network_id(),
            1,
            identity_id(),
            100,
            empty_proof(),
        ));
        assert_eq!(ClaimedAmount::<Test>::get(1), 100);
        assert_eq!(Balances::free_balance(1), 110);
        assert_eq!(Balances::free_balance(RESERVE), 999_900);
        assert_noop!(
            VibClaim::claim(RuntimeOrigin::signed(1), network_id(), 1, identity_id(), 100, empty_proof()),
            Error::<Test>::NothingToClaim
        );
    });
}

#[test]
fn new_root_claims_only_delta() {
    new_test_ext().execute_with(|| {
        assert_ok!(VibClaim::set_claim_root(RuntimeOrigin::root(), network_id(), 1, root_for(1, 100), 100, [1; 32]));
        assert_ok!(VibClaim::claim(RuntimeOrigin::signed(1), network_id(), 1, identity_id(), 100, empty_proof()));
        assert_ok!(VibClaim::set_claim_root(RuntimeOrigin::root(), network_id(), 2, root_for(1, 150), 150, [2; 32]));
        assert_ok!(VibClaim::claim(RuntimeOrigin::signed(1), network_id(), 2, identity_id(), 150, empty_proof()));
        assert_eq!(ClaimedAmount::<Test>::get(1), 150);
        assert_eq!(Balances::free_balance(1), 160);
        assert_eq!(Balances::free_balance(RESERVE), 999_850);
    });
}

#[test]
fn invalid_network_or_proof_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(VibClaim::set_claim_root(RuntimeOrigin::root(), network_id(), 1, [9; 32], 100, [1; 32]));
        assert_noop!(
            VibClaim::claim(RuntimeOrigin::signed(1), network_id(), 1, identity_id(), 100, empty_proof()),
            Error::<Test>::InvalidMerkleProof
        );
        let other_network = b"other".to_vec().try_into().unwrap();
        assert_noop!(
            VibClaim::claim(RuntimeOrigin::signed(1), other_network, 1, identity_id(), 100, empty_proof()),
            Error::<Test>::InvalidNetworkId
        );
    });
}

#[test]
fn paused_claims_are_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(VibClaim::set_claim_root(RuntimeOrigin::root(), network_id(), 1, root_for(1, 100), 100, [1; 32]));
        assert_ok!(VibClaim::set_claim_paused(RuntimeOrigin::root(), true));
        assert_noop!(
            VibClaim::claim(RuntimeOrigin::signed(1), network_id(), 1, identity_id(), 100, empty_proof()),
            Error::<Test>::ClaimPaused
        );
    });
}
