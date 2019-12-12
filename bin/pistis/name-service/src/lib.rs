//! # Name Service Module
//!
//! - [`name_service::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This module is for keeping track of account names on-chain. It aims to
//! create a name hierarchy, be a DNS replacement and provide reverse lookups.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! * `set_name` - Set the associated name of an account; a small deposit is reserved if not already
//!   taken.
//! * `clear_name` - Remove an account's associated name; the deposit is returned.
//! * `kill_name` - Forcibly remove the associated name; the deposit is lost.
//!
//! [`Call`]: ./enum.Call.html
//! [`Trait`]: ./trait.Trait.html

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use primitives::H256;
use rstd::prelude::*;
use sp_runtime::traits::{EnsureOrigin, Hash, StaticLookup, Zero};
use support::{
	decl_event, decl_module, decl_storage,
	dispatch::Result,
	ensure,
	traits::{Currency, Get, OnUnbalanced, ReservableCurrency},
	weights::SimpleDispatchInfo,
};
use system::{ensure_root, ensure_signed};
// use serde::{Serialize, Deserialize, de::DeserializeOwned};
// #[cfg(std)]
// use serde_json::Value;

// #[cfg(no_std)]
// use serde_json_core::Value;

#[cfg(test)]
mod name_service_test;

/// The node record
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct NodeRecord<AccountId> {
	/// The owner of the node
	pub owner: AccountId,
	/// The ttl of the record
	pub ttl: u64,
}

/// The resolve record
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct ResolveRecord<Hash, AccountId> {
	/// The resolved address
	pub addr: AccountId,
	/// The resolved name
	pub name: Vec<u8>,
	/// The resolved profile
	pub profile: Hash,
	/// The zone file
	pub zone: Vec<u8>,
}

// #[derive(Encode, Decode, Default, Clone, PartialEq)]
// pub struct ZoneFile {
// 	pub storage: Vec<u8>,
// 	pub read_url: Vec<u8>,
// 	pub write_url: Vec<u8>,
// }

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
type NegativeImbalanceOf<T> =
	<<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The currency trait.
	type Currency: ReservableCurrency<Self::AccountId>;

	/// Reservation fee.
	type ReservationFee: Get<BalanceOf<Self>>;

	/// What to do with slashed funds.
	type Slashed: OnUnbalanced<NegativeImbalanceOf<Self>>;

	/// The origin which may forcibly set or remove a name. Root can always do this.
	type ForceOrigin: EnsureOrigin<Self::Origin>;

	/// The minimum length a name may be.
	type MinLength: Get<usize>;

	/// The maximum length a name may be.
	type MaxLength: Get<usize>;

	/// The maxinum length a zone may be
	type MaxZoneLength: Get<usize>;
}

decl_storage! {
	trait Store for Module<T: Trait> as NameServiceModule {
		/// The lookup table for names.
		NameOf: map T::AccountId => Option<(Vec<u8>, BalanceOf<T>)>;
		/// The lookup table for node records
		NodeOf get(node_of): map T::Hash => Option<NodeRecord<T::AccountId>>;
		/// The lookup table for resolve records
		ResolveOf get(resolve_of): map T::Hash => Option<ResolveRecord<T::Hash, T::AccountId>>;
	}
}

decl_event!(
	pub enum Event<T>
	where
		Hash = <T as system::Trait>::Hash,
		AccountId = <T as system::Trait>::AccountId,
		Balance = BalanceOf<T>,
	{
		/// A name was set.
		NameSet(AccountId),
		/// A name was forcibly set.
		NameForced(AccountId),
		/// A name was changed.
		NameChanged(AccountId),
		/// A name was cleared, and the given balance returned.
		NameCleared(AccountId, Balance),
		/// A name was removed and the given balance slashed.
		NameKilled(AccountId, Balance),
		/// Logged when root is changed
		RootChanged(AccountId),
		/// Logged when the owner of a node assigns a new owner to a subnode.
		NewOwner(Hash, Hash, AccountId),
		/// Logged when the owner of a node transfers ownership to a new account.
		Transfer(Hash, AccountId),
		/// Logged when the resolver for a node changes.
		ResolveSet(Hash, AccountId),
		/// Logged when the TTL of a node changes
		NewTTL(Hash, u64),
		/// Logged when addr of resolve record changed
		ResolveAddrChanged(Hash, AccountId),
		/// Logged when name of resolve record changed
		ResolveNameChanged(Hash, Vec<u8>),
		/// Logged when profile of resolve record changed
		ResolveProfileChanged(Hash, Hash),
		/// Logged when zone of resolve record changed
		ResolveZoneChanged(Hash, Vec<u8>),
	}
);

