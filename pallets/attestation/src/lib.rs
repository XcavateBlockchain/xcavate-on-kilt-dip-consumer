// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2024 BOTLabs GmbH

// The KILT Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The KILT Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@botlabs.org

//! # Attestation Pallet
//!
//! Provides means of adding KILT attestations on chain and revoking them.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ### Terminology
//!
//! - **Claimer:**: A user which claims properties about themselves in the
//!   format of a CType. This could be a person which claims to have a valid
//!   driver's license.
//!
//! - **Attester:**: An entity which checks a user's claim and approves its
//!   validity. This could be a Citizens Registration Office which issues
//!   drivers licenses.
//!
//! - **Verifier:**: An entity which wants to check a user's claim by checking
//!   the provided attestation.
//!
//! - **CType:**: CTypes are claim types. In everyday language, they are
//!   standardised structures for credentials. For example, a company may need a
//!   standard identification credential to identify workers that includes their
//!   full name, date of birth, access level and id number. Each of these are
//!   referred to as an attribute of a credential.
//!
//! - **Attestation:**: An approved or revoked user's claim in the format of a
//!   CType.
//!
//! - **Delegation:**: An attestation which is not issued by the attester
//!   directly but via a (chain of) delegations which entitle the delegated
//!   attester. This could be an employe of a company which is authorized to
//!   sign documents for their superiors.
//!
//! ## Assumptions
//!
//! - The claim which shall be attested is based on a CType and signed by the
//!   claimer.
//! - The Verifier trusts the Attester. Otherwise, the attestation is worthless
//!   for the Verifier

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

pub mod attestations;
pub mod default_weights;
pub mod migrations;

#[cfg(any(feature = "mock", test))]
pub mod mock;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(any(feature = "try-runtime", test))]
mod try_state;

mod access_control;
pub mod authorized_by;
#[cfg(test)]
mod tests;

