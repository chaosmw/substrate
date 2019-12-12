/// tests for this module
#[cfg(test)]
mod tests {
    use crate::*;
	use super::*;

	use support::{assert_ok, assert_noop, impl_outer_origin, parameter_types, weights::Weight};
	use primitives::H256;
	use system::EnsureSignedBy;
	use name_service::NameServiceResolver;
	// The testing primitives are very useful for avoiding having to work with signatures
	// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
	use sp_runtime::{
		Perbill, testing::Header, traits::{BlakeTwo256, IdentityLookup},
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::one();
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Call = ();
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
	}
	parameter_types! {
		pub const ExistentialDeposit: u64 = 0;
		pub const TransferFee: u64 = 0;
		pub const CreationFee: u64 = 0;
	}
	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = ();
		type TransferPayment = ();
		type DustRemoval = ();
		type ExistentialDeposit = ExistentialDeposit;
		type TransferFee = TransferFee;
		type CreationFee = CreationFee;
	}

	const SCOPE_PISTIS: &str = "pistis"; 
	const BISINESS_OWNER: &str = "longguhu";
	const ALICE: &str = "alice";
	const BOB: &str = "bob";
	const RAY: &str = "ray";

	parameter_types! {
		pub const MinLength: usize = 3;
		pub const MaxLength: usize = 16;
		pub const MaxZoneLength: usize = 1024;
		pub const One: u64 = 1;
		pub const ScopeName: &'static str = SCOPE_PISTIS;
		pub const MaxSeqIDLength: usize = 64;
		pub const MaxExtraLength: usize = 1024;
		pub const MaxProductInfoCount: usize = 10;
	}
	impl Trait for Test {
		type Event = ();
		type ForceOrigin = EnsureSignedBy<One, u64>;
		type MinLength = MinLength;
		type MaxLength = MaxLength;
		type MaxZoneLength = MaxZoneLength;
		type ScopeName = ScopeName; 
		type MaxSeqIDLength = MaxSeqIDLength;
		type MaxExtraLength = MaxExtraLength;
		type MaxProductInfoCount = MaxProductInfoCount;
		type NameServiceResolver = Self;
	}

	impl NameServiceResolver<Test> for Test {
		fn resolve_addr(node_hash: <Test as system::Trait>::Hash) -> Option<<Test as system::Trait>::AccountId> {
			let scope = Self::single_name_hash(<Test as Trait>::ScopeName::get());
			let longguhu = Self::single_name_hash(BISINESS_OWNER);
			let alice = Self::single_name_hash(ALICE);
			let bob = Self::single_name_hash(BOB);

			let addr = if node_hash == scope {
				Some(1)
			} else if node_hash == longguhu {
				Some(2)
			} else if node_hash == alice {
				Some(3)
			} else if node_hash == bob {
				Some(4)
			} else {
				Some(100)
			}; 

			println!("resolved to addr {:#?}", addr);
			addr
		}
	}

	impl Test {
		pub fn single_name_hash(name: &str) -> <Test as system::Trait>::Hash {
			(name).using_encoded(<Test as system::Trait>::Hashing::hash)
		}
	}

	type System = system::Module<Test>;
	type Balances = balances::Module<Test>;
	type Service = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities {
		let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
		// We use default for brevity, but you can configure as desired if needed.
		balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 10),
				(2, 10),
			],
			vesting: vec![],
		}.assimilate_storage(&mut t).unwrap();
		t.into()
	}

	#[test]
	fn create_business_should_work() {
		new_test_ext().execute_with(|| {
			let owner_hash = (BISINESS_OWNER).using_encoded(<Test as system::Trait>::Hashing::hash); 
			assert_noop!(Service::create_business(Origin::signed(2), owner_hash, "crab".into(), 10), "Not authorized");
			assert_noop!(Service::create_business(Origin::signed(1), owner_hash, "c".into(), 10), "Name too short");
			assert_noop!(Service::create_business(Origin::signed(1), owner_hash, "c".repeat(17).into(), 10), "Name too long");
			System::set_block_number(10);
			assert_noop!(Service::create_business(Origin::signed(1), owner_hash, "crab".into(), 10), "Expired");
			let biz_hash = Service::business_hash(1, owner_hash);
			assert_ok!(Service::create_business(Origin::signed(1), owner_hash, "crab".into(), 20));
			assert_eq!(Service::block_number(), 10);
			assert_eq!(Service::business_of(biz_hash).creator, 1);
		});
	}

	#[test]
	fn set_business_expiration_should_work() {
		new_test_ext().execute_with(|| {
			let owner_hash = (BISINESS_OWNER).using_encoded(<Test as system::Trait>::Hashing::hash); 
			let biz_hash = <Test as system::Trait>::Hash::default();
			assert_noop!(Service::set_business_expiration(Origin::signed(2), biz_hash,  10), "Not authorized");
			assert_noop!(Service::set_business_expiration(Origin::signed(1), biz_hash,  10), "Business does not exist");

			System::set_block_number(10);
			let biz_hash = Service::business_hash(1, owner_hash);
			assert_ok!(Service::create_business(Origin::signed(1), owner_hash, "crab".into(), 20));

			assert_noop!(Service::set_business_expiration(Origin::signed(1), biz_hash,  10), "Expired");
			assert_noop!(Service::set_business_expiration(Origin::signed(1), biz_hash,  20), "Same value");
			assert_ok!(Service::set_business_expiration(Origin::signed(1), biz_hash,  15));
			assert_ok!(Service::set_business_expiration(Origin::signed(1), biz_hash,  25));
			assert_eq!(Service::business_of(biz_hash).expiration, 25);
		});
	}

	#[test]
	fn business_whitelist_should_work() {
		new_test_ext().execute_with(|| {
			let alice = (ALICE).using_encoded(<Test as system::Trait>::Hashing::hash);
			let bob = (BOB).using_encoded(<Test as system::Trait>::Hashing::hash);
			let ray = (RAY).using_encoded(<Test as system::Trait>::Hashing::hash);

			let owner_hash = (BISINESS_OWNER).using_encoded(<Test as system::Trait>::Hashing::hash); 
			let biz_hash = <Test as system::Trait>::Hash::default();
			assert_noop!(Service::add_business_whitelist(Origin::signed(1), biz_hash,  alice), "Business does not exist");

			System::set_block_number(10);
			let biz_hash = Service::business_hash(1, owner_hash);
			assert_ok!(Service::create_business(Origin::signed(1), owner_hash, "crab".into(), 20));
			assert_noop!(Service::add_business_whitelist(Origin::signed(3), biz_hash,  alice), "Not authorized");

			assert_ok!(Service::add_business_whitelist(Origin::signed(2), biz_hash,  alice));
			assert_noop!(Service::add_business_whitelist(Origin::signed(2), biz_hash,  alice), "Already in the whitelist");
			assert_ok!(Service::add_business_whitelist(Origin::signed(2), biz_hash,  bob));

			assert_eq!(Service::business_of(biz_hash).whitelist, [alice, bob]);
			assert_noop!(Service::remove_business_whitelist(Origin::signed(2), biz_hash,  ray), "Not in the whitelist");

			assert_ok!(Service::remove_business_whitelist(Origin::signed(2), biz_hash,  alice));
			assert_ok!(Service::remove_business_whitelist(Origin::signed(2), biz_hash,  bob));
			assert_eq!(Service::business_of(biz_hash).whitelist, []);
		});
	}

	#[test]
	fn product_should_work() {
		new_test_ext().execute_with(|| {
			let alice = (ALICE).using_encoded(<Test as system::Trait>::Hashing::hash);
			let bob = (BOB).using_encoded(<Test as system::Trait>::Hashing::hash);
			let ray = (RAY).using_encoded(<Test as system::Trait>::Hashing::hash);

			let owner_hash = (BISINESS_OWNER).using_encoded(<Test as system::Trait>::Hashing::hash); 
			let biz_hash = <Test as system::Trait>::Hash::default();
			let data_hash = ("I have a secret, haha~").using_encoded(<Test as system::Trait>::Hashing::hash);
			let extra = r#"{"amount":10000,"type":"btc","public_key":"1LrMVQmmEvJXsTmrXuarGrikk5nnB5Cvwg"}"#;
			let seq_id = &"1".repeat(64)[..];

			assert_noop!(Service::create_product(Origin::signed(3), bob, biz_hash, seq_id.into(), data_hash, extra.into()), "Not authorized");
			assert_noop!(Service::create_product(Origin::signed(3), alice, biz_hash, seq_id.into(), data_hash, extra.into()), "Business does not exist");

			System::set_block_number(10);
			let biz_hash = Service::business_hash(1, owner_hash);
			assert_ok!(Service::create_business(Origin::signed(1), owner_hash, "crab".into(), 20));

			assert_noop!(Service::create_product(Origin::signed(3), alice, biz_hash, seq_id.into(), data_hash, extra.into()), "Not in the whitelist");
			assert_ok!(Service::add_business_whitelist(Origin::signed(2), biz_hash,  alice));

			System::set_block_number(20);
			assert_noop!(Service::create_product(Origin::signed(3), alice, biz_hash, seq_id.into(), data_hash, extra.into()), "Expired");
			System::set_block_number(15);
			assert_noop!(Service::create_product(Origin::signed(3), alice, biz_hash, "1".repeat(65).into(), data_hash, extra.into()), "Sequence ID too long");
			assert_noop!(Service::create_product(Origin::signed(3), alice, biz_hash, seq_id.into(), data_hash, "e".repeat(1025).into()), "Extra info too long");

			assert_ok!(Service::create_product(Origin::signed(3), alice, biz_hash, seq_id.into(), data_hash, extra.into()));
			assert_noop!(Service::create_product(Origin::signed(3), alice, biz_hash, seq_id.into(), data_hash, extra.into()), "Product already exists");
			let product_hash = Service::product_hash(biz_hash, seq_id.into());	
			assert_eq!(Service::product_of(product_hash).seq_id, String::from(seq_id).as_bytes());
			assert_eq!(Service::product_of(product_hash).infos.len(), 1);
			assert_eq!(Service::product_of(product_hash).infos[0].data_hash, data_hash);

			assert_ok!(Service::add_business_whitelist(Origin::signed(2), biz_hash, bob));
			assert_noop!(Service::add_product_info(Origin::signed(3), bob, biz_hash, seq_id.into(), data_hash, extra.into()), "Not authorized");
			assert_ok!(Service::add_product_info(Origin::signed(4), bob, biz_hash, seq_id.into(), data_hash, extra.into()));

			assert_eq!(Service::product_of(product_hash).infos.len(), 2);
		});
	}

}