decl_module! {
	// Simple declaration of the `Module` type. Lets the macro know what it's working on.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		/// Reservation fee.
		const ReservationFee: BalanceOf<T> = T::ReservationFee::get();

		/// The minimum length a name may be.
		const MinLength: u32 = T::MinLength::get() as u32;

		/// The maximum length a name may be.
		const MaxLength: u32 = T::MaxLength::get() as u32;

		/// The maximum length a zone may be.
		const MaxZoneLength: u32 = T::MaxZoneLength::get() as u32;

		/// Set an account's name. The name should be a UTF-8-encoded string by convention, though
		/// we don't check it.
		///
		/// The name may not be more than `T::MaxLength` bytes, nor less than `T::MinLength` bytes.
		///
		/// If the account doesn't already have a name, then a fee of `ReservationFee` is reserved
		/// in the account.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// # <weight>
		/// - O(1).
		/// - At most one balance operation.
		/// - One storage read/write.
		/// - One event.
		/// # </weight>
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_name(origin, name: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			ensure!(name.len() >= T::MinLength::get(), "Name too short");
			ensure!(name.len() <= T::MaxLength::get(), "Name too long");

			let deposit = if let Some((_, deposit)) = <NameOf<T>>::get(&sender) {
				Self::deposit_event(RawEvent::NameSet(sender.clone()));
				deposit
			} else {
				let deposit = T::ReservationFee::get();
				T::Currency::reserve(&sender, deposit.clone())?;
				Self::deposit_event(RawEvent::NameChanged(sender.clone()));
				deposit
			};

			<NameOf<T>>::insert(&sender, (name, deposit));
		}

		/// Clear an account's name and return the deposit. Fails if the account was not named.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// # <weight>
		/// - O(1).
		/// - One balance operation.
		/// - One storage read/write.
		/// - One event.
		/// # </weight>
		fn clear_name(origin) {
			let sender = ensure_signed(origin)?;

			let deposit = <NameOf<T>>::take(&sender).ok_or("Not named")?.1;

			let _ = T::Currency::unreserve(&sender, deposit.clone());

			Self::deposit_event(RawEvent::NameCleared(sender, deposit));
		}

		/// Remove an account's name and take charge of the deposit.
		///
		/// Fails if `who` has not been named. The deposit is dealt with through `T::Slashed`
		/// imbalance handler.
		///
		/// The dispatch origin for this call must be _Root_ or match `T::ForceOrigin`.
		///
		/// # <weight>
		/// - O(1).
		/// - One unbalanced handler (probably a balance transfer)
		/// - One storage read/write.
		/// - One event.
		/// # </weight>
		#[weight = SimpleDispatchInfo::FreeOperational]
		fn kill_name(origin, target: <T::Lookup as StaticLookup>::Source) {
			T::ForceOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)
				.map_err(|_| "bad origin")?;

			// Figure out who we're meant to be clearing.
			let target = T::Lookup::lookup(target)?;
			// Grab their deposit (and check that they have one).
			let deposit = <NameOf<T>>::take(&target).ok_or("Not named")?.1;
			// Slash their deposit from them.
			T::Slashed::on_unbalanced(T::Currency::slash_reserved(&target, deposit.clone()).0);

			Self::deposit_event(RawEvent::NameKilled(target, deposit));
		}

		/// Set a third-party account's name with no deposit.
		///
		/// No length checking is done on the name.
		///
		/// The dispatch origin for this call must be _Root_ or match `T::ForceOrigin`.
		///
		/// # <weight>
		/// - O(1).
		/// - At most one balance operation.
		/// - One storage read/write.
		/// - One event.
		/// # </weight>
		#[weight = SimpleDispatchInfo::FreeOperational]
		fn force_name(origin, target: <T::Lookup as StaticLookup>::Source, name: Vec<u8>) {
			T::ForceOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)
				.map_err(|_| "bad origin")?;

			let target = T::Lookup::lookup(target)?;
			let deposit = <NameOf<T>>::get(&target).map(|x| x.1).unwrap_or_else(Zero::zero);
			<NameOf<T>>::insert(&target, (name, deposit));

			Self::deposit_event(RawEvent::NameForced(target));
		}

		/// Set admin owner for this module
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_root_owner(origin, owner: T::AccountId) -> Result {
			T::ForceOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)
				.map_err(|_| "bad origin")?;

			let node_hash: T::Hash = T::Hash::default();
			Self::do_set_owner(node_hash, &owner)?;
			Self::deposit_event(RawEvent::RootChanged(owner));

			Ok(())
		}

		/// Transfer ownership of a node to a new address. May only be called
		/// by the current owner of the node
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_owner(origin, node_hash: T::Hash, owner: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			Self::only_owner(node_hash, &sender)?;

			let mut record = Self::node_of(node_hash).unwrap();
			ensure!(record.owner != owner, "owner is the same account");
			record.owner = owner.clone();

			<NodeOf<T>>::insert(node_hash, record);
			Self::deposit_event(RawEvent::Transfer(node_hash, owner));
			Ok(())
		}

		/// Transfer ownership of a subnode sha3(node, label) to a new address. May only be called
		/// by the current owner of the parent node
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_subnode_owner(origin, node_hash: T::Hash, label: T::Hash, owner: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			Self::only_owner(node_hash, &sender)?;

			let subnode_hash = (
				node_hash,
				label,
			).using_encoded(<T as system::Trait>::Hashing::hash);

			Self::do_set_owner(subnode_hash, &owner)?;
			Self::deposit_event(RawEvent::NewOwner(node_hash, label, owner));
			Ok(())
		}

		/// Set the TTL for the specified node
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_ttl(origin, node_hash: T::Hash, ttl: u64) -> Result {
			let sender = ensure_signed(origin)?;
			Self::only_owner(node_hash, &sender)?;

			let mut record = Self::node_of(node_hash).unwrap();
			ensure!(record.ttl != ttl, "ttl is the same value");
			record.ttl = ttl;

			<NodeOf<T>>::insert(node_hash, record);
			Self::deposit_event(RawEvent::NewTTL(node_hash, ttl));

			Ok(())
		}

		/// Set the resolve addr for the node
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_resolve_addr(origin, node_hash: T::Hash, addr: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			Self::only_owner(node_hash, &sender)?;
			
			Self::do_set_resolve_addr(node_hash, &addr)?;
			Self::deposit_event(RawEvent::ResolveAddrChanged(node_hash, addr));

			Ok(())
		}	

		/// Set the resolve name for the node
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_resolve_name(origin, node_hash: T::Hash, name: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			Self::only_owner(node_hash, &sender)?;

			ensure!(name.len() >= T::MinLength::get(), "name too short");
			ensure!(name.len() <= T::MaxLength::get(), "name too long");
			
			Self::do_set_resolve_name(node_hash, &name)?;
			Self::deposit_event(RawEvent::ResolveNameChanged(node_hash, name));

			Ok(())	
		}

		/// Set the resolve profile for the node
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_resolve_profile(origin, node_hash: T::Hash, profile: T::Hash) -> Result {
			let sender = ensure_signed(origin)?;
			Self::only_owner(node_hash, &sender)?;
			
			Self::do_set_resolve_profile(node_hash, profile)?;
			Self::deposit_event(RawEvent::ResolveProfileChanged(node_hash, profile));

			Ok(())
		}	

		/// Set the resolve zone content for the node
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_resolve_zone(origin, node_hash: T::Hash, zone: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			Self::only_owner(node_hash, &sender)?;

			ensure!(zone.len() <= T::MaxZoneLength::get(), "zone content too long");
			// TODO: check json
			// #[cfg(std)]
			// let _json = serde_json::from_str::<Value>(&String::from_utf8(zone.clone()).unwrap());

			// #[cfg(no_std)]
			// let _json = serde_json_core::from_str::<Value>(&String::from_utf8(zone.clone()).unwrap());
			
			Self::do_set_resolve_zone(node_hash, &zone)?;
			Self::deposit_event(RawEvent::ResolveZoneChanged(node_hash, zone));

			Ok(())	
		}
	}
}

