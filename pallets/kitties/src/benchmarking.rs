//! Benchmark-demo pallet kitties.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as SubstrateKitties;

use frame_benchmarking::{benchmarks, account};
use frame_system::RawOrigin;
use sp_std::prelude::*;

benchmarks!{
	create_kitty {
		let caller: T::AccountId = account("caller", 0, 0);
		let kitties_owned = SubstrateKitties::<T>::kitties_owned(&caller);
	}: _(RawOrigin::Signed(caller))
	verify {
		let caller: T::AccountId = account("caller", 0, 0);
		let kitties_owned_now = SubstrateKitties::<T>::kitties_owned(caller);
		assert_eq!(kitties_owned.len() + 1, kitties_owned_now.len());
	}

	/*
	set_price {
		let caller = account("caller", 0, 0);
		// todo: why Error when use bid_price instead of variable s?
		let s in 0 .. 100;

		Pallet::<T>::create_kitty(RawOrigin::Signed(caller));
		let kitties_owned = Pallet::<T>::kitties_owned(caller);
	}: _ (RawOrigin::Signed(caller), kitties_owned[0], Some(s.into()))
	verify {
		// query the metadata of the kitty
		let kitty = Pallet::<T>::kitties(kitties_owned[0])
			.expect("Could have this kitty ID owned by caller");
		assert_eq!(kitty.price, Some(s.into()));
	}*/

	impl_benchmark_test_suite!(SubstrateKitties, crate::mock::new_test_ext(), crate::mock::Test);
}

// todo: Error cannot find function 'test_benchmark_create_kitty' in this scope
/*
#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::assert_ok;

	#[test]
	fn test_benchmarks() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_create_kitty::<Test>());
			assert_ok!(test_benchmark_set_price::<Test>());
		});
	}
}
 */
