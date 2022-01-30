use crate::{
	mock::*, pallet::{Error}
};
use frame_support::{assert_ok, assert_noop};
use sp_core::H256;
// use super::*;

fn events() -> Vec<Event> {
	let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}

#[test]
fn should_build_genesis_kitties() {
	new_test_ext().execute_with(|| {
		// Check we have 2 kitties, as specified
		assert_eq!(SubstrateKitties::kitty_cnt(), 5);

		// Check owners own the correct amount of kitties
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		assert_eq!(kitties_owned_by_1.len(), 1);

		let kitties_owned_by_2 = SubstrateKitties::kitties_owned(2);
		assert_eq!(kitties_owned_by_2.len(), 1);

		let kitties_owned_by_3 = SubstrateKitties::kitties_owned(3);
		assert_eq!(kitties_owned_by_3.len(), 3);

		// Check that kitties are owned correctly
		let kid1 = kitties_owned_by_1[0];
		let kitty1 = SubstrateKitties::kitties(kid1)
			.expect("Could have this kitty ID owned by acct 1");
		assert_eq!(kitty1.owner, 1);

		let kid2 = kitties_owned_by_2[0];
		let kitty2 = SubstrateKitties::kitties(kid2)
			.expect("Could have this kitty ID owned by acct 2");
		assert_eq!(kitty2.owner, 2);

		let kid3 = kitties_owned_by_3[0];
		let kitty3 = SubstrateKitties::kitties(kid3)
			.expect("Could have this kitty ID owned by acct 3");
		assert_eq!(kitty3.owner, 3);

		let kid3 = kitties_owned_by_3[1];
		let kitty3 = SubstrateKitties::kitties(kid3)
			.expect("Could have this kitty ID owned by acct 3");
		assert_eq!(kitty3.owner, 3);

		let kid3 = kitties_owned_by_3[2];
		let kitty3 = SubstrateKitties::kitties(kid3)
			.expect("Could have this kitty ID owned by acct 3");
		assert_eq!(kitty3.owner, 3);
	});
}

#[test]
fn create_kitty_test() {
	new_test_ext().execute_with(|| {
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);

		// account 1 create a kitty
		assert_ok!(SubstrateKitties::create_kitty(Origin::signed(1)));

		// get the kitties created by account 1
		let kitties_owned_by_1_new = SubstrateKitties::kitties_owned(1);
		assert_eq!(kitties_owned_by_1_new.len(), kitties_owned_by_1.len() + 1);

		let events = events();
		assert_eq!(
			events[1],
			Event::SubstrateKitties(crate::Event::Created(1, kitties_owned_by_1_new[1]))
		);
	});
}

#[test]
fn create_kitty_not_enough_reserve_balance() {
	new_test_ext().execute_with(|| {
		// account 3 create a kitty with insufficient balance (owned 2, required 3)
		assert_noop!(
			SubstrateKitties::create_kitty(Origin::signed(3)),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
	});
}

/*
#[test]
fn create_kitty_overflow() {
	new_test_ext().execute_with(|| {

		let count = MaxKittyOwned::get();

		let mut i = 1;
		while i <= count {
			SubstrateKitties::create_kitty(Origin::signed(1));
			i += 1;
		}

		// account 1 create serveral kitties that exceed the storage limit
		assert_noop!(
			SubstrateKitties::create_kitty(Origin::signed(1)),
			Error::<Test>::ExceedMaxKittyOwned
		);
	});
}*/

#[test]
fn set_price_test() {
	new_test_ext().execute_with(|| {

		// account 1 set the price of his first kitty
		let bid_price = 100;
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitties_owned_by_1[0], Some(bid_price)));

		// query the metadata of the kitty
		let kitty = SubstrateKitties::kitties(kitties_owned_by_1[0])
			.expect("Could have this kitty ID owned by acct 1");
		assert_eq!(kitty.price, Some(bid_price));

		assert_eq!(
			events(),
			[
				Event::SubstrateKitties(crate::Event::PriceSet(1, kitties_owned_by_1[0], Some(bid_price)))
			]
		);
	});
}

#[test]
fn set_price_not_owner() {
	new_test_ext().execute_with(|| {
		let kitties_owned_by_2 = SubstrateKitties::kitties_owned(2);

		// account 1 set the price of a kitty of account 2
		assert_noop!(
			SubstrateKitties::set_price(Origin::signed(1), kitties_owned_by_2[0], Some(100)),
			Error::<Test>::NotKittyOwner
		);
	});
}


