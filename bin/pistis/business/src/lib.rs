//! # Business Module
//!
//! - [`name_service::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This module is for business registration and product records
//!


#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use primitives::H256;
use rstd::prelude::*;
use sp_runtime::traits::{EnsureOrigin, Hash, StaticLookup, Zero};
use support::{
	decl_event, decl_module, decl_storage,
	dispatch::Result,
	ensure,
	traits::{Currency, Get, OnUnbalanced, ReservableCurrency, Randomness},
	weights::SimpleDispatchInfo,
};
use system::{ensure_root, ensure_signed};
use name_service::NameServiceResolver;

#[cfg(test)]
mod business_test;

/// The business information
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Business<NameHash, AccountId, BlockNumber> {
	/// The creator
	pub creator: AccountId,
	/// The name hash of the owner
	pub owner: NameHash,
	/// The name of business
	pub name: Vec<u8>,
	/// The whitelist account
	pub whitelist: Vec<NameHash>,
	/// The expiration of business 
	pub expiration: BlockNumber,
}

/// The information of some a product, generally speaking
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct ProductInfo<Hash, AccountId, BlockNumber> {
	/// Creator account
	pub creator: AccountId,
	/// Creation time
	pub created_at: BlockNumber,
	/// Hash of data
	pub data_hash: Hash,
	/// Extra information
	pub extra: Vec<u8>, // JSON info for details
}

/// The product information
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Product<Hash, AccountId, BlockNumber> {
	/// Sequence ID of the record
	pub seq_id: Vec<u8>, 
	/// Product info array
	pub infos: Vec<ProductInfo<Hash, AccountId, BlockNumber>>,
}

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
type NegativeImbalanceOf<T> =
	<<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

type NameHash<T> = <T as system::Trait>::Hash;
type BusinessOf<T> = Business<NameHash<T>, <T as system::Trait>::AccountId, <T as system::Trait>::BlockNumber>;
type ProductOf<T> = Product<<T as system::Trait>::Hash, <T as system::Trait>::AccountId, <T as system::Trait>::BlockNumber>;
type ProductInfoOf<T> = ProductInfo<<T as system::Trait>::Hash, <T as system::Trait>::AccountId, <T as system::Trait>::BlockNumber>;

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

	/// The maximum length a zone may be
	type MaxZoneLength: Get<usize>;

	/// The scope's name hash 
	type ScopeNameHash: Get<<Self as system::Trait>::Hash>; 

	/// The maximum length a sequence id may be
	type MaxSeqIDLength: Get<usize>;
	
	/// The maximum length an extra info may be
	type MaxExtraLength: Get<usize>;

	/// The maximum info entries a product may have
	type MaxProductInfoCount: Get<usize>;

	type NameServiceResolver: NameServiceResolver<Self>; 
}

decl_storage! {
	trait Store for Module<T: Trait> as NameServiceModule {
		/// The lookup table for all the businesses
		Businesses get(business_of): map T::Hash => BusinessOf<T>;	
		/// The lookup table for all the product infos
		Products get(product_of):  map T::Hash => ProductOf<T>;
		/// The counting table for business
		ProductCount get(product_count): map T::Hash => u64;
		/// The lookup table for querying hash of product info with business and index
		BusinessProductIndex get(business_product_index): map (T::Hash, u64) => T::Hash;
		/// The nonce for hashing
		Nonce: u64;
	}
}

