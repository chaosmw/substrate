/// tests for this module
#[cfg(test)]
mod tests {
    use crate::*;
	use super::*;

	use support::{assert_ok, assert_noop, impl_outer_origin, parameter_types, weights::Weight};
	use primitives::H256;
	use system::EnsureSignedBy;
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
	parameter_types! {
		pub const MinNameLength: usize = 3;
		pub const MaxNameLength: usize = 16;
		pub const MaxZoneLength: usize = 1024;
		pub const One: u64 = 1;
	}
	impl Trait for Test {
		type Event = ();
		type ForceOrigin = EnsureSignedBy<One, u64>;
		type MinNameLength = MinNameLength;
		type MaxNameLength = MaxNameLength;
		type MaxZoneLength = MaxZoneLength;
	}

	type System = system::Module<Test>;
	type Balances = balances::Module<Test>;
	type NameService = Module<Test>;

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
	fn set_root_owner_should_work() {
		new_test_ext().execute_with(|| {
			assert_noop!(NameService::set_root_owner(Origin::signed(2), 3), "Bad origin");
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			assert_eq!(NameService::node_of(<Test as system::Trait>::Hash::default()).unwrap().owner, 3);
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 4));
			assert_eq!(NameService::node_of(<Test as system::Trait>::Hash::default()).unwrap().owner, 4);
		});	
	}

	#[test]
	fn set_owner_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			assert_noop!(NameService::set_owner(Origin::signed(1), [1;32].into(), 4), "Node does not exist");
			let root_hash = <Test as system::Trait>::Hash::default(); 
			assert_noop!(NameService::set_owner(Origin::signed(1), root_hash, 4), "Sender is not owner");
			assert_ok!(NameService::set_owner(Origin::signed(3), root_hash, 4));
			assert_eq!(NameService::node_of(root_hash).unwrap().owner, 4);
		});	
	}

	#[test]
	fn set_subnode_owner_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			let label = ("eth").using_encoded(<Test as system::Trait>::Hashing::hash); 
			assert_noop!(NameService::set_subnode_owner(Origin::signed(1), [1;32].into(), label, 4), "Node does not exist");
			let root_hash = <Test as system::Trait>::Hash::default(); 
			assert_noop!(NameService::set_subnode_owner(Origin::signed(1), root_hash, label, 4), "Sender is not owner");
			// set eth to account 4
			assert_ok!(NameService::set_subnode_owner(Origin::signed(3), root_hash, label, 4));
			let node_hash = (root_hash, label).using_encoded(<Test as system::Trait>::Hashing::hash);
			assert_eq!(NameService::node_of(node_hash).unwrap().owner, 4);
			println!("node_hash={}", node_hash);

			let label = ("hsiung").using_encoded(<Test as system::Trait>::Hashing::hash);
			// set hsiung.eth to account 5
			assert_ok!(NameService::set_subnode_owner(Origin::signed(4), node_hash, label, 5));
			let node_hash = (node_hash, label).using_encoded(<Test as system::Trait>::Hashing::hash);
			assert_eq!(NameService::node_of(node_hash).unwrap().owner, 5);
			println!("node_hash={}", node_hash);	
		});	
	}

	#[test]
	fn set_ttl_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			let label = ("eth").using_encoded(<Test as system::Trait>::Hashing::hash); 
			assert_noop!(NameService::set_subnode_owner(Origin::signed(1), [1;32].into(), label, 4), "Node does not exist");
			let node_hash = <Test as system::Trait>::Hash::default(); 
			assert_noop!(NameService::set_ttl(Origin::signed(3), node_hash, 0), "TTL is the same value");
			assert_ok!(NameService::set_ttl(Origin::signed(3), node_hash, 10));
			assert_eq!(NameService::node_of(node_hash).unwrap().ttl, 10);
		});
	}

	#[test]
	fn set_resolve_addr_should_work() {
		new_test_ext().execute_with(||{
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			let label = ("eth").using_encoded(<Test as system::Trait>::Hashing::hash); 
			let root_hash = <Test as system::Trait>::Hash::default(); 
			assert_ok!(NameService::set_subnode_owner(Origin::signed(3), root_hash, label, 4));
			let node_hash = (root_hash, label).using_encoded(<Test as system::Trait>::Hashing::hash);

			let addr = 1004;
			assert_ok!(NameService::set_resolve_addr(Origin::signed(4), node_hash, addr));
			assert_noop!(NameService::set_resolve_addr(Origin::signed(4), node_hash, addr), "Addr is the same value");
			assert_eq!(NameService::resolve_of(node_hash).unwrap().addr, addr);
		});
	}

	#[test]
	fn set_resolve_name_should_work() {
		new_test_ext().execute_with(||{
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			let label = ("eth").using_encoded(<Test as system::Trait>::Hashing::hash); 
			let root_hash = <Test as system::Trait>::Hash::default(); 
			assert_ok!(NameService::set_subnode_owner(Origin::signed(3), root_hash, label, 4));
			let node_hash = (root_hash, label).using_encoded(<Test as system::Trait>::Hashing::hash);

			assert_noop!(NameService::set_resolve_name(Origin::signed(4), node_hash, "e".into()), "Name too short");
			assert_noop!(NameService::set_resolve_name(Origin::signed(4), node_hash, "e".repeat(17).into()), "Name too long");
			
			assert_ok!(NameService::set_resolve_name(Origin::signed(4), node_hash, "eth".into()));
			assert_noop!(NameService::set_resolve_name(Origin::signed(4), node_hash, "eth".into()), "Name is the same value");
			assert_eq!(NameService::resolve_of(node_hash).unwrap().name, "eth".as_bytes());
		});
	}


	#[test]
	fn set_resolve_profile_should_work() {
		new_test_ext().execute_with(||{
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			let label = ("eth").using_encoded(<Test as system::Trait>::Hashing::hash); 
			let root_hash = <Test as system::Trait>::Hash::default(); 
			assert_ok!(NameService::set_subnode_owner(Origin::signed(3), root_hash, label, 4));
			let node_hash = (root_hash, label).using_encoded(<Test as system::Trait>::Hashing::hash);

			let profile = ("did:pistis:v0:1LrMVQmmEvJXsTmrXuarGrikk5nnB5Cvwg-1").using_encoded(<Test as system::Trait>::Hashing::hash);
			assert_ok!(NameService::set_resolve_profile(Origin::signed(4), node_hash, profile));
			assert_noop!(NameService::set_resolve_profile(Origin::signed(4), node_hash, profile), "Profile is the same value");
			assert_eq!(NameService::resolve_of(node_hash).unwrap().profile, profile);
		});
	}

	#[test]
	fn set_resolve_zone_should_work() {
		new_test_ext().execute_with(||{
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			let label = ("eth").using_encoded(<Test as system::Trait>::Hashing::hash); 
			let root_hash = <Test as system::Trait>::Hash::default(); 
			assert_ok!(NameService::set_subnode_owner(Origin::signed(3), root_hash, label, 4));
			let node_hash = (root_hash, label).using_encoded(<Test as system::Trait>::Hashing::hash);

			let zone = r#"{"compacity":50000000,"class":"normal","storage":"http://example.com/1LrMVQmmEvJXsTmrXuarGrikk5nnB5Cvwg"}"#;
			assert_noop!(NameService::set_resolve_zone(Origin::signed(4), node_hash, "z".repeat(1025).into()), "Zone content too long");
			
			assert_ok!(NameService::set_resolve_zone(Origin::signed(4), node_hash, zone.into()));
			assert_noop!(NameService::set_resolve_zone(Origin::signed(4), node_hash, zone.into()), "Zone is the same value");
			assert_eq!(NameService::resolve_of(node_hash).unwrap().zone, zone.as_bytes());
		});
	}

	#[test]
	fn blake2_name_hash_should_work() {
		let data = b"eth";
		let label = (data).using_encoded(<Test as system::Trait>::Hashing::hash); 
		println!("label1 = {:#?}", label);

		let mut dest = [0;32];
		dest.copy_from_slice(blake2_rfc::blake2b::blake2b(32, &[], data).as_bytes());
		let label:H256 = dest.into(); 
		println!("label2 = {:#?}", label);

		let hash = <Test as system::Trait>::Hashing::hash(data);
		println!("label3 = {:#?}", hash);

		let hash = <Test as system::Trait>::Hashing::hash(String::from("eth").as_bytes());
		println!("label4 = {:#?}", hash);

		// should use byte literal
		let label = ("eth").using_encoded(<Test as system::Trait>::Hashing::hash); 
		println!("label5 = {:#?}", label);
		assert_ne!(hash, label);

		println!("root hash = {:#?}", <Test as system::Trait>::Hashing::hash(&[0u8;32]));

		let label = (String::from("eth").as_bytes()).using_encoded(<Test as system::Trait>::Hashing::hash); 
		println!("label6 = {:#?}", label);

		let node_hash: H256 = from_slice(&NameService::namehash("hsiung.eth")).into();
		println!("namehash of hsiung.eth = {:#?}", node_hash);
	}

	fn from_slice(bytes: &[u8]) -> [u8; 32] {
		let mut array = [0; 32];
		let bytes = &bytes[..array.len()]; // panics if not enough data
		array.copy_from_slice(bytes); 
		array
	}

}