#[test]
fn set_price_kitty_not_exist() {
	new_test_ext().execute_with(|| {

		// account 1 set the price of a not exist kitty
		assert_noop!(
			SubstrateKitties::set_price(Origin::signed(1), H256::from_low_u64_be(2), Some(100)),
			Error::<Test>::KittyNotExist
		);

	});
}

#[test]
fn transfer_test() {
	new_test_ext().execute_with(|| {
		// account transfer the ownership of his first kitty to account2
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		let kitty_id_owned_by_1_old = kitties_owned_by_1[0];
		assert_ok!(SubstrateKitties::transfer(Origin::signed(1), kitties_owned_by_1[0], 2));

		// check the ID of the kitty transfered matched
		let kitties_owned_by_2 = SubstrateKitties::kitties_owned(2);
		assert_eq!(
			kitty_id_owned_by_1_old,
			kitties_owned_by_2[1]
		);

		assert_eq!(
			events(),
			[
				Event::SubstrateKitties(crate::Event::Transferred(1, 2, kitty_id_owned_by_1_old))
			]
		);
	});
}

#[test]
fn transfer_not_owner() {
	new_test_ext().execute_with(|| {
		let kitties_owned_by_2 = SubstrateKitties::kitties_owned(2);

		// account 1 transfer a kitty of account 2 to account 3
		assert_noop!(
			SubstrateKitties::transfer(Origin::signed(1), kitties_owned_by_2[0], 3),
			Error::<Test>::NotKittyOwner
		);
	});
}

#[test]
fn transfer_to_self() {
	new_test_ext().execute_with(|| {
		// account 1 transfer a kitty to himself
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		assert_noop!(
			SubstrateKitties::transfer(Origin::signed(1), kitties_owned_by_1[0], 1),
			Error::<Test>::TransferToSelf
		);
	});
}


#[test]
fn transfer_kitty_not_exist() {
	new_test_ext().execute_with(|| {
		// account 1 transfer a kitty not exist
		assert_noop!(
			SubstrateKitties::transfer(Origin::signed(1), H256::from_low_u64_be(2), 2),
			Error::<Test>::KittyNotExist
		);
	});
}

#[test]
fn buy_kitty_test() {
	new_test_ext().execute_with(|| {
		// account 1 set the price of his first kitty
		let sell_price = 8;
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		let kitty_index_onsell = kitties_owned_by_1[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_index_onsell, Some(sell_price)));

		// account 2 buy the kitty that selled by account 1
		let bid_price = 8;
		assert_ok!(SubstrateKitties::buy_kitty(Origin::signed(2), kitty_index_onsell, bid_price));

		// check the ID of the kitty bought matched and it is not on sell
		let kitties_owned_by_2 = SubstrateKitties::kitties_owned(2);
		let kitty_index_bought = kitties_owned_by_2[1];
		assert_eq!(kitty_index_onsell, kitty_index_bought);

		let kitty = SubstrateKitties::kitties(kitty_index_bought)
			.expect("Could have this kitty ID owned by acct 1");
		assert_eq!(kitty.price, None);

		let events = events();
		assert_eq!(
			events[2],
			Event::SubstrateKitties(crate::Event::Bought(1, 2, kitty_index_onsell, bid_price))
		);
	});
}


#[test]
fn buy_kitty_not_exit() {
	new_test_ext().execute_with(|| {
		// account 1 buy a kitty not exist
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(2), H256::from_low_u64_be(2), 8),
			Error::<Test>::KittyNotExist
		);
	});
}

#[test]
fn buy_kitty_bid_price_low() {
	new_test_ext().execute_with(|| {
		// account 1 set the price of his first kitty at 100
		let sell_price = 100;
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		let kitty_index_onsell = kitties_owned_by_1[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_index_onsell, Some(sell_price)));

		// account 2 bid at 8 less than 1oo
		let bid_price = 8;
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(2), kitty_index_onsell, bid_price),
			Error::<Test>::KittyBidPriceTooLow
		);
	});
}

#[test]
fn buy_kitty_not_for_sale() {
	new_test_ext().execute_with(|| {
		// get a kitty of account 1 which is not on sale
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		let kitty_index_not_sell = kitties_owned_by_1[0];

		// account 2 buy a kitty not for sell
		let bid_price = 8;
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(2), kitty_index_not_sell, bid_price),
			Error::<Test>::KittyNotForSale
		);
	});
}

