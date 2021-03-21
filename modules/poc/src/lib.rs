//! # Proof of Commitment
//!
//! Stake tokens with extremely long unbonding period,
//! to obtain the Technical Council election voting rights.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, ReservableCurrency, IsType, WithdrawReasons, ExistenceRequirement},
	weights::Weight,
	ensure,
	transactional,
};
use frame_support::sp_runtime::traits::CheckedAdd;
use frame_system::pallet_prelude::*;

#[cfg(feature = "std")]
pub use serde::{Deserialize, Serialize};

mod mock;
mod tests;

pub use module::*;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq)]
pub enum LockState<BlockNumber> {
	Committed,
	Unbonding(BlockNumber),
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Debug, PartialEq)]
pub enum LockDuration {
	OneMonth,
	OneYear,
	TenYears,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default)]
pub struct Commitment<AccountId, BalanceOf, BlockNumber> {
	pub state: LockState<BlockNumber>,
	pub duration: LockDuration,
	pub amount: BalanceOf,
	pub candidate: AccountId,
}

impl<BlockNumber> Default for LockState<BlockNumber> {
	fn default() -> Self {
		Self::Committed
	}
}

impl Default for LockDuration {
	fn default() -> Self {
		Self::OneMonth
	}
}


// config
// % of total supply that can be locked

// voting
// only LockState::Committed funds can vote
// lock duration multiplier

// council
// how does council get paid?
// membership pallet?
// how often are elections?

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type CommitmentOf<T> =
	Commitment<
		<T as frame_system::Config>::AccountId,
		BalanceOf<T>,
		<T as frame_system::Config>::BlockNumber,
	>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account already has an active commitment
		AlreadyCommitted,
		/// Cannot operate on a non existing commitment
		CommitmentNotFound,
		/// The commitment is not active
		NotCommitted,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// Created a new committment
		Committed(T::AccountId),
		/// Add more funds to existing commitment
		FundsAdded(T::AccountId),
	}

	#[pallet::storage]
	#[pallet::getter(fn commitments)]
	pub(crate) type Commitments<T: Config> = StorageMap<_,
		Blake2_128Concat, T::AccountId, CommitmentOf<T>, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_finalize(_n: T::BlockNumber) {
			// TODO set winners at the end of era
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(10_000)]
		pub fn commit(
			origin: OriginFor<T>,
			#[pallet::compact] amount: BalanceOf<T>,
			duration: LockDuration,
			candidate: T::AccountId,
			) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;

			ensure!(!<Commitments<T>>::contains_key(&origin), Error::<T>::AlreadyCommitted);

			// TODO check if at totalSupply capacity

			T::Currency::withdraw(
				&origin, amount,
				WithdrawReasons::RESERVE,
				ExistenceRequirement::KeepAlive)?;

			// create a new commitment
			<Commitments<T>>::insert(&origin, Commitment {
				amount: amount,
				duration: duration,
				candidate: candidate,
				..Default::default()
			});
			Self::deposit_event(Event::Committed(origin));
			Ok(().into())
		}


		#[pallet::weight(10_000)]
		pub fn add_funds(origin: OriginFor<T>, #[pallet::compact] amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;

			ensure!(<Commitments<T>>::contains_key(&origin), Error::<T>::CommitmentNotFound);
			let mut commitment = <Commitments<T>>::get(&origin);

			// TODO check if at totalSupply capacity

			T::Currency::withdraw(
				&origin, amount,
				WithdrawReasons::RESERVE,
				ExistenceRequirement::KeepAlive)?;
			commitment.amount = commitment.amount.checked_add(&amount).ok_or("currency overflow")?;

			// always re-commit
			commitment.state = LockState::Committed;

			// save the commitment
			<Commitments<T>>::insert(&origin, commitment);


			Ok(().into())
		}


		#[pallet::weight(10_000)]
		pub fn unbond(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;

			ensure!(<Commitments<T>>::contains_key(&origin), Error::<T>::CommitmentNotFound);
			let mut commitment = <Commitments<T>>::get(&origin);
			ensure!(commitment.state == LockState::Committed, Error::<T>::NotCommitted);

			// TODO
			// calculate block number from lock duration

			<Commitments<T>>::insert(&origin, commitment);
			Ok(().into())
		}


		#[pallet::weight(10_000)]
		pub fn withdraw(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;

			ensure!(<Commitments<T>>::contains_key(&origin), Error::<T>::CommitmentNotFound);
			let commitment = <Commitments<T>>::get(&origin);

			// TODO check if Unbonding period is over

			// credit the user his funds

			// delete the commitment
			<Commitments<T>>::remove(&origin);

			Ok(().into())
		}


		#[pallet::weight(10_000)]
		pub fn set_candidate(
			origin: OriginFor<T>,
			candidate: T::AccountId,
			) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;

			ensure!(<Commitments<T>>::contains_key(&origin), Error::<T>::CommitmentNotFound);
			let mut commitment = <Commitments<T>>::get(&origin);
			ensure!(commitment.state == LockState::Committed, Error::<T>::NotCommitted);

			commitment.candidate = candidate;
			<Commitments<T>>::insert(&origin, commitment);

			Ok(().into())
		}

		// TODO claim reward


	}
}

impl<T: Config> Pallet<T> {
	pub fn voting_weight(commitment: &Commitment<T::AccountId, BalanceOf<T>, T::BlockNumber>) -> BalanceOf<T> {
		if commitment.state != LockState::Committed {
			return BalanceOf::<T>::from(0 as u32);
		}
		let multiplier = match commitment.duration {
			LockDuration::OneMonth => 1,
			LockDuration::OneYear  => 10,
			LockDuration::TenYears => 100,
		};
		commitment.amount * BalanceOf::<T>::from(multiplier as u32)
	}
}