decl_event!(
	pub enum Event<T>
	where
		BlockNumber = <T as system::Trait>::BlockNumber,
		Hash = <T as system::Trait>::Hash,
		AccountId = <T as system::Trait>::AccountId,
	{
		/// Business created
		BusinessCreated(AccountId, Hash),
		/// Business expiration changed
		BusinessExpirationChanged(AccountId, Hash, BlockNumber),
		/// Bisiness whitelist changed
		BusinessWhitelistChanged(AccountId, Hash, Vec<Hash>),
		/// Product info created
		ProductCreated(AccountId, Hash, Vec<u8>, Hash),
		/// Product info appended
		ProductInfoAppended(AccountId, Hash, Vec<u8>, Hash),
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

		/// The scope's name hash
		const ScopeNameHash: T::Hash = T::ScopeNameHash::get();

		/// The maximum length a sequence id may be
		const MaxSeqIDLength: u32 = T::MaxSeqIDLength::get() as u32;
	
		/// The maximum length an extra info may be
		const MaxExtraLength: u32 = T::MaxExtraLength::get() as u32; 

		/// The maximum info entries a product may have
		const MaxProductInfoCount: u32 = T::MaxProductInfoCount::get() as u32;

		/// Create business 
		/// 
		/// @origin	the sender
		/// @owner	the hash of the owner name
		/// @name	the business name in utf8
		/// @expiration	the expiration height
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn create_business(origin, owner: NameHash<T>, name: Vec<u8>, expiration: T::BlockNumber) {
			let sender = ensure_signed(origin)?;
			// Check if sender has previledge
			Self::validate_authorization(&sender, T::ScopeNameHash::get())?;

			ensure!(name.len() >= T::MinLength::get(), "Name too short");
			ensure!(name.len() <= T::MaxLength::get(), "Name too long");

			Self::validate_expiration(expiration)?;

			// Generate hash for business
			let nonce = Nonce::get();
			let biz_hash = (
				<randomness_collective_flip::Module<T>>::random_seed(),
				&sender,
				owner, // TODO: add other fields
				nonce,
			).using_encoded(<T as system::Trait>::Hashing::hash);

			let business = BusinessOf::<T> {
				creator: sender.clone(),
				owner: owner, 
				name: name.clone(),
				whitelist: Vec::new(),
				expiration: expiration,
			};

			Self::insert_business(biz_hash, &business)?;
			Self::deposit_event(RawEvent::BusinessCreated(sender.clone(), biz_hash));
			// Change nonce value to introduce random value
			Nonce::mutate(|n| *n += 1);
		}

		/// Set expiration of business
		/// 
		/// @origin 	the sender
		/// @biz_hash	the business hash
		/// @expiration	the expiration height 
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn set_business_expiration(origin, biz_hash: T::Hash, expiration: T::BlockNumber) {
			let sender = ensure_signed(origin)?;
			Self::validate_authorization(&sender, T::ScopeNameHash::get())?;

			ensure!(<Businesses<T>>::exists(biz_hash), "Business does not exist");
			let mut business = Self::business_of(biz_hash);
			
			ensure!(business.expiration != expiration, "Same value");
			business.expiration = expiration;
			<Businesses<T>>::insert(biz_hash, business);

			Self::deposit_event(RawEvent::BusinessExpirationChanged(sender.clone(), biz_hash, expiration));	
		}

		/// Add a name hash to the whitelist for a business
		///
		/// @origin	the sender
		/// @biz_hash	the business hash
		/// @name_hash	the name hash of operator 
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn add_business_whitelist(origin, biz_hash: T::Hash, name_hash: NameHash<T>) {
			let sender = ensure_signed(origin)?;

			ensure!(<Businesses<T>>::exists(biz_hash), "Business does not exist");
			let mut business = Self::business_of(biz_hash);
			Self::validate_authorization(&sender, business.owner)?;

			ensure!(!business.whitelist.contains(&name_hash), "Already in the whitelist");
			business.whitelist.push(name_hash);
			let new_list = business.whitelist.clone();
			<Businesses<T>>::insert(biz_hash, business);

			Self::deposit_event(RawEvent::BusinessWhitelistChanged(sender.clone(), biz_hash, new_list));	
		}

		/// Remove a namehash from the whitelist for a business
		///
		/// @origin	the sender
		/// @biz_hash	the business hash
		/// @name_hash	the name hash of operator to be removed
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn remove_business_whitelist(origin, biz_hash: T::Hash, name_hash: NameHash<T>) {
			let sender = ensure_signed(origin)?;

			ensure!(<Businesses<T>>::exists(biz_hash), "Business does not exist");
			let mut business = Self::business_of(biz_hash);
			Self::validate_authorization(&sender, business.owner)?;

			ensure!(business.whitelist.contains(&name_hash), "Not in the whitelist");
			business.whitelist.retain(|o| o != &name_hash);
			let new_list = business.whitelist.clone();
			<Businesses<T>>::insert(biz_hash, business);

			Self::deposit_event(RawEvent::BusinessWhitelistChanged(sender.clone(), biz_hash, new_list));	
		}

		/// Create product for a business
		/// 
		/// @origin	the sender
		/// @name_hash	the name hash of the operator
		/// @biz_hash	the business hash
		/// @seq_id	the sequence id, should be unique within the business scope
		/// @data_hash	the data hash to be stored with the product
		/// @extra	the extra information, can be json string 
		#[weight = SimpleDispatchInfo::FixedNormal(150_000)]
		fn create_product(origin, name_hash: NameHash<T>, biz_hash: T::Hash, seq_id: Vec<u8>, data_hash: T::Hash, extra: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			Self::validate_authorization(&sender, name_hash)?;

			ensure!(<Businesses<T>>::exists(biz_hash), "Business does not exist");
			let business = Self::business_of(biz_hash);
			ensure!(business.whitelist.contains(&name_hash), "Not in the whitelist");
			
			Self::validate_expiration(business.expiration)?;
			ensure!(seq_id.len() <= T::MaxSeqIDLength::get(), "Sequence ID too long");
			ensure!(extra.len() <= T::MaxExtraLength::get(), "Extra info too long");
			// FIXME: what if the product hash collides?
			let product_hash = (
				biz_hash,
				seq_id.clone(),
			).using_encoded(<T as system::Trait>::Hashing::hash);

			let info = ProductInfoOf::<T> {
				creator: sender.clone(),
				created_at: Self::block_number(),
				data_hash: data_hash,
				extra: extra.clone(),
			};

			let product = ProductOf::<T> {
				seq_id: seq_id.clone(),
				infos: vec![info],
			};

			Self::insert_product(biz_hash, product_hash, &product)?;
			Self::deposit_event(RawEvent::ProductCreated(sender.clone(), biz_hash, seq_id.clone(), product_hash));	
		}

		/// Add product info for a business
		/// 
		/// @origin	the sender
		/// @name_hash	the name hash of the operator
		/// @biz_hash	the business hash
		/// @seq_id	the sequence id, should be unique within the business scope
		/// @data_hash	the data hash to be stored with the product
		/// @extra	the extra information, can be json string 
		#[weight = SimpleDispatchInfo::FixedNormal(50_000)]
		fn add_product_info(origin, name_hash: NameHash<T>, biz_hash: T::Hash, seq_id: Vec<u8>, data_hash: T::Hash, extra: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			Self::validate_authorization(&sender, name_hash)?;

			ensure!(<Businesses<T>>::exists(biz_hash), "Business does not exist");
			let business = Self::business_of(biz_hash);
			ensure!(business.whitelist.contains(&name_hash), "Not in the whitelist");
			
			Self::validate_expiration(business.expiration)?;
			ensure!(seq_id.len() <= T::MaxSeqIDLength::get(), "Sequence ID too long");
			ensure!(extra.len() <= T::MaxExtraLength::get(), "Extra info too long");
			// FIXME: what if the info hash collides?
			let product_hash = (
				biz_hash,
				seq_id.clone(),
			).using_encoded(<T as system::Trait>::Hashing::hash);

			let info = ProductInfoOf::<T> {
				creator: sender.clone(),
				created_at: Self::block_number(),
				data_hash: data_hash,
				extra: extra.clone(),
			};

			Self::append_product_info(product_hash, &seq_id, info)?;
			Self::deposit_event(RawEvent::ProductInfoAppended(sender.clone(), biz_hash, seq_id.clone(), product_hash));	
		}
		
	}
}

