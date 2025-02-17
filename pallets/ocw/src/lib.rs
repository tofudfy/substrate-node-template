#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod test;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	//! A demonstration of an offchain worker that sends onchain callbacks
	use core::{convert::TryInto, fmt};
	use parity_scale_codec::{Decode, Encode};
	use frame_support::pallet_prelude::*;
	use frame_system::{
		pallet_prelude::*,
		offchain::{
			AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
			SignedPayload, Signer, SigningTypes, SubmitTransaction,
		},
	};
	use sp_core::{crypto::KeyTypeId};
	use sp_arithmetic::per_things::Permill;
	use sp_runtime::{
		offchain as rt_offchain,
		traits::{
			BlockNumberProvider
		},
		offchain::{
			storage::StorageValueRef,
			storage_lock::{BlockAndTime, StorageLock},
		},
		transaction_validity::{
			InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
		},
		RuntimeDebug,
	};
	use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};

	use serde::{Deserialize, Deserializer};

	use serde::__private::ToString;
	use scale_info::prelude::string::String;

	/// Defines application identifier for crypto keys of this module.
	///
	/// Every module that deals with signatures needs to declare its unique identifier for
	/// its crypto keys.
	/// When an offchain worker is signing transactions it's going to request keys from type
	/// `KeyTypeId` via the keystore to sign the transaction.
	/// The keys can be inserted manually via RPC (see `author_insertKey`).
	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"demo");
	const NUM_VEC_LEN: usize = 10;
	/// The type to sign and send transactions.
	const UNSIGNED_TXS_PRIORITY: u64 = 100;

	// We are fetching information from the github public API about organization`substrate-developer-hub`.
	// const HTTP_REMOTE_REQUEST: &str = "https://api.github.com/orgs/substrate-developer-hub";
	const HTTP_GITHUB_REMOTE_REQUEST: &str = "https://api.github.com/orgs/substrate-developer-hub";
	const HTTP_PRICE_REMOTE_REQUEST: &str = "https://api.coincap.io/v2/assets/polkadot";
	const HTTP_HEADER_USER_AGENT: &str = "tofu2022";

	const FETCH_TIMEOUT_PERIOD: u64 = 3000; // in milli-seconds
	const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds
	const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number

	// the maximum accuracy of Permill::from_parts
	const PERMILL_MAX: u64 = 1000000;

	/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrapper.
	/// We can utilize the supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
	/// them with the pallet-specific identifier.
	pub mod crypto {
		use crate::KEY_TYPE;
		use sp_core::sr25519::Signature as Sr25519Signature;
		use sp_runtime::app_crypto::{app_crypto, sr25519};
		use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

		app_crypto!(sr25519, KEY_TYPE);

		pub struct TestAuthId;
		// implemented for ocw-runtime
		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}

		// implemented for mock runtime in test
		impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
		{
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}
	}

	// Define the unsigned transactions payload
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct Payload<Public> {
		number: u64,
		public: Public,
	}

	impl<T: SigningTypes> SignedPayload<T> for Payload<T::Public> {
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct PayloadPrice<Public> {
		number: Price,
		public: Public,
	}

	impl<T: SigningTypes> SignedPayload<T> for PayloadPrice<T::Public> {
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}

	/// Define a struct to fetch the remote info from github
	// ref: https://serde.rs/container-attrs.html#crate
	#[derive(Deserialize, Encode, Decode, Default, scale_info::TypeInfo)]
	struct GithubInfo {
		// Specify our own deserializing function to convert JSON string to vector of bytes
		#[serde(deserialize_with = "de_string_to_bytes")]
		login: Vec<u8>,
		#[serde(deserialize_with = "de_string_to_bytes")]
		blog: Vec<u8>,
		public_repos: u32,
	}

	pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
		where
			D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(de)?;
		Ok(s.as_bytes().to_vec())
	}

	impl fmt::Debug for GithubInfo {
		// `fmt` converts the vector of bytes inside the struct back to string for
		//   more friendly display.
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			write!(
				f,
				"{{ login: {}, blog: {}, public_repos: {} }}",
				str::from_utf8(&self.login).map_err(|_| fmt::Error)?,
				str::from_utf8(&self.blog).map_err(|_| fmt::Error)?,
				&self.public_repos
			)
		}
	}

	/// Define a struct to fetch the price info from polkadot
	// Json format ref: https://api.coincap.io/v2/assets/polkadot
	#[derive(Deserialize, Encode, Decode, Default, scale_info::TypeInfo)]
	struct PriceInfoOuter {
		data: PriceInfo,
		timestamp: u64,
	}

	#[derive(Deserialize, Encode, Decode, Default, scale_info::TypeInfo)]
	struct PriceInfo {
		#[serde(deserialize_with = "de_string_to_tuple")]
		priceUsd: Price,
	}

	// Todo: move to pallet::config
	type Price = (u64, Permill);
	fn format_price(data: Price) -> String {
		data.0.to_string() + "." + &data.1.deconstruct().to_string()
	}

	pub fn de_string_to_tuple<'de, D>(de: D) -> Result<Price, D::Error>
		where
			D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(de)?;
		let partition: Vec<&str> = s.split(".").collect();

		// todo: avoid overflow from string to integer
		let integer = partition[0].parse::<u64>().unwrap();
		let decimal = cut_u64_to_u32_permill(
			partition[1].parse::<u64>().unwrap());
		let permill = Permill::from_parts(decimal);
		log::info!("Price parsing result: ({}, {})", integer, decimal);

		let price: Price = (integer, permill);
		log::info!("Price decode result: ({})", format_price(price));

		Ok(price)
	}

	fn cut_u64_to_u32_permill(mut num: u64) -> u32 {
		while num > PERMILL_MAX {
			num /= 10;
		}
		return num as u32
	}

	// Debug trait for type PriceInfo is required by log::info! macro
	impl fmt::Debug for PriceInfo {
		// `fmt` converts the vector of bytes inside the struct back to string for
		//   more friendly display.
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			write!(
				f,
				"{{ price used: {} }}",
				format_price(self.priceUsd)
			)
		}
	}


	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The overarching dispatch call type.
		type Call: From<Call<Self>>;
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn numbers)]
	// Learn more about declaring storage items:
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
	pub type Numbers<T> = StorageValue<_, VecDeque<u64>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn prices)]
	pub type Prices<T> = StorageValue<_, VecDeque<(u64, Permill)>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewNumber(Option<T::AccountId>, u64),
		NewPrice(Option<T::AccountId>, Price),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		// Error returned when not sure which ocw function to executed
		UnknownOffchainMux,

		// Error returned when making signed transactions in off-chain worker
		NoLocalAcctForSigning,
		OffchainSignedTxError,

		// Error returned when making unsigned transactions in off-chain worker
		OffchainUnsignedTxError,

		// Error returned when making unsigned transactions with signed payloads in off-chain worker
		OffchainUnsignedTxSignedPayloadError,

		// Error returned when fetching github info
		HttpFetchingError,

		// Error returned when try the lock of local persistent storage
		TryLockFailed,

		// Error returned when failed to deserialize the json string to type
		DeserializeError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("Hello World from offchain workers!");

			// Here we are showcasing various techniques used when running off-chain workers (ocw)
			// 1. Sending signed transaction from ocw
			// 2. Sending unsigned transaction from ocw
			// 3. Sending unsigned transactions with signed payloads from ocw
			// 4. Fetching JSON via http requests in ocw
			const TX_TYPES: u32 = 5;
			let modu = block_number.try_into().map_or(TX_TYPES, |bn: usize| (bn as u32) % TX_TYPES);
			let result = match modu {
				0 => Self::offchain_signed_tx(block_number),
				1 => Self::offchain_unsigned_tx(block_number),
				2 => Self::offchain_unsigned_tx_signed_payload(block_number),
				3 => Self::fetch_github_info(),
				4 => Self::fetch_price_n_submit_tx(),
				_ => Err(Error::<T>::UnknownOffchainMux),
			};

			if let Err(e) = result {
				log::error!("offchain_worker error: {:?}", e);
			}

			log::info!("End of offchain workers!");
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call)
		-> TransactionValidity
		{
			let valid_tx = |provide| ValidTransaction::with_tag_prefix("ocw-demo")
			.priority(UNSIGNED_TXS_PRIORITY)
			.and_provides([&provide])
			.longevity(3)
			.propagate(true)
			.build();

			match call {
				Call::submit_number_unsigned { number: _number } => valid_tx(b"submit_number_unsigned".to_vec()),
				Call::submit_number_unsigned_with_signed_payload { ref payload, ref signature } => {
					if !SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone()) {
						return InvalidTransaction::BadProof.into();
					}
					valid_tx(b"submit_number_unsigned_with_signed_payload".to_vec())
				},
				Call::submit_price_unsigned_with_signed_payload { ref payload, ref signature } => {
					if !SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone()) {
						return InvalidTransaction::BadProof.into();
					}
					valid_tx(b"submit_number_unsigned_with_signed_payload".to_vec())
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		pub fn submit_number_signed(origin: OriginFor<T>, number: u64) -> DispatchResult {
			let who = ensure_signed(origin)?;
			log::info!("submit_number_signed: ({}, {:?})", number, who);
			Self::append_or_replace_number(number);

			Self::deposit_event(Event::NewNumber(Some(who), number));
			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn submit_number_unsigned(origin: OriginFor<T>, number: u64) -> DispatchResult {
			let _ = ensure_none(origin)?;
			log::info!("submit_number_unsigned: {}", number);
			Self::append_or_replace_number(number);

			Self::deposit_event(Event::NewNumber(None, number));
			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn submit_number_unsigned_with_signed_payload(origin: OriginFor<T>, payload: Payload<T::Public>,
			_signature: T::Signature) -> DispatchResult
		{
			let _ = ensure_none(origin)?;
			// we don't need to verify the signature here because it has been verified in
			//   `validate_unsigned` function when sending out the unsigned tx.
			let Payload { number, public } = payload;
			log::info!("submit_number_unsigned_with_signed_payload: ({}, {:?})", number, public);
			Self::append_or_replace_number(number);

			Self::deposit_event(Event::NewNumber(None, number));
			Ok(())
		}

		/// 作业回答：
		/// 选择unsigned transaction是将其与普通交易区分开
		/// 选择signed payload是因为price等信息需要有对应的问责对象，否则可能导致spamming
		#[pallet::weight(10000)]
		pub fn submit_price_unsigned_with_signed_payload(origin: OriginFor<T>, payload: PayloadPrice<T::Public>,
														  _signature: T::Signature) -> DispatchResult
		{
			let _ = ensure_none(origin)?;
			// we don't need to verify the signature here because it has been verified in
			//   `validate_unsigned` function when sending out the unsigned tx.
			let PayloadPrice { number, public } = payload;
			log::info!("submit_number_unsigned_with_signed_payload: ({}, {:?})", format_price(number), public);
			Self::append_or_replace_price(number);

			Self::deposit_event(Event::NewPrice(None, number));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Append a new number to the tail of the list, removing an element from the head if reaching
		///   the bounded length.
		fn append_or_replace_number(number: u64) {
			Numbers::<T>::mutate(|numbers| {
				if numbers.len() == NUM_VEC_LEN {
					let _ = numbers.pop_front();
				}
				numbers.push_back(number);
				log::info!("Number vector: {:?}", numbers);
			});
		}

		fn append_or_replace_price(value: Price) {
			Prices::<T>::mutate(|values| {
				if values.len() == NUM_VEC_LEN {
					let _ = values.pop_front();
				}
				values.push_back(value);
				// log::info!("New price vector: {:?}", values);
			});
		}

		// todo generalize append_or_replace function
		/*
		fn append_or_replace<P: S, U>(number: U) {
			P::<T>::mutate(|values| {
				if values.len() == NUM_VEC_LEN {
					let _ = values.pop_front();
				}
				values.push_back(number);
				log::info!("New vector: {:?}", values);
			});
		}
		 */

		/// fetch the price and generate an unsigned transaction
		fn fetch_price_n_submit_tx() -> Result<(), Error<T>> {
			match Self::fetch_price_info() {
				Ok(price) => {
					log::info!("fetch price successfully: ({:?})", price);
					return Self::offchain_unsigned_tx_signed_payload_price(price.priceUsd)
				}
				Err(err) => {
					return Err(err);
				}
			}
		}

		/// fetch the price from a local storage or a remote url
		fn fetch_price_info() -> Result<PriceInfo, Error<T>> {

			// Note: 建立持久化存储的意义在于部分数据更新频次低，但链上访问频繁
			// 因此通过本地持久化存储的方式，减少远程通信开销，同时提高访问速度
			// 然而在价格预言机的场景中，价格频繁更新，因此不适用持久化存储
			/*
			// fetch price from local storage, if success return the value
			let s_info = StorageValueRef::persistent(b"offchain-demo::price-info");

			if let Ok(Some(price_info)) = s_info.get::<PriceInfo>() {
				log::info!("cached price-info: {:?}", price_info);
				return Ok(price_info);
			}*/

			// if not, try to lock the storage and fetch the price remotely
			let mut lock = StorageLock::<BlockAndTime<Self>>::with_block_and_time_deadline(
				b"offchain-demo::price_lock", LOCK_BLOCK_EXPIRATION,
				rt_offchain::Duration::from_millis(LOCK_TIMEOUT_EXPIRATION)
			);

			lock.try_lock().map_err(|_| <Error<T>>::TryLockFailed)?;

			match Self::fetch_n_parse_price() {
				Ok(price_info) => {
					s_info.set(&price_info);
					return Ok(price_info);
				}
				Err(err) => { return Err(err); }
			}
		}


		/// Check if we have fetched github info before. If yes, we can use the cached version
		///   stored in off-chain worker storage `storage`. If not, we fetch the remote info and
		///   write the info into the storage for future retrieval.
		fn fetch_github_info() -> Result<(), Error<T>> {
			// Create a reference to Local Storage value.
			// Since the local storage is common for all offchain workers, it's a good practice
			// to prepend our entry with the pallet name.
			let s_info = StorageValueRef::persistent(b"offchain-demo::gh-info");

			// Local storage is persisted and shared between runs of the offchain workers,
			// offchain workers may run concurrently. We can use the `mutate` function to
			// write a storage entry in an atomic fashion.
			//
			// With a similar API as `StorageValue` with the variables `get`, `set`, `mutate`.
			// We will likely want to use `mutate` to access
			// the storage comprehensively.
			//
			if let Ok(Some(gh_info)) = s_info.get::<GithubInfo>() {
				// gh-info has already been fetched. Return early.
				log::info!("cached gh-info: {:?}", gh_info);
				return Ok(());
			}

			// Since off-chain storage can be accessed by off-chain workers from multiple runs, it is important to lock
			//   it before doing heavy computations or write operations.
			//
			// There are four ways of defining a lock:
			//   1) `new` - lock with default time and block exipration
			//   2) `with_deadline` - lock with default block but custom time expiration
			//   3) `with_block_deadline` - lock with default time but custom block expiration
			//   4) `with_block_and_time_deadline` - lock with custom time and block expiration
			// Here we choose the most custom one for demonstration purpose.
			let mut lock = StorageLock::<BlockAndTime<Self>>::with_block_and_time_deadline(
				b"offchain-demo::lock", LOCK_BLOCK_EXPIRATION,
				rt_offchain::Duration::from_millis(LOCK_TIMEOUT_EXPIRATION)
				);

			// We try to acquire the lock here. If failed, we know the `fetch_n_parse` part inside is being
			//   executed by previous run of ocw, so the function just returns.
			if let Ok(_guard) = lock.try_lock() {
				match Self::fetch_n_parse() {
					Ok(gh_info) => { s_info.set(&gh_info); }
					Err(err) => { return Err(err); }
				}
			}
			Ok(())
		}

		/// Fetch from remote and deserialize the JSON to a struct
		fn fetch_n_parse() -> Result<GithubInfo, Error<T>> {
			let resp_bytes = Self::fetch_from_remote(HTTP_GITHUB_REMOTE_REQUEST).map_err(|e| {
				log::error!("fetch_from_remote error: {:?}", e);
				<Error<T>>::HttpFetchingError
			})?;

			let resp_str = str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;
			// Print out our fetched JSON string
			log::info!("{}", resp_str);

			// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
			let gh_info: GithubInfo =
			serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::HttpFetchingError)?;
			Ok(gh_info)
		}

		fn fetch_n_parse_price() -> Result<PriceInfo, Error<T>> {
			let resp_bytes = Self::fetch_from_remote(HTTP_PRICE_REMOTE_REQUEST).map_err(|e| {
				log::error!("fetch_from_remote error: {:?}", e);
				<Error<T>>::HttpFetchingError
			})?;

			let resp_str = str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;
			// Print out our fetched JSON string
			log::info!("{}", resp_str);

			// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
			let price_info: PriceInfoOuter =
				serde_json::from_str(&resp_str).map_err(|e| {
					log::error!("deserialize PriceInfoOuter error: {:?}", e);
					<Error<T>>::DeserializeError
				})?;
			Ok(price_info.data)
		}

		/// todo: generalize the fetch_n_parse function
		/*
		fn fetch_n_parse_general<'a, P: Deserialize<'a>>(url: &str) -> Result<P, Error<T>> {
			let resp_bytes = Self::fetch_from_remote(url).map_err(|e| {
				log::error!("fetch_from_remote error: {:?}", e);
				<Error<T>>::HttpFetchingError
			})?;

			let resp_str = str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;
			// Print out our fetched JSON string
			log::info!("{}", resp_str);

			// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
			let parse_info: P =
				serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::HttpFetchingError)?;
			Ok(parse_info)
		}
		 */

		/// This function uses the `offchain::http` API to query the remote github information,
		///   and returns the JSON response as vector of bytes.
		fn fetch_from_remote(url: &str) -> Result<Vec<u8>, Error<T>> {
			log::info!("sending request to: {}", url);

			// Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
			let request = rt_offchain::http::Request::get(url);

			// Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
			let timeout = sp_io::offchain::timestamp()
			.add(rt_offchain::Duration::from_millis(FETCH_TIMEOUT_PERIOD));

			// For github API request, we also need to specify `user-agent` in http request header.
			//   See: https://developer.github.com/v3/#user-agent-required
			let pending = request
			.add_header("User-Agent", HTTP_HEADER_USER_AGENT)
				.deadline(timeout) // Setting the timeout time
				.send() // Sending the request out by the host
				.map_err(|_| <Error<T>>::HttpFetchingError)?;

			// By default, the http request is async from the runtime perspective. So we are asking the
			//   runtime to wait here.
			// The returning value here is a `Result` of `Result`, so we are unwrapping it twice by two `?`
			//   ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
			let response = pending
			.try_wait(timeout)
			.map_err(|_| <Error<T>>::HttpFetchingError)?
			.map_err(|_| <Error<T>>::HttpFetchingError)?;

			if response.code != 200 {
				log::error!("Unexpected http request status code: {}", response.code);
				return Err(<Error<T>>::HttpFetchingError);
			}

			// Next we fully read the response body and collect it to a vector of bytes.
			Ok(response.body().collect::<Vec<u8>>())
		}

		fn offchain_signed_tx(block_number: T::BlockNumber) -> Result<(), Error<T>> {
			// We retrieve a signer and check if it is valid.
			//   Since this pallet only has one key in the keystore. We use `any_account()1 to
			//   retrieve it. If there are multiple keys and we want to pinpoint it, `with_filter()` can be chained,
			let signer = Signer::<T, T::AuthorityId>::any_account();

			// Translating the current block number to number and submit it on-chain
			let number: u64 = block_number.try_into().unwrap_or(0);

			// `result` is in the type of `Option<(Account<T>, Result<(), ()>)>`. It is:
			//   - `None`: no account is available for sending transaction
			//   - `Some((account, Ok(())))`: transaction is successfully sent
			//   - `Some((account, Err(())))`: error occured when sending the transaction
			let result = signer.send_signed_transaction(|_acct|
				// This is the on-chain function
				Call::submit_number_signed { number }
				);

			// Display error if the signed tx fails.
			if let Some((acc, res)) = result {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Err(<Error<T>>::OffchainSignedTxError);
				}
				// Transaction is sent successfully
				return Ok(());
			}

			// The case of `None`: no account is available for sending
			log::error!("No local account available");
			Err(<Error<T>>::NoLocalAcctForSigning)
		}

		fn offchain_unsigned_tx(block_number: T::BlockNumber) -> Result<(), Error<T>> {
			let number: u64 = block_number.try_into().unwrap_or(0);
			let call = Call::submit_number_unsigned { number };

			// `submit_unsigned_transaction` returns a type of `Result<(), ()>`
			//   ref: https://substrate.dev/rustdocs/v2.0.0/frame_system/offchain/struct.SubmitTransaction.html#method.submit_unsigned_transaction
			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| {
				log::error!("Failed in offchain_unsigned_tx");
				<Error<T>>::OffchainUnsignedTxError
			})
		}

		fn offchain_unsigned_tx_signed_payload(block_number: T::BlockNumber) -> Result<(), Error<T>> {
			// Retrieve the signer to sign the payload
			let signer = Signer::<T, T::AuthorityId>::any_account();

			let number: u64 = block_number.try_into().unwrap_or(0);

			// `send_unsigned_transaction` is returning a type of `Option<(Account<T>, Result<(), ()>)>`.
			//   Similar to `send_signed_transaction`, they account for:
			//   - `None`: no account is available for sending transaction
			//   - `Some((account, Ok(())))`: transaction is successfully sent
			//   - `Some((account, Err(())))`: error occured when sending the transaction
			if let Some((_, res)) = signer.send_unsigned_transaction(
				|acct| Payload { number, public: acct.public.clone() },
				|payload, signature| Call::submit_number_unsigned_with_signed_payload { payload, signature }
					) {
				return res.map_err(|_| {
					log::error!("Failed in offchain_unsigned_tx_signed_payload");
					<Error<T>>::OffchainUnsignedTxSignedPayloadError
				});
			}

			// The case of `None`: no account is available for sending
			log::error!("No local account available");
			Err(<Error<T>>::NoLocalAcctForSigning)
		}

		fn offchain_unsigned_tx_signed_payload_price(price: Price) -> Result<(), Error<T>> {
			// Retrieve the signer to sign the payload
			let signer = Signer::<T, T::AuthorityId>::any_account();

			// `send_unsigned_transaction` is returning a type of `Option<(Account<T>, Result<(), ()>)>`.
			//   Similar to `send_signed_transaction`, they account for:
			//   - `None`: no account is available for sending transaction
			//   - `Some((account, Ok(())))`: transaction is successfully sent
			//   - `Some((account, Err(())))`: error occured when sending the transaction
			if let Some((_, res)) = signer.send_unsigned_transaction(
				|acct| PayloadPrice { number: price, public: acct.public.clone() },
				|payload, signature| Call::submit_price_unsigned_with_signed_payload { payload, signature }
			) {
				return res.map_err(|_| {
					log::error!("Failed in offchain_unsigned_tx_signed_payload");
					<Error<T>>::OffchainUnsignedTxSignedPayloadError
				});
			}

			// The case of `None`: no account is available for sending
			log::error!("No local account available");
			Err(<Error<T>>::NoLocalAcctForSigning)
		}
	}

	impl<T: Config> BlockNumberProvider for Pallet<T> {
		type BlockNumber = T::BlockNumber;

		fn current_block_number() -> Self::BlockNumber {
			<frame_system::Pallet<T>>::block_number()
		}
	}
}