/*
fn kitty_on_sale() -> SubstrateKitties::Config {
	// account 1 set the price of his first kitty at 8
	let sell_price = 8;
	let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
	let kitty_index_onsell = kitties_owned_by_1[0];
	assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_index_onsell, Some(sell_price)));
	kitty_index_onsell
}*/

#[test]
fn buy_kitty_of_own() {
	new_test_ext().execute_with(|| {
		// account 1 set the price of his first kitty at 8
		let sell_price = 8;
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		let kitty_index_onsell = kitties_owned_by_1[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_index_onsell, Some(sell_price)));

		// account 1 but a kitty of his own
		let bid_price = 8;
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(1), kitty_index_onsell, bid_price),
			Error::<Test>::BuyerIsKittyOwner
		);
	});
}

#[test]
fn buy_kitty_without_sufficient_balance() {
	new_test_ext().execute_with(|| {
		// account 1 set the price of his first kitty at 8
		let sell_price = 8;
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		let kitty_index_onsell = kitties_owned_by_1[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_index_onsell, Some(sell_price)));

		// account 2 buy the kitty at 12
		let bid_price = 12;
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(2), kitty_index_onsell, bid_price),
			Error::<Test>::NotEnoughBalance
		);
	});
}

#[test]
fn buy_kitty_not_alive() {
	new_test_ext().execute_with(|| {
		// account 1 set the price of his first kitty at 8
		let sell_price = 8;
		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		let kitty_index_onsell = kitties_owned_by_1[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_index_onsell, Some(sell_price)));

		// account 2 transfer a kitty
		let bid_price = 10;
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(2), kitty_index_onsell, bid_price),
			pallet_balances::Error::<Test>::KeepAlive
		);
	});
}

#[test]
fn breed_kitty_test() {
	new_test_ext().execute_with(|| {
		// account 3 breed a kitty
		let kitties_owned_by_3 = SubstrateKitties::kitties_owned(3);
		let parent_index_1 = kitties_owned_by_3[0]; // (3, *b"123456789012345e", Gender::Male)
		let parent_index_2 = kitties_owned_by_3[2]; // (3, *b"1234567890123466", Gender::Female)

		assert_ok!(SubstrateKitties::breed_kitty(Origin::signed(3), parent_index_1, parent_index_2));

		let kitties_owned_by_3 = SubstrateKitties::kitties_owned(3);
		let new_kitty_index = kitties_owned_by_3[3];
		assert_eq!(
			events(),
			[
				Event::SubstrateKitties(crate::Event::Created(3, new_kitty_index))
			]
		);
	});
}

#[test]
fn breed_kitty_from_same_kitty() {
	new_test_ext().execute_with(|| {
		let kitties_owned_by_3 = SubstrateKitties::kitties_owned(3);
		let parent_index_1 = kitties_owned_by_3[0]; // (3, *b"123456789012345e", Gender::Male)

		// account 3 breed a kitty by the same kitty
		assert_noop!(
			SubstrateKitties::breed_kitty(Origin::signed(3), parent_index_1, parent_index_1),
			Error::<Test>::SameParentKittyId
		);
	});
}

/*
#[test]
fn breed_kitty_not_exist() {
	new_test_ext().execute_with(|| {
		let kitties_owned_by_3 = SubstrateKitties::kitties_owned(3);
		let parent_index_1 = kitties_owned_by_3[0];

		// account 3 breed a kitty by the same kitty
		assert_noop!(
			SubstrateKitties::breed_kitty(Origin::signed(3), parent_index_1, parent_index_1),
			Error::<Test>::KittyNotExist
		);
	});
}*/

#[test]
fn breed_kitty_from_same_kitty_gender() {
	new_test_ext().execute_with(|| {
		let kitties_owned_by_3 = SubstrateKitties::kitties_owned(3);
		let parent_index_1 = kitties_owned_by_3[0];
		let parent_index_2 = kitties_owned_by_3[1]; // (3, *b"1234567890123462", Gender::Male)

		// account 3 breed a kitty by the same kitty
		assert_noop!(
			SubstrateKitties::breed_kitty(Origin::signed(3), parent_index_1, parent_index_2),
			Error::<Test>::SameParentGender
		);
	});
}
