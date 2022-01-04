use crate::{
	mock::*, pallet::{Error}
};
use frame_support::{assert_ok, assert_noop};
use super::*;

#[test]
fn should_build_genesis_kitties() {
	new_test_ext().execute_with(|| {
		// Check we have 2 kitties, as specified
		assert_eq!(SubstrateKitties::kitty_cnt(), 2);

		// Check owners own the correct amount of kitties
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		assert_eq!(kitties_owned_by_1.len(), 1);

		let kitties_owned_by_2 = SubstrateKitties::kitties_owned(2);
		assert_eq!(kitties_owned_by_2.len(), 1);

		// Check that kitties are owned correctly
		let kid1 = kitties_owned_by_1[0];
		let kitty1 = SubstrateKitties::kitties(kid1)
			.expect("Could have this kitty ID owned by acct 1");
		assert_eq!(kitty1.owner, 1);

		let kid2 = kitties_owned_by_2[0];
		let kitty2 = SubstrateKitties::kitties(kid2)
			.expect("Could have this kitty ID owned by acct 2");
		assert_eq!(kitty2.owner, 2);
	});
}
