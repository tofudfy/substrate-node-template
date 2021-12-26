use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use super::*;

#[test]
fn create_claim_test() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1.
        let proof = vec![0, 1];
		assert_ok!(PoeModule::create_claim(Origin::signed(1), proof.clone()));
		
        // Read pallet storage and assert an expected result.
		assert_eq!(
            Proofs::<Test>::get(&proof),
            (1, frame_system::Pallet::<Test>::block_number())
        );
	});
}

#[test]
fn create_claim_exceed_length_limit() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1 that exceeds 32 bits.
        let proof_exceed = vec![0, 1, 1, 0, 1, 0];
		assert_noop!(
            PoeModule::create_claim(Origin::signed(1), proof_exceed.clone()),
            Error::<Test>::ExceedProofLengthLimit
        );
	});
}

#[test]
fn create_claim_fail_when_claim_exist() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1.
        let proof = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
		
		assert_noop!(
            PoeModule::create_claim(Origin::signed(1), proof.clone()),
            Error::<Test>::ProofAlreadyClaimed
        );
	});
}

#[test]
fn revoke_claim_test() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1.
        let proof = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
	
        assert_ok!(PoeModule::revoke_claim(Origin::signed(1), proof.clone()));

		// assert_eq!(Proofs::<Test>::get(&proof), None);
	});
}

#[test]
fn revoke_claim_not_exist() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1.
        let proof = vec![0, 1];	
        
        assert_noop!(
            PoeModule::revoke_claim(Origin::signed(1), proof.clone()),
            Error::<Test>::NoSuchProof
        );
	});
}

#[test]
fn revoke_claim_not_owner() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1.
        let proof = vec![0, 1];	
        let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
        
        //
        assert_noop!(
            PoeModule::revoke_claim(Origin::signed(2), proof.clone()),
            Error::<Test>::NotProofOwner
        );
	});
}

#[test]
fn transfer_claim_test() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1.
        let proof = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());

        let receiver = 2;
        assert_ok!(PoeModule::transfer_claim(Origin::signed(1), proof.clone(), receiver));
		
        let (owner, _) = Proofs::<Test>::get(&proof);
		assert_eq!(owner, receiver);
	});
}

#[test]
fn transfer_claim_not_exist() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1.
        let proof = vec![0, 1];
        let proof2 = vec![0, 2];
		let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
		
        let receiver = 2;
        // Read pallet storage and assert an expected result.
		assert_noop!(
            PoeModule::transfer_claim(Origin::signed(1), proof2.clone(), receiver),
            Error::<Test>::NoSuchProof
        );
	});
}

#[test]
fn transfer_claim_not_owner() {
    new_test_ext().execute_with(|| {
		// Dispatch a claim extrinsic from account 1.
        let proof = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
		
        let receiver = 2;
        // Read pallet storage and assert an expected result.
		assert_noop!(
            PoeModule::transfer_claim(Origin::signed(3), proof.clone(), receiver),
            Error::<Test>::NotProofOwner
        );
	});
}