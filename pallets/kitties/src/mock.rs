use crate as pallet_kitties;
use pallet_kitties::Gender;
use frame_support::{
	parameter_types,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
		SubstrateKitties: pallet_kitties::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = u64;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

parameter_types! {
	// One can owned at most 5 Kitties
	pub const MaxKittyOwned: u32 = 5;
	// The reserve price of mining a kitty is 3
	pub const MinKittyMintingPrice: u32 = 3;
}

// impl Config for Test (add use super::*)
impl pallet_kitties::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type KittyRandomness = RandomnessCollectiveFlip;
	type MaxKittyOwned = MaxKittyOwned;
	type MinKittyMintingPrice = MinKittyMintingPrice;
	type KittyIndex = sp_core::H256;
	type KittyHashing = BlakeTwo256;
}

impl pallet_randomness_collective_flip::Config for Test {}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	GenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				(1,  100),
				(2,  10),
				(3,  2)
			]
		},
		substrate_kitties: SubstrateKittiesConfig {
			kitties: vec![
				(1, *b"1234567890123456", Gender::Female),
				(2, *b"123456789012345a", Gender::Male),
				(3, *b"123456789012345e", Gender::Male),
				(3, *b"1234567890123462", Gender::Male),
				(3, *b"1234567890123466", Gender::Female),
			]
		},
		..Default::default()
	}
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/*
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(1,  10), // accountID:1, balance:10
			(2,  10),
			(3,  2)
		],
	}
		.assimilate_storage(&mut t)
		.unwrap();

	pallet_kitties::GenesisConfig::<Test> {
		kitties: vec![
			(1, *b"1234567890123456", Gender::Female),
			(2, *b"123456789012345a", Gender::Male)
		]
	};

	// setting the block number to 1.
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
*/