impl<T: Trait> Module<T> {

	/// Validate authorization by checking if the name hash is resolved to the sender
	/// 
	/// @sender	the sender
	/// @hash	the name hash 
	pub fn validate_authorization(sender: &T::AccountId, hash: NameHash<T>) -> Result {
		// Check the resolved address against sender
		ensure!(Some(sender.clone()) == T::NameServiceResolver::resolve_addr(hash), "Not authorized");
		Ok(())
	}

	/// Validate expiration 
	/// 
	/// @expiration	the expiration height at which business is expired
	pub fn validate_expiration(expiration: T::BlockNumber) -> Result {
		ensure!(Self::block_number() < expiration, "Expired");
		Ok(())
	}

	/// Insert business to the lookup table
	/// 
	/// @hash	the business hash
	/// @business	the business object
	pub fn insert_business(hash: T::Hash, business: &BusinessOf<T>) -> Result {
		ensure!(!<Businesses<T>>::exists(hash), "Business already exists");
		<Businesses<T>>::insert(hash, business);
			
		Ok(())
	}

	/// Insert product info to the lookup table
	/// 
	/// @biz_hash	the business hash
	/// @product_hash	the info hash
	/// @info	the product info
	pub fn insert_product(biz_hash: T::Hash, product_hash: T::Hash, info: &ProductOf<T>) -> Result {
		ensure!(!<Products<T>>::exists(product_hash), "Product already exists");

        let info_count = Self::product_count(biz_hash);
        let new_info_count = info_count
            .checked_add(1)
            .ok_or("Overflow adding a new product")?;

		ensure!(!<BusinessProductIndex<T>>::exists((biz_hash, info_count)), "Business product hash collides???");
		<Products<T>>::insert(product_hash, info);
		<BusinessProductIndex<T>>::insert((biz_hash, info_count), product_hash);
		<ProductCount<T>>::insert(biz_hash, new_info_count);
		
		Ok(())
	}

	/// Append product info to an existing product
	/// 
	/// @product_hash	the product hash
	/// @seq_id	the sequence id
	/// @info	the product info
	pub fn append_product_info(product_hash: T::Hash, seq_id: &Vec<u8>, info: ProductInfoOf<T>) -> Result {
		ensure!(<Products<T>>::exists(product_hash), "Product does not exist");

		let mut product = Self::product_of(product_hash);
		ensure!(product.seq_id == *seq_id, "Product sequence id not match, should not happen");
		ensure!(product.infos.len() < T::MaxProductInfoCount::get(), "Exceeds max product info limit");
		// Append the record to the end of collection
		product.infos.push(info);

		<Products<T>>::insert(product_hash, product);

		Ok(())
	}

	/// Get current block number
    fn block_number() -> T::BlockNumber {
        <system::Module<T>>::block_number()
    }
}
