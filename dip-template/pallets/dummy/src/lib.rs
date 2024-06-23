

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet(dev_mode)]
pub mod pallet {

	use super::*;

	use frame_support::{
		pallet_prelude::{DispatchResult, *},
		traits::EnsureOrigin,
		BoundedVec,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Hash;
	use sp_std::fmt::Debug;

	use crate::{
		// post::{Comment, Post},
		// traits::GetUsername,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

//	pub type BoundedTextOf<T> = BoundedVec<u8, <T as Config>::MaxTextLength>;
	// pub type PostOf<T> = Post<<T as frame_system::Config>::Hash, BoundedTextOf<T>, <T as Config>::Username>;
	// pub type CommentOf<T> = Comment<<T as frame_system::Config>::Hash, BoundedTextOf<T>, <T as Config>::Username>;

	#[pallet::config]
	pub trait Config: frame_system::Config {
        type WeightInfo;
//		type MaxTextLength: Get<u32>;
//		type OriginCheck: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
//		type OriginSuccess: GetUsername<Username = Self::Username>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
//		type Username: Encode + Decode + TypeInfo + MaxEncodedLen + Clone + PartialEq + Debug + Default;
	}

	// #[pallet::storage]
	// #[pallet::getter(fn posts)]
	// pub type Posts<T> = StorageMap<_, Twox64Concat, <T as frame_system::Config>::Hash, PostOf<T>>;

	// #[pallet::storage]
	// #[pallet::getter(fn comments)]
	// pub type Comments<T> = StorageMap<_, Twox64Concat, <T as frame_system::Config>::Hash, CommentOf<T>>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        DummyEvent
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
	}
}