impl<T: Trait> Module<T> {
	/// Check if the sender is the current owner of the node
	///
	/// @node_hash	the node hash
	/// @sender	the sender
	fn only_owner(node_hash: T::Hash, sender: &T::AccountId) -> Result {
		if let Some(record) = Self::node_of(node_hash) {
			ensure!(record.owner == *sender, "sender is not owner");
			Ok(())
		} else {
			Err("node does not exist")
		}
	}

	/// Set owner of the node
	///
	/// @node_hash 	the node hash to be set
	/// @owner	the owner account
	fn do_set_owner(node_hash: T::Hash, owner: &T::AccountId) -> Result {
		let mut record = if let Some(record) = Self::node_of(node_hash) {
			ensure!(record.owner != *owner, "owner is the same account");
			record
		} else {
			NodeRecord::<T::AccountId>::default()
		};

		record.owner = owner.clone();
		<NodeOf<T>>::insert(node_hash, record);

		Ok(())
	}

	/// Set resolve addr for the node
	///
	/// @node_hash 	the node hash to be set
	/// @addr	the resolve addr
	fn do_set_resolve_addr(node_hash: T::Hash, addr: &T::AccountId) -> Result {
		let mut record = if let Some(record) = Self::resolve_of(node_hash) {
			ensure!(record.addr != *addr, "addr is the same value");
			record
		} else {
			ResolveRecord::<T::Hash, T::AccountId>::default()
		};

		record.addr = addr.clone();
		<ResolveOf<T>>::insert(node_hash, record);

		Ok(())
	}

