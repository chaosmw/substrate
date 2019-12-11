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
		pub const ReservationFee: u64 = 2;
		pub const MinLength: usize = 3;
		pub const MaxLength: usize = 16;
		pub const MaxZoneLength: usize = 1024;
		pub const One: u64 = 1;
	}
	impl Trait for Test {
		type Event = ();
		type Currency = Balances;
		type ReservationFee = ReservationFee;
		type Slashed = ();
		type ForceOrigin = EnsureSignedBy<One, u64>;
		type MinLength = MinLength;
		type MaxLength = MaxLength;
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
	fn kill_name_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(NameService::set_name(Origin::signed(2), b"Dave".to_vec()));
			assert_eq!(Balances::total_balance(&2), 10);
			assert_ok!(NameService::kill_name(Origin::signed(1), 2));
			assert_eq!(Balances::total_balance(&2), 8);
			assert_eq!(<NameOf<Test>>::get(2), None);
		});
	}

	#[test]
	fn force_name_should_work() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				NameService::set_name(Origin::signed(2), b"Dr. David Brubeck, III".to_vec()),
				"Name too long"
			);

			assert_ok!(NameService::set_name(Origin::signed(2), b"Dave".to_vec()));
			assert_eq!(Balances::reserved_balance(&2), 2);
			assert_ok!(NameService::force_name(Origin::signed(1), 2, b"Dr. David Brubeck, III".to_vec()));
			assert_eq!(Balances::reserved_balance(&2), 2);
			assert_eq!(<NameOf<Test>>::get(2).unwrap(), (b"Dr. David Brubeck, III".to_vec(), 2));
		});
	}

	#[test]
	fn normal_operation_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(NameService::set_name(Origin::signed(1), b"Gav".to_vec()));
			assert_eq!(Balances::reserved_balance(&1), 2);
			assert_eq!(Balances::free_balance(&1), 8);
			assert_eq!(<NameOf<Test>>::get(1).unwrap().0, b"Gav".to_vec());

			assert_ok!(NameService::set_name(Origin::signed(1), b"Gavin".to_vec()));
			assert_eq!(Balances::reserved_balance(&1), 2);
			assert_eq!(Balances::free_balance(&1), 8);
			assert_eq!(<NameOf<Test>>::get(1).unwrap().0, b"Gavin".to_vec());

			assert_ok!(NameService::clear_name(Origin::signed(1)));
			assert_eq!(Balances::reserved_balance(&1), 0);
			assert_eq!(Balances::free_balance(&1), 10);
		});
	}

	#[test]
	fn error_catching_should_work() {
		new_test_ext().execute_with(|| {
			assert_noop!(NameService::clear_name(Origin::signed(1)), "Not named");

			assert_noop!(NameService::set_name(Origin::signed(3), b"Dave".to_vec()), "not enough free funds");

			assert_noop!(NameService::set_name(Origin::signed(1), b"Ga".to_vec()), "Name too short");
			assert_noop!(
				NameService::set_name(Origin::signed(1), b"Gavin James Wood, Esquire".to_vec()),
				"Name too long"
			);
			assert_ok!(NameService::set_name(Origin::signed(1), b"Dave".to_vec()));
			assert_noop!(NameService::kill_name(Origin::signed(2), 1), "bad origin");
			assert_noop!(NameService::force_name(Origin::signed(2), 1, b"Whatever".to_vec()), "bad origin");
		});
	}

	#[test]
	fn set_root_owner_should_work() {
		new_test_ext().execute_with(|| {
			assert_noop!(NameService::set_root_owner(Origin::signed(2), 3), "bad origin");
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
			assert_noop!(NameService::set_owner(Origin::signed(1), [1;32].into(), 4), "node does not exist");
			let root_hash = <Test as system::Trait>::Hash::default(); 
			assert_noop!(NameService::set_owner(Origin::signed(1), root_hash, 4), "sender is not owner");
			assert_ok!(NameService::set_owner(Origin::signed(3), root_hash, 4));
			assert_eq!(NameService::node_of(root_hash).unwrap().owner, 4);
		});	
	}

	#[test]
	fn set_subnode_owner_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(NameService::set_root_owner(Origin::signed(1), 3));
			let label = ("eth").using_encoded(<Test as system::Trait>::Hashing::hash); 
			assert_noop!(NameService::set_subnode_owner(Origin::signed(1), [1;32].into(), label, 4), "node does not exist");
			let root_hash = <Test as system::Trait>::Hash::default(); 
			assert_noop!(NameService::set_subnode_owner(Origin::signed(1), root_hash, label, 4), "sender is not owner");
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
			assert_noop!(NameService::set_subnode_owner(Origin::signed(1), [1;32].into(), label, 4), "node does not exist");
			let node_hash = <Test as system::Trait>::Hash::default(); 
			assert_noop!(NameService::set_ttl(Origin::signed(3), node_hash, 0), "ttl is the same value");
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
			assert_noop!(NameService::set_resolve_addr(Origin::signed(4), node_hash, addr), "addr is the same value");
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

			assert_noop!(NameService::set_resolve_name(Origin::signed(4), node_hash, "e".into()), "name too short");
			assert_noop!(NameService::set_resolve_name(Origin::signed(4), node_hash, "e".repeat(17).into()), "name too long");
			
			assert_ok!(NameService::set_resolve_name(Origin::signed(4), node_hash, "eth".into()));
			assert_noop!(NameService::set_resolve_name(Origin::signed(4), node_hash, "eth".into()), "name is the same value");
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
			assert_noop!(NameService::set_resolve_profile(Origin::signed(4), node_hash, profile), "profile is the same value");
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
			assert_noop!(NameService::set_resolve_zone(Origin::signed(4), node_hash, "z".repeat(1025).into()), "zone content too long");
			
			assert_ok!(NameService::set_resolve_zone(Origin::signed(4), node_hash, zone.into()));
			assert_noop!(NameService::set_resolve_zone(Origin::signed(4), node_hash, zone.into()), "zone is the same value");
			assert_eq!(NameService::resolve_of(node_hash).unwrap().zone, zone.as_bytes());
		});
	}

}