pub use crate::{
	access_control::AttestationAccessControl, attestations::AttestationDetails, default_weights::WeightInfo, pallet::*,
};

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	use authorized_by::AuthorizedBy;
	use frame_support::{
		dispatch::{DispatchResult, DispatchResultWithPostInfo},
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, MutateHold},
			Get, StorageVersion,
		},
	};
	use frame_system::pallet_prelude::*;

	use ctype::CtypeHashOf;
	use kilt_support::{
		traits::{BalanceMigrationManager, CallSources, StorageDepositCollector},
		Deposit,
	};

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	/// Type of a claim hash.
	pub type ClaimHashOf<T> = <T as frame_system::Config>::Hash;

	/// Type of an attester identifier.
	pub type AttesterOf<T> = <T as Config>::AttesterId;

	/// Authorization id type
	pub(crate) type AuthorizationIdOf<T> = <T as Config>::AuthorizationId;

	pub(crate) type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

	pub(crate) type BalanceOf<T> = <<T as Config>::Currency as Inspect<AccountIdOf<T>>>::Balance;

	pub(crate) type CurrencyOf<T> = <T as Config>::Currency;

	pub(crate) type HoldReasonOf<T> = <T as Config>::RuntimeHoldReason;

	pub(crate) type BalanceMigrationManagerOf<T> = <T as Config>::BalanceMigrationManager;

	pub(crate) type AuthorizedByOf<T> = authorized_by::AuthorizedBy<AccountIdOf<T>, AttesterOf<T>>;

	pub type AttestationDetailsOf<T> =
		AttestationDetails<CtypeHashOf<T>, AttesterOf<T>, AuthorizationIdOf<T>, AccountIdOf<T>, BalanceOf<T>>;

	#[pallet::composite_enum]
	pub enum HoldReason {
		Deposit,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + ctype::Config {
		type EnsureOrigin: EnsureOrigin<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = <Self as Config>::OriginSuccess,
		>;
		type OriginSuccess: CallSources<AccountIdOf<Self>, AttesterOf<Self>>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;

		type RuntimeHoldReason: From<HoldReason>;

		/// The currency that is used to hold funds for each attestation.
		type Currency: MutateHold<AccountIdOf<Self>, Reason = HoldReasonOf<Self>>;

		/// The deposit that is required for storing an attestation.
		#[pallet::constant]
		type Deposit: Get<BalanceOf<Self>>;

		/// The maximum number of delegated attestations which can be made by
		/// the same delegation.
		#[pallet::constant]
		type MaxDelegatedAttestations: Get<u32>;

		type AttesterId: Parameter + MaxEncodedLen;

		type AuthorizationId: Parameter + MaxEncodedLen;

		type AccessControl: Parameter
			+ AttestationAccessControl<Self::AttesterId, Self::AuthorizationId, CtypeHashOf<Self>, ClaimHashOf<Self>>;

		/// Migration manager to handle new created entries
		type BalanceMigrationManager: BalanceMigrationManager<AccountIdOf<Self>, BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			crate::try_state::do_try_state::<T>()
		}
	}

	/// Attestations stored on chain.
	///
	/// It maps from a claim hash to the full attestation.
	#[pallet::storage]
	#[pallet::getter(fn attestations)]
	pub type Attestations<T> = StorageMap<_, Blake2_128Concat, ClaimHashOf<T>, AttestationDetailsOf<T>>;

	/// Delegated attestations stored on chain.
	///
	/// It maps from a delegation ID to a vector of claim hashes.
	#[pallet::storage]
	#[pallet::getter(fn external_attestations)]
	pub type ExternalAttestations<T> =
		StorageDoubleMap<_, Twox64Concat, AuthorizationIdOf<T>, Blake2_128Concat, ClaimHashOf<T>, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new attestation has been created.
		AttestationCreated {
			/// The DID which issued this attestation.
			attester: AttesterOf<T>,
			/// The claim hash of the attested credential.
			claim_hash: ClaimHashOf<T>,
			/// The ctype of the attested credential.
			ctype_hash: CtypeHashOf<T>,
			/// The authorization information. If this is available, it
			/// authorizes a group of attesters to manage this attestation.
			authorization: Option<AuthorizationIdOf<T>>,
		},
		/// An attestation has been revoked.
		AttestationRevoked {
			/// Who authorized the revocation of the attestation.
			authorized_by: AuthorizedByOf<T>,
			/// The attester who initially created the attestation.
			attester: AttesterOf<T>,
			/// The ctype of the attested credential.
			ctype_hash: CtypeHashOf<T>,
			/// The claim hash of the credential that is revoked.
			claim_hash: ClaimHashOf<T>,
		},
		/// An attestation has been removed.
		AttestationRemoved {
			/// Who authorized the deletion of the attestation.
			authorized_by: AuthorizedByOf<T>,
			/// The attester who initially created the attestation.
			attester: AttesterOf<T>,
			/// The ctype of the attested credential.
			ctype_hash: CtypeHashOf<T>,
			/// The claim hash of the credential for which the attestation entry
			/// was deleted.
			claim_hash: ClaimHashOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// There is already an attestation with the same claim hash stored on
		/// chain.
		AlreadyAttested,
		/// The attestation has already been revoked.
		AlreadyRevoked,
		/// No attestation on chain matching the claim hash.
		NotFound,
		/// The attestation CType does not match the CType specified in the
		/// delegation hierarchy root.
		CTypeMismatch,
		/// The call origin is not authorized to change the attestation.
		NotAuthorized,
		/// The maximum number of delegated attestations has already been
		/// reached for the corresponding delegation id such that another one
		/// cannot be added.
		MaxDelegatedAttestationsExceeded,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new attestation.
		///
		/// The attester can optionally provide a reference to an existing
		/// delegation that will be saved along with the attestation itself in
		/// the form of an attested delegation.
		///
		/// The referenced CType hash must already be present on chain.
		///
		/// If an optional delegation id is provided, the dispatch origin must
		/// be the owner of the delegation. Otherwise, it could be any
		/// `DelegationEntityId`.
		///
		/// Emits `AttestationCreated`.
		#[pallet::call_index(0)]
		#[pallet::weight(
			<T as pallet::Config>::WeightInfo::add()
			.saturating_add(authorization.as_ref().map(|ac| ac.can_attest_weight()).unwrap_or(Weight::zero()))
		)]
		pub fn add(
			origin: OriginFor<T>,
			claim_hash: ClaimHashOf<T>,
			ctype_hash: CtypeHashOf<T>,
			authorization: Option<T::AccessControl>,
		) -> DispatchResult {
			let source = <T as Config>::EnsureOrigin::ensure_origin(origin)?;
			let payer = source.sender();
			let who = source.subject();
			let deposit_amount = <T as Config>::Deposit::get();

			ensure!(
				ctype::Ctypes::<T>::contains_key(ctype_hash),
				ctype::Error::<T>::NotFound
			);
			ensure!(
				!Attestations::<T>::contains_key(claim_hash),
				Error::<T>::AlreadyAttested
			);

			// Check for validity of the delegation node if specified.
			authorization
				.as_ref()
				.map(|ac| ac.can_attest(&who, &ctype_hash, &claim_hash))
				.transpose()?;
			let authorization_id = authorization.as_ref().map(|ac| ac.authorization_id());

			let deposit = AttestationStorageDepositCollector::<T>::create_deposit(payer, deposit_amount)?;
			<T as Config>::BalanceMigrationManager::exclude_key_from_migration(&Attestations::<T>::hashed_key_for(
				claim_hash,
			));

			log::debug!("insert Attestation");

			Attestations::<T>::insert(
				claim_hash,
				AttestationDetails {
					ctype_hash,
					attester: who.clone(),
					authorization_id: authorization_id.clone(),
					revoked: false,
					deposit,
				},
			);
			if let Some(authorization_id) = &authorization_id {
				ExternalAttestations::<T>::insert(authorization_id, claim_hash, true);
			}

			Self::deposit_event(Event::AttestationCreated {
				attester: who,
				claim_hash,
				ctype_hash,
				authorization: authorization_id,
			});

			Ok(())
		}

		/// Revoke an existing attestation.
		///
		/// The revoker must be either the creator of the attestation being
		/// revoked or an entity that in the delegation tree is an ancestor of
		/// the attester, i.e., it was either the delegator of the attester or
		/// an ancestor thereof.
		///
		/// Emits `AttestationRevoked`.
		#[pallet::call_index(1)]
		#[pallet::weight(
			<T as pallet::Config>::WeightInfo::revoke()
			.saturating_add(authorization.as_ref().map(|ac| ac.can_revoke_weight()).unwrap_or(Weight::zero()))
		)]
		pub fn revoke(
			origin: OriginFor<T>,
			claim_hash: ClaimHashOf<T>,
			authorization: Option<T::AccessControl>,
		) -> DispatchResultWithPostInfo {
			let source = <T as Config>::EnsureOrigin::ensure_origin(origin)?;
			let who = source.subject();

			let attestation = Attestations::<T>::get(claim_hash).ok_or(Error::<T>::NotFound)?;
			let attester = attestation.attester.clone();

			ensure!(!attestation.revoked, Error::<T>::AlreadyRevoked);

			let authorized_by = if attester != who {
				let attestation_auth_id = attestation.authorization_id.as_ref().ok_or(Error::<T>::NotAuthorized)?;
				authorization.ok_or(Error::<T>::NotAuthorized)?.can_revoke(
					&who,
					&attestation.ctype_hash,
					&claim_hash,
					attestation_auth_id,
				)?;

				AuthorizedBy::Authorization(who)
			} else {
				AuthorizedBy::Attester(who)
			};

			log::debug!("revoking Attestation");
			Attestations::<T>::insert(
				claim_hash,
				AttestationDetails {
					revoked: true,
					..attestation
				},
			);

			Self::deposit_event(Event::AttestationRevoked {
				attester,
				authorized_by,
				ctype_hash: attestation.ctype_hash,
				claim_hash,
			});

			Ok(Some(<T as pallet::Config>::WeightInfo::revoke()).into())
		}

		/// Remove an attestation.
		///
		/// The origin must be either the creator of the attestation or an
		/// entity which is an ancestor of the attester in the delegation tree,
		/// i.e., it was either the delegator of the attester or an ancestor
		/// thereof.
		///
		/// Always emits `AttestationRemoved` and emits `AttestationRevoked`
		/// only if the attestation was not revoked yet.
		#[pallet::call_index(2)]
		#[pallet::weight(
			<T as pallet::Config>::WeightInfo::remove()
			.saturating_add(authorization.as_ref().map(|ac| ac.can_remove_weight()).unwrap_or(Weight::zero()))
		)]
		pub fn remove(
			origin: OriginFor<T>,
			claim_hash: ClaimHashOf<T>,
			authorization: Option<T::AccessControl>,
		) -> DispatchResultWithPostInfo {
			let source = <T as Config>::EnsureOrigin::ensure_origin(origin)?;
			let who = source.subject();

			let attestation = Attestations::<T>::get(claim_hash).ok_or(Error::<T>::NotFound)?;

			let authorized_by = if attestation.attester != who {
				let attestation_auth_id = attestation.authorization_id.as_ref().ok_or(Error::<T>::NotAuthorized)?;
				authorization.ok_or(Error::<T>::NotAuthorized)?.can_remove(
					&who,
					&attestation.ctype_hash,
					&claim_hash,
					attestation_auth_id,
				)?;
				AuthorizedBy::Authorization(who)
			} else {
				AuthorizedBy::Attester(who)
			};

			log::debug!("removing Attestation");

			Self::remove_attestation(authorized_by, attestation, claim_hash)?;

			Ok(Some(<T as pallet::Config>::WeightInfo::remove()).into())
		}

		/// Reclaim a storage deposit by removing an attestation
		///
		/// Always emits `AttestationRemoved` and emits `AttestationRevoked`
		/// only if the attestation was not revoked yet.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::reclaim_deposit())]
		pub fn reclaim_deposit(origin: OriginFor<T>, claim_hash: ClaimHashOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let attestation = Attestations::<T>::get(claim_hash).ok_or(Error::<T>::NotFound)?;

			ensure!(attestation.deposit.owner == who, Error::<T>::NotAuthorized);

			log::debug!("removing Attestation");

			Self::remove_attestation(AuthorizedBy::DepositOwner(who), attestation, claim_hash)?;

			Ok(())
		}

		/// Changes the deposit owner.
		///
		/// The balance that is reserved by the current deposit owner will be
		/// freed and balance of the new deposit owner will get reserved.
		///
		/// The subject of the call must be the attester who issues the
		/// attestation. The sender of the call will be the new deposit owner.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::change_deposit_owner())]
		pub fn change_deposit_owner(origin: OriginFor<T>, claim_hash: ClaimHashOf<T>) -> DispatchResult {
			let source = <T as Config>::EnsureOrigin::ensure_origin(origin)?;
			let subject = source.subject();
			let sender = source.sender();

			let attestation = Attestations::<T>::get(claim_hash).ok_or(Error::<T>::NotFound)?;
			ensure!(attestation.attester == subject, Error::<T>::NotAuthorized);

			AttestationStorageDepositCollector::<T>::change_deposit_owner::<BalanceMigrationManagerOf<T>>(
				&claim_hash,
				sender,
			)?;

			Ok(())
		}

		/// Updates the deposit amount to the current deposit rate.
		///
		/// The sender must be the deposit owner.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update_deposit())]
		pub fn update_deposit(origin: OriginFor<T>, claim_hash: ClaimHashOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let attestation = Attestations::<T>::get(claim_hash).ok_or(Error::<T>::NotFound)?;
			ensure!(attestation.deposit.owner == sender, Error::<T>::NotAuthorized);

			AttestationStorageDepositCollector::<T>::update_deposit::<BalanceMigrationManagerOf<T>>(&claim_hash)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn remove_attestation(
			authorized_by: AuthorizedByOf<T>,
			attestation: AttestationDetailsOf<T>,
			claim_hash: ClaimHashOf<T>,
		) -> DispatchResult {
			let is_key_migrated =
				<T as Config>::BalanceMigrationManager::is_key_migrated(&Attestations::<T>::hashed_key_for(claim_hash));
			if is_key_migrated {
				AttestationStorageDepositCollector::<T>::free_deposit(attestation.deposit)?;
			} else {
				<T as Config>::BalanceMigrationManager::release_reserved_deposit(
					&attestation.deposit.owner,
					&attestation.deposit.amount,
				)
			}

			Attestations::<T>::remove(claim_hash);
			if let Some(authorization_id) = &attestation.authorization_id {
				ExternalAttestations::<T>::remove(authorization_id, claim_hash);
			}
			if !attestation.revoked {
				Self::deposit_event(Event::AttestationRevoked {
					attester: attestation.attester.clone(),
					authorized_by: authorized_by.clone(),
					claim_hash,
					ctype_hash: attestation.ctype_hash,
				});
			}
			Self::deposit_event(Event::AttestationRemoved {
				attester: attestation.attester,
				authorized_by,
				claim_hash,
				ctype_hash: attestation.ctype_hash,
			});
			Ok(())
		}
	}

	pub(crate) struct AttestationStorageDepositCollector<T: Config>(PhantomData<T>);
	impl<T: Config> StorageDepositCollector<AccountIdOf<T>, ClaimHashOf<T>, T::RuntimeHoldReason>
		for AttestationStorageDepositCollector<T>
	{
		type Currency = <T as Config>::Currency;
		type Reason = HoldReason;

		fn reason() -> Self::Reason {
			HoldReason::Deposit
		}

		fn get_hashed_key(key: &ClaimHashOf<T>) -> Result<sp_std::vec::Vec<u8>, DispatchError> {
			Ok(Attestations::<T>::hashed_key_for(key))
		}

		fn deposit(
			key: &ClaimHashOf<T>,
		) -> Result<Deposit<AccountIdOf<T>, <Self::Currency as Inspect<AccountIdOf<T>>>::Balance>, DispatchError> {
			let attestation = Attestations::<T>::get(key).ok_or(Error::<T>::NotFound)?;
			Ok(attestation.deposit)
		}

		fn deposit_amount(_key: &ClaimHashOf<T>) -> <Self::Currency as Inspect<AccountIdOf<T>>>::Balance {
			T::Deposit::get()
		}

		fn store_deposit(
			key: &ClaimHashOf<T>,
			deposit: Deposit<AccountIdOf<T>, <Self::Currency as Inspect<AccountIdOf<T>>>::Balance>,
		) -> Result<(), DispatchError> {
			let attestation = Attestations::<T>::get(key).ok_or(Error::<T>::NotFound)?;
			Attestations::<T>::insert(key, AttestationDetails { deposit, ..attestation });

			Ok(())
		}
	}
}