	/// Set resolve name for the node
	///
	/// @node_hash 	the node hash to be set
	/// @name	the resolve name
	fn do_set_resolve_name(node_hash: T::Hash, name: &Vec<u8>) -> Result {
		let mut record = if let Some(record) = Self::resolve_of(node_hash) {
			ensure!(record.name != *name, "name is the same value");
			record
		} else {
			ResolveRecord::<T::Hash, T::AccountId>::default()
		};

		record.name = name.clone();
		<ResolveOf<T>>::insert(node_hash, record);

		Ok(())
	}

	/// Set resolve profile for the node
	///
	/// @node_hash 	the node hash to be set
	/// @profile	the resolve profile
	fn do_set_resolve_profile(node_hash: T::Hash, profile: T::Hash) -> Result {
		let mut record = if let Some(record) = Self::resolve_of(node_hash) {
			ensure!(record.profile != profile, "profile is the same value");
			record
		} else {
			ResolveRecord::<T::Hash, T::AccountId>::default()
		};

		record.profile = profile;
		<ResolveOf<T>>::insert(node_hash, record);

		Ok(())
	}

	/// Set resolve zone content for the node
	///
	/// @node_hash 	the node hash to be set
	/// @zone	the resolve zone content
	fn do_set_resolve_zone(node_hash: T::Hash, zone: &Vec<u8>) -> Result {
		let mut record = if let Some(record) = Self::resolve_of(node_hash) {
			ensure!(record.zone != *zone, "zone is the same value");
			record
		} else {
			ResolveRecord::<T::Hash, T::AccountId>::default()
		};

		record.zone = zone.clone();
		<ResolveOf<T>>::insert(node_hash, record);

		Ok(())
	}
}

/// Client module should use this trait to communicate with the name service module
pub trait NameServiceResolver<T: system::Trait> {
	/// Resolve to record
	fn resolve(node_hash: T::Hash) -> Option<ResolveRecord<T::Hash, T::AccountId>> { None }
	/// Resolve to addr
	fn resolve_addr(node_hash: T::Hash) -> Option<T::AccountId> { None }
	/// Resolve to name
	fn resolve_name(node_hash: T::Hash) -> Option<Vec<u8>> { None }
	/// Resolve to profile hash
	fn resolve_profile(node_hash: T::Hash) -> Option<T::Hash> { None }
	/// Resolve to zone content
	fn resolve_zone(node_hash: T::Hash) -> Option<Vec<u8>> { None }
}

impl <T: Trait> NameServiceResolver<T> for Module<T> {
	/// Resolve name hash to record
	fn resolve(node_hash: T::Hash) -> Option<ResolveRecord<T::Hash, T::AccountId>> {
		Self::resolve_of(node_hash)
	}

	/// Resolve name hash to addr
	fn resolve_addr(node_hash: T::Hash) -> Option<T::AccountId> {
		match Self::resolve_of(node_hash) {
			Some(record) => Some(record.addr),
			None => None,
		}
	}

	/// Resolve name hash to name
	fn resolve_name(node_hash: T::Hash) -> Option<Vec<u8>> {
		match Self::resolve_of(node_hash) {
			Some(record) => Some(record.name),
			None => None,
		}
	}

	/// Resolve name hash to profile
	fn resolve_profile(node_hash: T::Hash) -> Option<T::Hash> {
		match Self::resolve_of(node_hash) {
			Some(record) => Some(record.profile),
			None => None,
		}
	}

	/// Resolve name hash to zone content
	fn resolve_zone(node_hash: T::Hash) -> Option<Vec<u8>> {
		match Self::resolve_of(node_hash) {
			Some(record) => Some(record.zone),
			None => None,
		}
	}	
}

