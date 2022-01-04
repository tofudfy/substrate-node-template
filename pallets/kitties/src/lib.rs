#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		sp_runtime::traits::Hash,
		dispatch::DispatchResult, // DispatchResultWithPostInfo
		traits::{Currency, ReservableCurrency, ExistenceRequirement, Randomness},
		pallet_prelude::*,
	};

	// helper traits
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use frame_support::{
		sp_io::hashing::blake2_128,
		transactional
	};

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	// Write a Struct for holding Kitty information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
	}

	// Enum declaration for Gender.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Gender {
		Male,
		Female,
	}


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types it depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the Kitties pallet.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		// Specify the type for Randomness we want to specify for runtime.
		// type KittyRandomness: Randomness<Self::KittyIndex, Self::BlockNumber>;
		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		// type KittyIndex = Self::Hash;
		// type KittyHashing = Self::Hashing;
		type KittyIndex: Parameter + Member + MaybeSerializeDeserialize + Copy;
		type KittyHashing: Hash<Output = Self::KittyIndex> + TypeInfo;

		// Add MaxKittyOwned constant
		#[pallet::constant]
		type MaxKittyOwned: Get<u32>;

		#[pallet::constant]
		type MinKittyMintingPrice: Get<u32>;
	}

	// Storage items.
	#[pallet::storage]
	#[pallet::getter(fn kitty_cnt)]
	/// Keeps track of the number of Kitties in existence.
	pub(super) type KittyCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	/// Unique assets （Hash Map）
	pub(super) type Kitties<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::KittyIndex,
		Kitty<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn kitties_owned)]
	/// Keeps track of what accounts own what Kitty.
	pub(super) type KittiesOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::KittyIndex, T::MaxKittyOwned>,
		ValueQuery,
	>;


	// Our pallet's genesis configuration.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub kitties: Vec<(T::AccountId, [u8; 16], Gender)>,
	}

	// Required to implement default for GenesisConfig.
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { kitties: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// When building a kitty from genesis config, we require the dna and gender to be supplied.
			for (acct, dna, gender) in &self.kitties {
				let _ = <Pallet<T>>::mint(acct, Some(dna.clone()), Some(gender.clone()));
			}
		}
	}

	// Errors.
	#[pallet::error]
	pub enum Error<T> {
		/// Handles arithmetic overflow when incrementing the Kitty counter.
		KittyCntOverflow,
		/// An account cannot own more Kitties than `MaxKittyCount`.
		ExceedMaxKittyOwned,
		/// Buyer cannot be the owner.
		BuyerIsKittyOwner,
		/// Cannot transfer a kitty to its owner.
		TransferToSelf,
		/// Handles checking whether the Kitty exists.
		KittyNotExist,
		/// Handles checking that the Kitty is owned by the account transferring, buying or setting a price for it.
		NotKittyOwner,
		/// Ensures the Kitty is for sale.
		KittyNotForSale,
		/// Ensures that the buying price is greater than the asking price.
		KittyBidPriceTooLow,
		/// Ensures that an account has enough funds to purchase a Kitty.
		NotEnoughBalance,
		/// Ensure the genders of the parents are different [Male, Female]
		SameParentGender,
		/// Ensure the parent kitties are not the same
		SameParentKittyId,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>{
		/// A function succeeded. [time, day]
		// Success(T::Time, T::Day),
		/// A new Kitty was sucessfully created. \[sender, kitty_id\]
		Created(T::AccountId, T::KittyIndex),
		/// Kitty price was sucessfully set. \[sender, kitty_id, new_price\]
		PriceSet(T::AccountId, T::KittyIndex, Option<BalanceOf<T>>),
		/// A Kitty was sucessfully transferred. \[from, to, kitty_id\]
		Transferred(T::AccountId, T::AccountId, T::KittyIndex),
		/// A Kitty was sucessfully bought. \[buyer, seller, kitty_id, bid_price\]
		Bought(T::AccountId, T::AccountId, T::KittyIndex, BalanceOf<T>),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		// create_kitty: create a kitty and update the Storage
		#[pallet::weight(1_000)]
		pub fn create_kitty(
			origin: OriginFor<T>
		) -> DispatchResult {
			// checks the origin is signed
			let sender = ensure_signed(origin)?;

			// Check the buyer has enough minting balance
			let min_balance = <BalanceOf<T>>::from(T::MinKittyMintingPrice::get());
			T::Currency::reserve(&sender, min_balance)?;

			// calls a private mint() function
			let kitty_id = Self::mint(&sender, None, None)?;

			// emits log and event that create a kitty successfully
			log::info!("A kitty is born with ID: {:?}.", kitty_id);
			Self::deposit_event(Event::Created(sender, kitty_id));

			Ok(())
		}

		// set_price: set the price of the kitty by its owner
		#[pallet::weight(1_000)]
		pub fn set_price(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			new_price: Option<BalanceOf<T>>
		) -> DispatchResult {
			// checks the origin is signed
			let sender = ensure_signed(origin)?;

			// checks the ownership of the specific kitty
			ensure!(Self::is_kitty_owner(&kitty_id, &sender)?, <Error<T>>::NotKittyOwner);

			// Get the kitty object and modify the price
			let mut kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;
			kitty.price = new_price.clone();
			<Kitties<T>>::insert(&kitty_id, kitty);

			// Deposit a "PriceSet" event.
			Self::deposit_event(Event::PriceSet(sender, kitty_id, new_price));

			Ok(())
		}

		// transfer: transfer the ownership of a kitty
		#[pallet::weight(1_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			receiver: T::AccountId
		) -> DispatchResult {
			// checks the origin is signed
			let sender = ensure_signed(origin)?;

			// checks the ownership of the specific kitty
			ensure!(Self::is_kitty_owner(&kitty_id, &sender)?, <Error<T>>::NotKittyOwner);

			// Verify the kitty is not transferring back to its owner.
			ensure!(sender != receiver, <Error<T>>::TransferToSelf);

			// Verify the recipient has the capacity to receive one more kitty
			let to_owned = <KittiesOwned<T>>::get(&receiver);
			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			// calls a private transfer_kitty_to() function
			Self::transfer_kitty_to(&kitty_id, &receiver)?;

			// Deposit a "Transferred" event.
			Self::deposit_event(Event::Transferred(sender, receiver, kitty_id));

			Ok(())
		}

		// buy_kitty: buy a on-selling kitty
		#[pallet::weight(1_000)]
		pub fn buy_kitty(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			bid_price: BalanceOf<T>
		) -> DispatchResult {
			// checks the origin is signed
			let buyer = ensure_signed(origin)?;

			// Get the kitty object
			let kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			// Check the kitty is for sale and the kitty ask price <= bid_price
			if let Some(ask_price) = kitty.price {
				ensure!(ask_price <= bid_price, <Error<T>>::KittyBidPriceTooLow);
			} else {
				Err(<Error<T>>::KittyNotForSale)?;
			}

			// checks the ownership of the kitty is not from buyer himself
			let seller = kitty.owner;
			ensure!(buyer != seller, <Error<T>>::BuyerIsKittyOwner);

			// Check the buyer has enough free balance
			ensure!(T::Currency::free_balance(&buyer) >= bid_price, <Error<T>>::NotEnoughBalance);

			// Verify the buyer has the capacity to receive one more kitty
			let to_owned = <KittiesOwned<T>>::get(&buyer);
			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			// Transfer the amount from buyer to seller
			T::Currency::transfer(&buyer, &seller, bid_price, ExistenceRequirement::KeepAlive)?;

			// calls a private transfer_kitty_to() function
			Self::transfer_kitty_to(&kitty_id, &buyer)?;

			// Deposit a "Bought" event.
			Self::deposit_event(Event::Bought(buyer, seller, kitty_id, bid_price));

			Ok(())
		}

		// breed_kitty: breed a baby kitty by two kitties owned with different gender
		#[pallet::weight(1_000)]
		pub fn breed_kitty(
			origin: OriginFor<T>,
			kitty_id1: T::KittyIndex,
			kitty_id2: T::KittyIndex,
		) -> DispatchResult {
			// checks the origin is signed
			let owner = ensure_signed(origin)?;

			// The kitty id cannot be the same
			ensure!(kitty_id1 != kitty_id2, <Error<T>>::SameParentKittyId);

			// Get the kitty object by kitty_id1
			let parent1 = Self::kitties(&kitty_id1).ok_or(<Error<T>>::KittyNotExist)?;
			let parent2 = Self::kitties(&kitty_id2).ok_or(<Error<T>>::KittyNotExist)?;

			// checks the ownership of the kitty
			let owner_kitty1 = parent1.owner;
			ensure!(owner != owner_kitty1, <Error<T>>::NotKittyOwner);

			let owner_kitty2 = parent2.owner;
			ensure!(owner != owner_kitty2, <Error<T>>::NotKittyOwner);

			// check the genders of the parents
			let gender_kitty1 = parent1.gender;
			let gender_kitty2 = parent2.gender;
			ensure!(gender_kitty1 != gender_kitty2, <Error<T>>::SameParentGender);

			let new_dna = Self::breed_dna(parent1.dna, parent2.dna)?;
			let new_kitty_id = Self::mint(&owner, Some(new_dna), None)?;

			// Deposit a "Created" event.
			Self::deposit_event(Event::Created(owner, new_kitty_id));

			Ok(())
		}
	}

	/// helper functions for dispatchable functions
	impl<T: Config> Pallet<T> {
		// Function to randomly generate gender for Kitty struct
		fn gen_gender() -> Gender {
			let random = T::KittyRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				_ => Gender::Female,
			}
		}

		// Funtion to randomly generate DNA for kitty struct
		fn gen_dna() -> [u8; 16] {
			let payload = (
				T::KittyRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::block_number(),
			);
			payload.using_encoded(blake2_128)
		}

		// Helper to mint a Kitty.
		pub fn mint(
			owner: &T::AccountId,
			dna: Option<[u8; 16]>,
			gender: Option<Gender>,
		) -> Result<T::KittyIndex, Error<T>> {
			let kitty = Kitty::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				gender: gender.unwrap_or_else(Self::gen_gender),
				owner: owner.clone(),
			};

			// Gen an id of the kitty
			let kitty_id = T::KittyHashing::hash_of(&kitty);

			// Performs this operation first as it may fail
			let new_cnt = Self::kitty_cnt().checked_add(1)
				.ok_or(<Error<T>>::KittyCntOverflow)?;

			// Performs this operation first because as it may fail
			<KittiesOwned<T>>::try_mutate(&owner, |kitty_vec| {
				kitty_vec.try_push(kitty_id)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			<Kitties<T>>::insert(kitty_id, kitty);
			<KittyCnt<T>>::put(new_cnt);
			Ok(kitty_id)
		}

		// check if the sender is the owner of the kitty
		pub fn is_kitty_owner(
			kitty_id: &T::KittyIndex,
			acct: &T::AccountId
		) -> Result<bool, Error<T>> {
			match Self::kitties(kitty_id) {
				Some(kitty) => Ok(kitty.owner == *acct),
				None => Err(<Error<T>>::KittyNotExist)
			}
		}

		// Helper to generate a Extrinsics to transfer the ownership of a kitty
		#[transactional]
		pub fn transfer_kitty_to(
			kitty_id: &T::KittyIndex,
			recv: &T::AccountId
		) -> Result<(), Error<T>> {
			// get the kitty object by kitty_id
			let mut kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;
			let prev_owner = kitty.owner;

			// Remove `kitty_id` from the KittyOwned vector of `prev_kitty_owner`
			<KittiesOwned<T>>::try_mutate(&prev_owner, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == *kitty_id) {
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			}).map_err(|_| <Error<T>>::KittyNotExist)?;

			// Transfer the ownership of the kitty
			kitty.owner = recv.clone();
			// Reset the ask price so the kitty is not for sale until `set_price()` is called
			// by the current owner.
			kitty.price = None;

			<Kitties<T>>::insert(&kitty_id, kitty);

			// update the ownership storage
			<KittiesOwned<T>>::try_mutate(&recv, |kitty_vec| {
				kitty_vec.try_push(*kitty_id)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			Ok(())
		}

		// generate a breed dna by mixing with dna1 and dna2
		pub fn breed_dna(
			dna1: [u8; 16],
			dna2: [u8; 16]
		) -> Result<[u8; 16], Error<T>> {

			let mut new_dna = Self::gen_dna();

			for i in 0..new_dna.len() {
				new_dna[i] = (new_dna[i] & dna1[i]) | (!new_dna[i] & dna2[i]);
			}

			Ok(new_dna)
		}
	}
}
