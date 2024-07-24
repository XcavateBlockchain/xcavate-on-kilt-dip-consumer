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

//! Runtime template of a Decentralized Identity Provider (DIP) consumer, which
//! does not itself include any identity-related pallets, but only the
//! [`pallet_dip_consumer::Pallet`] pallet (configured to work with the
//! [`dip_provider_runtime_template::Runtime`] template runtime), the
//! [`pallet_relay_store::Pallet`] pallet to keep track of finalized relaychain
//! state roots, and the example [`pallet_postit::Pallet`], which allows any
//! entity that can be identified with a username (e.g., a web3name carried over
//! from the provider chain) to post a message on chain, reply to another
//! on-chain message (including another reply), or like a message and/or any of
//! its replies.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod dip;
mod origin_adapter;
mod weights;
pub use crate::{dip::*, origin_adapter::*};

/// Constant values used within the runtimezz
pub mod constants;
pub mod impls;
mod voter_bags;

use parity_scale_codec as codec;
use sp_runtime::codec::Decode;
use constants::{currency::*, time::*};
use frame_election_provider_support::{onchain, ExtendedBalance, SequentialPhragmen, VoteWeight};
use frame_support::genesis_builder_helper::{build_config, create_default_config};
use frame_support::{
	instances::{Instance1, Instance2},
	ord_parameter_types,
	pallet_prelude::{DispatchClass, Get},
	traits::{
//		fungible::HoldConsideration,
		tokens::{PayFromAccount, 
//			UnityAssetBalanceConversion
		},
		AsEnsureOriginWithArg, EitherOfDiverse, EqualPrivilegeOnly,
		// LinearStoragePrice,
	},
	PalletId,
};
use sp_staking::currency_to_vote::U128CurrencyToVote;
use frame_system::{EnsureSigned, EnsureSignedBy,EnsureWithSuccess,};
use polkadot_primitives::Nonce;
use node_primitives::Moment;
use pallet_grandpa::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
use crate::opaque::SessionKeys;
use sp_runtime::{
	create_runtime_str,
	curve::PiecewiseLinear,
	generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, Convert, ConvertInto,
		IdentifyAccount, NumberFor, One, OpaqueKeys, Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128, MultiSignature, Percent,
};
use impls::CreditToBlockAuthor;

// A few exports that help ease life for downstream crates.
use frame_election_provider_support::bounds::{ElectionBounds, ElectionBoundsBuilder};
pub use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{
		ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem, Randomness, StorageInfo,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight,
	},
	BoundedVec, StorageValue,
};

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
use pallet_election_provider_multi_phase::{
//	GeometricDepositBase, 
	SolutionAccuracyOf};
/// Import the nft pallet
use pallet_nfts::PalletFeatures;
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
use sp_runtime::traits::IdentityLookup;

use dip_provider_runtime_template::Web3Name;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use sp_runtime::{MultiAddress, Perbill, Permill};

use cumulus_pallet_parachain_system::{ParachainSetCode, RelayNumberMonotonicallyIncreases};
use cumulus_primitives_core::CollationInfo;
use frame_support::traits::Everything;
use frame_system::{
	limits::{BlockLength, BlockWeights},
	ChainContext, EnsureRoot,
};
use pallet_balances::AccountData;
use pallet_collator_selection::IdentityCollator;
use pallet_session::{FindAccountFromAuthorIndex, PeriodicSessions};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::SlotDuration;
use sp_core::{crypto::KeyTypeId, ConstBool, ConstU16, OpaqueMetadata};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
		AccountId32, OpaqueExtrinsic,
};
use sp_std::prelude::*;
use sp_version::RuntimeVersion;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

#[cfg(feature = "std")]
use sp_version::NativeVersion;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
//pub type AccountId = AccountId32;

pub type Address = MultiAddress<AccountId, ()>;
pub type Balance = u128;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type BlockNumber = u64;
pub type DidIdentifier = AccountId;
pub type Hasher = BlakeTwo256;
pub type Hash = sp_core::H256;
pub type Header = generic::Header<BlockNumber, Hasher>;
//pub type Nonce = u64;
pub type Signature = MultiSignature;
/// Index of a transaction in the chain.
pub type Index = u32;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			// pub aura: Aura,
			pub aura: Aura,
			pub grandpa: Grandpa,
 			pub im_online: ImOnline,
			pub authority_discovery: AuthorityDiscovery,
		}
	}
}

pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
pub type Executive = frame_executive::Executive<Runtime, Block, ChainContext<Runtime>, Runtime, AllPalletsWithSystem>;
pub type NodeBlock = generic::Block<Header, OpaqueExtrinsic>;
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

pub const MILLISECS_PER_BLOCK: u64 = 12000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;

pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLIUNIT: Balance = UNIT / 1_000;

// Charge fee for stored bytes and items
pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 100 * MILLICENTS * INIT_SUPPLY_FACTOR + (bytes as Balance) * STORAGE_BYTE_FEE
}

construct_runtime!(
	pub struct Runtime
	{
		// System
		System: frame_system,
		ParachainSystem: cumulus_pallet_parachain_system,
		Timestamp: pallet_timestamp,
		ParachainInfo: parachain_info,
		Sudo: pallet_sudo,
		Utility: pallet_utility,

		// Money
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,

		// Collators
		Authorship: pallet_authorship,
		CollatorSelection: pallet_collator_selection,
		Session: pallet_session,
		Aura: pallet_aura,
		AuraExt: cumulus_pallet_aura_ext,

		// PostIt
		PostIt: pallet_postit,

		// DIP
		DipConsumer: pallet_dip_consumer,
		RelayStore: pallet_relay_store,

		// Xcavate dependeny pallets
		Nfts: pallet_nfts,
		Uniques: pallet_uniques, //10
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Assets: pallet_assets::<Instance1>,
		PoolAssets: pallet_assets::<Instance2>,
		Multisig: pallet_multisig,
		Treasury: pallet_treasury,
// //		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>},
		Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>},
		ChildBounties: pallet_child_bounties::{Pallet, Call, Storage, Event<T>},
		Babe: pallet_babe,
// //		Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned},
		Historical: pallet_session_historical::{Pallet},
		Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
		BagsList: pallet_bags_list::{Pallet, Call, Storage, Event<T>},
		NominationPools: pallet_nomination_pools::{Pallet, Call, Storage, Event<T>, Config<T>},
// 		NominationPools: pallet_nomination_pools::{Pallet, Call, Storage, Event<T>, Config<T>, FreezeReason},
		Offences: pallet_offences::{Pallet, Storage, Event},
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
		Council: pallet_collective::<Instance1>,
		TechnicalCommittee: pallet_collective::<Instance2>,
		AllianceMotion: pallet_collective::<Instance3>,
		AuthorityDiscovery: pallet_authority_discovery,
		AssetTxPayment: pallet_asset_tx_payment,
		Scheduler: pallet_scheduler,
		Preimage: pallet_preimage,
		Democracy: pallet_democracy,
		NftFractionalization: pallet_nft_fractionalization,

// 		// Xcavate
 		Grandpa: pallet_grandpa,
 		XcavateWhitelist: pallet_xcavate_whitelist,
//		Dummy: pallet_dummy,
 		CommunityLoanPool: pallet_community_loan_pool,
 		XcavateStaking: pallet_xcavate_staking,
 		NftMarketplace: pallet_nft_marketplace,
 		CommunityProject: pallet_community_projects,
 		PropertyManagement: pallet_property_management,
 		PropertyGovernance: pallet_property_governance,
	}
);

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("dip-consumer-runtime-template"),
	impl_name: create_runtime_str!("dip-consumer-runtime-template"),
	authoring_version: 1,
	spec_version: 11400,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration = 
	sp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
	};

#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}

const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
	cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
	BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
	.base_block(BlockExecutionWeight::get())
	.for_class(DispatchClass::all(), |weights| {
		weights.base_extrinsic = ExtrinsicBaseWeight::get();
	})
	.for_class(DispatchClass::Normal, |weights| {
		weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
	})
	.for_class(DispatchClass::Operational, |weights| {
		weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
		weights.reserved = Some(
			MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
		);
	})
	.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
	.build_or_panic();
}

pub const SS58_PREFIX: u16 = 101;

impl frame_system::Config for Runtime {
	type AccountData = AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = Everything;
	type BlockHashCount = ConstU64<256>;
	type BlockLength = RuntimeBlockLength;
	type Block = Block;
	type BlockWeights = RuntimeBlockWeights;
	type DbWeight = RocksDbWeight;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type MaxConsumers = ConstU32<16>;
	type Nonce = u64;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ParachainSetCode<Self>;
	type PalletInfo = PalletInfo;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SS58Prefix = ConstU16<SS58_PREFIX>;
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type Version = Version;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included into the
/// relay chain.
const UNINCLUDED_SEGMENT_CAPACITY: u32 = 1;
/// How many parachain blocks are processed by the relay chain per parent. Limits the number of
/// blocks authored per slot.
const BLOCK_PROCESSING_VELOCITY: u32 = 1;
/// Relay chain slot duration, in milliseconds.
const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;

impl cumulus_pallet_parachain_system::Config for Runtime {
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type DmpMessageHandler = ();
	type OnSystemEvent = ();
	type OutboundXcmpMessageSource = ();
	type ReservedDmpWeight = ();
	type ReservedXcmpWeight = ();
	type RuntimeEvent = RuntimeEvent;
	type SelfParaId = ParachainInfo;
	type XcmpMessageHandler = ();
	type ConsensusHook = ConsensusHook;
}

impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = ConstU64<{ MILLISECS_PER_BLOCK / 2 }>;
	type Moment = u64;
	type OnTimestampSet = Aura;
	type WeightInfo = ();
}

impl parachain_info::Config for Runtime {}

// impl pallet_sudo::Config for Runtime {
// 	type RuntimeCall = RuntimeCall;
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightInfo = ();
// }

// impl pallet_utility::Config for Runtime {
// 	type PalletsOrigin = OriginCaller;
// 	type RuntimeCall = RuntimeCall;
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightInfo = ();
// }

pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

// impl pallet_balances::Config for Runtime {
// 	type AccountStore = System;
// 	type Balance = Balance;
// 	type DustRemoval = ();
// 	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
// 	type FreezeIdentifier = RuntimeFreezeReason;
// 	type MaxFreezes = ConstU32<50>;
// 	type MaxHolds = ConstU32<50>;
// 	type MaxLocks = ConstU32<50>;
// 	type MaxReserves = ConstU32<50>;
// 	type ReserveIdentifier = [u8; 8];
// 	type RuntimeEvent = RuntimeEvent;
// 	type RuntimeHoldReason = RuntimeHoldReason;
// 	type WeightInfo = ();
// }

// impl pallet_transaction_payment::Config for Runtime {
// 	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
// 	type FeeMultiplierUpdate = ();
// 	type LengthToFee = IdentityFee<Balance>;
// 	type OperationalFeeMultiplier = ConstU8<1>;
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightToFee = IdentityFee<Balance>;
// }

// impl pallet_authorship::Config for Runtime {
// 	type EventHandler = (CollatorSelection,);
// 	type FindAuthor = FindAccountFromAuthorIndex<Self, Aura>;
// }

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
}

impl pallet_collator_selection::Config for Runtime {
	type Currency = Balances;
	type PotId = PotId;
	type KickThreshold = ConstU64<{ 6 * HOURS }>;
	type MaxCandidates = ConstU32<1_000>;
	type MaxInvulnerables = ConstU32<100>;
	type MinEligibleCollators = ConstU32<5>;
	type RuntimeEvent = RuntimeEvent;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type ValidatorId = AccountId;
	type ValidatorIdOf = IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = ();
}

// impl_opaque_keys! {
// 	pub struct SessionKeys {
// 		pub aura: Aura,
// 		pub grandpa: Grandpa,
// 		pub im_online: ImOnline,
// 	    pub authority_discovery: AuthorityDiscovery,
// 	}
// }

// impl pallet_session::Config for Runtime {
// 	type Keys = SessionKeys;
// 	type NextSessionRotation = PeriodicSessions<ConstU64<HOURS>, ConstU64<0>>;
// 	type RuntimeEvent = RuntimeEvent;
// 	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
// 	type SessionManager = CollatorSelection;
// 	type ShouldEndSession = PeriodicSessions<ConstU64<HOURS>, ConstU64<0>>;
// 	type ValidatorId = AccountId;
// 	type ValidatorIdOf = IdentityCollator;
// 	type WeightInfo = ();
// }

impl pallet_aura::Config for Runtime {
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
	type MaxNominators = ConstU32<0>; // FIXME
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

parameter_types! {
	pub NposSolutionPriority: TransactionPriority =
		Perbill::from_percent(90) * TransactionPriority::max_value();
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorSet = Historical;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = ();
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

parameter_types! {
	pub MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
//	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type MaxHolds = ConstU32<1>;
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

impl pallet_asset_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Fungibles = Assets;
	type OnChargeAssetTransaction = pallet_asset_tx_payment::FungiblesAdapter<
		pallet_assets::BalanceToAssetBalance<Balances, Runtime, ConvertInto, Instance1>,
		CreditToBlockAuthor,
	>;
}

// impl pallet_asset_conversion_tx_payment::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type Fungibles = Assets;
// 	type OnChargeAssetTransaction =
// 		pallet_asset_conversion_tx_payment::AssetConversionAdapter<Balances, AssetConversion>;
// }

// parameter_types! {
// 	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
// 	pub AllowMultiAssetPools: bool = true;
// 	pub const PoolSetupFee: Balance = 1 * DOLLARS; // should be more or equal to the existential
// deposit 	pub const MintMinLiquidity: Balance = 100;  // 100 is good enough when the main currency
// has 10-12 decimals. 	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);  //
// should be non-zero if AllowMultiAssetPools is true, otherwise can be zero. }

// impl pallet_asset_conversion::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type Currency = Balances;
// 	type AssetBalance = <Self as pallet_balances::Config>::Balance;
// 	type HigherPrecisionBalance = sp_core::U256;
// 	type Assets = Assets;
// 	type Balance = u128;
// 	type PoolAssets = PoolAssets; // TODO:
// 	type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
// 	type MultiAssetId = NativeOrAssetId<u32>;
// 	type PoolAssetId = <Self as pallet_assets::Config<Instance2>>::AssetId;
// 	type PalletId = AssetConversionPalletId;
// 	type LPFee = ConstU32<3>; // means 0.3%
// 	type PoolSetupFee = PoolSetupFee;
// 	type PoolSetupFeeReceiver = AssetConversionOrigin;
// 	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
// 	type WeightInfo = pallet_asset_conversion::weights::SubstrateWeight<Runtime>;
// 	type AllowMultiAssetPools = AllowMultiAssetPools;
// 	type MaxSwapPathLength = ConstU32<4>;
// 	type MintMinLiquidity = MintMinLiquidity;
// 	type MultiAssetIdConverter = NativeOrAssetIdConverter<u32>;
// 	#[cfg(feature = "runtime-benchmarks")]
// 	type BenchmarkHelper = ();
// }

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>; // FIXME
}

parameter_types! {
	pub const CommunityLoanPalletId: PalletId = PalletId(*b"py/loana");
	pub const MaxLoans: u32 = 1000;
	pub const VotingTime: BlockNumber = 10;
	pub const MaximumCommitteeMembers: u32 = 10;
	pub const MaxMilestones: u32 = 10;
}

/// Configure the pallet-community-loan-pool in pallets/community-loan-pool.
impl pallet_community_loan_pool::Config for Runtime {
	type PalletId = CommunityLoanPalletId;
	type Currency = Balances;
	type CommitteeOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
	>;
	type RuntimeEvent = RuntimeEvent;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ();
	type OnSlash = ();
	type MaxOngoingLoans = MaxLoans;
	type TimeProvider = Timestamp;
	type WeightInfo = pallet_community_loan_pool::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = pallet_community_loan_pool::NftHelper;
	type VotingTime = VotingTime;
	type MaxCommitteeMembers = MaximumCommitteeMembers;
	type MaxMilestonesPerProject = MaxMilestones;
	type CollectionId = u32;
	type ItemId = u32;
}

parameter_types! {
	pub const MinimumRemainingAmount: Balance = DOLLARS;
	pub const MaxStaker: u32 = 5000;
	pub const RewardsDistributing: BlockNumber = 1;
}

/// Configure the pallet-xcavate-staking in pallets/xcavate-staking.
impl pallet_xcavate_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_xcavate_staking::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type MinimumRemainingAmount = MinimumRemainingAmount;
	type MaxStakers = MaxStaker;
	type TimeProvider = Timestamp;
	type RewardsDistributingTime = RewardsDistributing;
}

parameter_types! {
	pub const NftMarketplacePalletId: PalletId = PalletId(*b"py/nftxc");
	pub const MaxNftTokens: u32 = 100;
	pub const Postcode: u32 = 10;
}

/// Configure the pallet-nft-marketplace in pallets/nft-marketplace.
impl pallet_nft_marketplace::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_nft_marketplace::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type PalletId = NftMarketplacePalletId;
	type MaxNftToken = MaxNftTokens;
	type LocationOrigin = EnsureRoot<Self::AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = pallet_nft_marketplace::NftHelper;
	type CollectionId = u32;
	type ItemId = u32;
	type TreasuryId = TreasuryPalletId;
	type CommunityProjectsId = CommunityProjectPalletId;
 	type FractionalizeCollectionId = <Self as pallet_nfts::Config>::CollectionId;
	type FractionalizeItemId = <Self as pallet_nfts::Config>::ItemId;
	type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
	type AssetId2 = u32; 
	type PostcodeLimit = Postcode;
}

parameter_types! {
	pub const CommunityProjectPalletId: PalletId = PalletId(*b"py/cmprj");
	pub const MaxNftType: u32 = 4;
	pub const MaxNftsInCollectionProject: u32 = 100;
	pub const MaxOngoingProject: u32 = 250;
}

/// Configure the pallet-community-projects in pallets/community-projects.
impl pallet_community_projects::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_community_projects::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type PalletId = CommunityProjectPalletId;
	type MaxNftTypes = MaxNftType;
	type MaxNftInCollection = MaxNftsInCollectionProject;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = pallet_community_projects::NftHelper;
	type TimeProvider = Timestamp;
	type MaxOngoingProjects = MaxOngoingProject;
	type AssetId = u32;
	type CollectionId = u32;
	type ItemId = u32;
	type MinimumRemainingAmount = MinimumRemainingAmount;
}

parameter_types! {
	pub const MaxWhitelistUsers: u32 = 1000;
}

/// Configure the pallet-xcavate-whitelist in pallets/xcavate-whitelist.
impl pallet_xcavate_whitelist::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_xcavate_whitelist::weights::SubstrateWeight<Runtime>;
	type WhitelistOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
	>;
	type MaxUsersInWhitelist = MaxWhitelistUsers;
}

// impl pallet_dummy::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightInfo = ();
// //	type WeightInfo = pallet_xcavate_whitelist::weights::SubstrateWeight<Runtime>;
// }

parameter_types! {
	pub const MinimumStakingAmount: Balance = 100 * DOLLARS;
	pub const PropertyManagementPalletId: PalletId = PalletId(*b"py/ppmmt");
	pub const MaxProperty: u32 = 1000;
	pub const MaxLettingAgent: u32 = 100;
	pub const MaxLocation: u32 = 100;
}

/// Configure the pallet-property-management in pallets/property-management.
impl pallet_property_management::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_property_management::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type PalletId = PropertyManagementPalletId;
	type AgentOrigin = EnsureRoot<Self::AccountId>;
	type MinStakingAmount = MinimumStakingAmount;
	type MaxProperties = MaxProperty;
	type MaxLettingAgents = MaxLettingAgent;
	type MaxLocations = MaxLocation;
}

parameter_types! {
	pub const PropertyVotingTime: BlockNumber = 30;
	pub const MaxVoteForBlock: u32 = 100;
	pub const MinimumSlashingAmount: Balance = 10 * DOLLARS;
	pub const MaximumVoter: u32 = 100;
	pub const VotingThreshold: u8 = 67;
}

/// Configure the pallet-property-governance in pallets/property-governance.
impl pallet_property_governance::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_property_governance::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type VotingTime = PropertyVotingTime;
	type MaxVotesForBlock =  MaxVoteForBlock;
	type Slash = ();
	type MinSlashingAmount = MinimumSlashingAmount;
	type MaxVoter = MaximumVoter;
	type Threshold = VotingThreshold;
} 

parameter_types! {
	pub Features: PalletFeatures = PalletFeatures::all_enabled();
	pub const MaxAttributesPerCall: u32 = 10;
	pub const CollectionDeposit: Balance = DOLLARS;
	pub const ItemDeposit: Balance = DOLLARS;
	pub const MetadataDepositBase: Balance = DOLLARS;
	pub const MetadataDepositPerByte: Balance = DOLLARS / 100;
	pub const StringLimit: u32 = 5000;
	pub const KeyLimit: u32 = 32;
	pub const ValueLimit: u32 = 256;
	pub const ApprovalsLimit: u32 = 20;
	pub const ItemAttributesApprovalsLimit: u32 = 20;
	pub const MaxTips: u32 = 10;
	pub const MaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS as u64;

	pub const UserStringLimit: u32 = 5;

}

ord_parameter_types! {
	pub const CollectionCreationOrigin: AccountId = AccountIdConversion::<AccountId>::into_account_truncating(&CommunityLoanPalletId::get());
}

impl pallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type ApprovalsLimit = ApprovalsLimit;
	type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
	type MaxTips = MaxTips;
	type MaxDeadlineDuration = MaxDeadlineDuration;
	type MaxAttributesPerCall = MaxAttributesPerCall;
	type Features = Features;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as Verify>::Signer;
	type WeightInfo = pallet_nfts::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	//type CreateOrigin = AsEnsureOriginWithArg<EnsureSignedBy<CollectionCreationOrigin, AccountId>>;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
}

impl pallet_uniques::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type WeightInfo = pallet_uniques::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
}

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub const AssetDeposit: Balance = 100 * DOLLARS;
	pub const ApprovalDeposit: Balance = DOLLARS;

}

impl pallet_assets::Config<Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	type RemoveItemsLimit = ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

ord_parameter_types! {
	pub const AssetConversionOrigin: AccountId = AccountIdConversion::<AccountId>::into_account_truncating(&AssetConversionPalletId::get());
}

impl pallet_assets::Config<Instance2> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSignedBy<AssetConversionOrigin, AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	type RemoveItemsLimit = ConstU32<1000>;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	// pub MaxElectingVoters: u32 = 40_000;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = ConstU32<100>;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(16)
);

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_040_000,
		max_inflation: 0_050_000,
		// 60% of total issuance at a yearly inflation rate of 5%
		ideal_stake: 0_600_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	// pub MaxNominations: u32 = <NposSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
	// pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	pub const SessionsPerEra: sp_staking::SessionIndex = 2;
	pub const BondingDuration: sp_staking::EraIndex = 24 * 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 24 * 7; // 1/4 the bonding duration.
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
	pub OffchainRepeat: BlockNumber = 5;
	pub const HistoryDepth: u32 = 80;
	pub const MaxExposurePageSize: u32 = 64;
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxValidators = ConstU32<1000>;
	type MaxNominators = ConstU32<0>; // FIXME
}

impl pallet_staking::Config for Runtime {
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = U128CurrencyToVote;
	type RewardRemainder = Treasury;
	type RuntimeEvent = RuntimeEvent;
	type Slash = Treasury; // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type SessionInterface = Self;
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
//	type MaxExposurePageSize = MaxExposurePageSize;
	type NextNewSession = Session;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = BagsList;
	type MaxUnlockingChunks = ConstU32<32>;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type TargetList = pallet_staking::UseValidatorsMap<Runtime>;
//	type MaxControllersInDeprecationBatch = ConstU32<5900>;
	type HistoryDepth = HistoryDepth;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>; // FIXME
	type EventListeners = (); // FIXME
	type MaxNominatorRewardedPerValidator = ();
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = DOLLARS;
	// pub const SpendPeriod: BlockNumber = DAYS;
	pub const SpendPeriod: BlockNumber = MINUTES;
	pub const Burn: Permill = Permill::from_percent(50);
	// pub const TipCountdown: BlockNumber = DAYS;
	pub const TipCountdown: BlockNumber = MINUTES;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = DOLLARS;
	pub const DataDepositPerByte: Balance = CENTS;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaximumReasonLength: u32 = 300;
	pub const MaxApprovals: u32 = 100;
	pub const MaxBalance: Balance = Balance::max_value();
	pub TreasuryAccount: AccountId = Treasury::account_id();
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS as u64;
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;

	type ApproveOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
	>;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
	>;

	// type ApproveOrigin = EnsureSigned<AccountId>;
	// type RejectOrigin = EnsureSigned<AccountId>;

	type RuntimeEvent = RuntimeEvent;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ();
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = Bounties;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, MaxBalance>;
//	type AssetKind = ();
//	type Beneficiary = Self::AccountId;
//	type BeneficiaryLookup = IdentityLookup<Self::AccountId>;
//	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
//	type BalanceConverter = UnityAssetBalanceConversion;
//	type PayoutPeriod = PayoutSpendPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS as u64;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = ();
//	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS as u64;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = ();
//	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

const ALLIANCE_MOTION_DURATION_IN_BLOCKS: BlockNumber = 5 * DAYS as u64;

parameter_types! {
	pub const AllianceMotionDuration: BlockNumber = ALLIANCE_MOTION_DURATION_IN_BLOCKS;
	pub const AllianceMaxProposals: u32 = 100;
	pub const AllianceMaxMembers: u32 = 100;
}

type AllianceCollective = pallet_collective::Instance3;
impl pallet_collective::Config<AllianceCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = AllianceMotionDuration;
	type MaxProposals = AllianceMaxProposals;
	type MaxMembers = AllianceMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = ();
//	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

parameter_types! {
	// pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 5 * DOLLARS;
	pub const BountyDepositBase: Balance = DOLLARS;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = DOLLARS;
	pub const CuratorDepositMax: Balance = 100 * DOLLARS;
	pub const BountyDepositPayoutDelay: BlockNumber = DAYS as u64;
	pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS as u64;
}

impl pallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
	type ChildBountyManager = ChildBounties;
}

parameter_types! {
	pub const ChildBountyValueMinimum: Balance = DOLLARS;
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 = BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
	pub const MaxAuthorities: u32 = 100;
}

impl pallet_child_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = ConstU32<5>;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = pallet_child_bounties::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Keys = SessionKeys;
	type NextSessionRotation = Babe;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type ShouldEndSession = Babe;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
	type MaxNominators = ConstU32<0>; // FIXME
}

/// The numbers configured here could always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory. For now, we set them to smaller values
/// since the staking is bounded and the weight pipeline takes hours for this single pallet.
pub struct ElectionProviderBenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
	const VOTERS: [u32; 2] = [1000, 2000];
	const TARGETS: [u32; 2] = [500, 1000];
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const MAXIMUM_TARGETS: u32 = 300;
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;
impl Get<Option<(usize, ExtendedBalance)>> for OffchainRandomBalancing {
	fn get() -> Option<(usize, ExtendedBalance)> {
		use sp_runtime::traits::TrailingZeroInput;
		let iters = match MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = sp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed")
					% max.saturating_add(1);
				random as usize
			},
		};

		Some((iters, 0))
	}
}

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
	pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;

	// We prioritize im-online heartbeats over election solution submission.
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;

	// signed config
	pub const SignedRewardBase: Balance = DOLLARS;
	pub const SignedDepositBase: Balance = DOLLARS;
	pub const SignedDepositByte: Balance = CENTS;

	// The maximum winners that can be elected by the Election pallet which is equivalent to the
	// maximum active validators the staking pallet can have.
	// pub MaxActiveValidators: u32 = 1000;
}

parameter_types! {
	pub MaxNominations: u32 = <NposSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
	pub MaxElectingVoters: u32 = 40_000;
	// OnChain values are lower.
	pub MaxOnChainElectingVoters: u32 = 5000;
	pub MaxOnChainElectableTargets: u16 = 1250;
	// The maximum winners that can be elected by the Election pallet which is equivalent to the
	// maximum active validators the staking pallet can have.
	pub MaxActiveValidators: u32 = 1000;

	// miner configs
	pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
	// pub MinerMaxWeight: Weight = BlockWeights::get()
	// 	.get(DispatchClass::Normal)
	// 	.max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
	// 	.saturating_sub(BlockExecutionWeight::get());
	// Solution can occupy 90% of normal block size
	// pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
	// 	*BlockLength::get()
	// 	.max
	// 	.get(DispatchClass::Normal);

	pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(10_000.into()).targets_count(1_500.into()).build(); // FIXME
	pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(5_000.into()).targets_count(1_250.into()).build(); // FIXME
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
	>;
	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type Bounds = ElectionBoundsOnChain;
}

pub struct SubmitMinerConfig;
impl pallet_election_provider_multi_phase::MinerConfig for SubmitMinerConfig {
	type AccountId = AccountId;
	type MaxLength = ();
	type MaxWeight = ();
	// type MaxLength = MinerMaxLength;
	// type MaxWeight = MinerMaxWeight;
	type MaxVotesPerVoter = ConstU32<16>;
	type Solution = NposSolution16;
	type MaxWinners = MaxActiveValidators;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
			<Runtime as pallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

parameter_types! {
	pub const SignedFixedDeposit: Balance = deposit(2, 0);
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type BetterSignedThreshold = ();
	type MinerConfig = SubmitMinerConfig;
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = MultiPhaseUnsignedPriority;
	type SignedMaxSubmissions = ConstU32<10>;
	type SignedRewardBase = SignedRewardBase;
	// type SignedDepositBase =
	// 	GeometricDepositBase<Balance, SignedFixedDeposit, SignedDepositIncreaseFactor>;
	type SignedDepositByte = SignedDepositByte;
	type SignedMaxRefunds = ConstU32<3>;
	type SignedDepositWeight = ();
	type SignedMaxWeight = ();
//	type SignedMaxWeight = MinerMaxWeight;
	type SlashHandler = (); // burn slashes
	type RewardHandler = (); // nothing to do upon rewards
	type DataProvider = Staking;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, ()>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type MaxWinners = MaxActiveValidators;
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Self>;
	type ElectionBounds = ElectionBoundsMultiPhase;
	type BetterUnsignedThreshold = ();
	type SignedDepositBase = ();
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
}

impl pallet_bags_list::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type WeightInfo = pallet_bags_list::weights::SubstrateWeight<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = VoteWeight;
}

parameter_types! {
  pub const PostUnbondPoolsWindow: u32 = 4;
  pub const NominationPoolsPalletId: PalletId = PalletId(*b"py/nopls");
  pub const MaxPointsToBalance: u8 = 10;
}

pub struct BalanceToU256;
impl Convert<Balance, sp_core::U256> for BalanceToU256 {
	fn convert(balance: Balance) -> sp_core::U256 {
		sp_core::U256::from(balance)
	}
}
pub struct U256ToBalance;
impl Convert<sp_core::U256, Balance> for U256ToBalance {
	fn convert(n: sp_core::U256) -> Balance {
		n.try_into().unwrap_or(Balance::max_value())
	}
}

impl pallet_nomination_pools::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
//	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type Staking = Staking;
	type PostUnbondingPoolsWindow = PostUnbondPoolsWindow;
	type MaxMetadataLen = ConstU32<256>;
	type MaxUnbonding = ConstU32<8>;
	type PalletId = NominationPoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = MINUTES;
	pub const VotingPeriod: BlockNumber = 3 * MINUTES;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
	pub const MinimumDeposit: Balance = 100 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = MINUTES;
	pub const CooloffPeriod: BlockNumber = MINUTES;
	pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type InstantOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type InstantAllowed = frame_support::traits::ConstBool<true>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = ConstU32<100>;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type MaxProposals = MaxProposals;
	type Preimages = Preimage;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxScheduledPerBlock = ConstU32<512>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}

parameter_types! {
	pub const PreimageBaseDeposit: Balance = DOLLARS;
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = CENTS;
//	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;

	type BaseDeposit = ();
	type ByteDeposit = ();
	// type Consideration = HoldConsideration<
	// 	AccountId,
	// 	Balances,
	// 	PreimageHoldReason,
	// 	LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	// >;
}

parameter_types! {
	pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
	pub NewAssetSymbol: BoundedVec<u8, StringLimit> = (*b"BRIX").to_vec().try_into().unwrap();
	pub NewAssetName: BoundedVec<u8, StringLimit> = (*b"Brix").to_vec().try_into().unwrap();
	pub const Deposit: Balance = DOLLARS;
}

impl pallet_nft_fractionalization::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Deposit = Deposit;
	type Currency = Balances;
	type NewAssetSymbol = NewAssetSymbol;
	type NewAssetName = NewAssetName;
	type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
	type NftId = <Self as pallet_nfts::Config>::ItemId;
	type AssetBalance = <Self as pallet_balances::Config>::Balance;
	type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
	type Assets = Assets;
	type Nfts = Nfts;
	type PalletId = NftFractionalizationPalletId;
	type WeightInfo = ();
	type StringLimit = StringLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type RuntimeHoldReason = RuntimeHoldReason;
}

// TODO::

// parameter_types! {
// 	pub const VoteLockingPeriod: BlockNumber = 30 * DAYS;
// }

// impl pallet_conviction_voting::Config for Runtime {
// 	type WeightInfo = pallet_conviction_voting::weights::SubstrateWeight<Self>;
// 	type RuntimeEvent = RuntimeEvent;
// 	type Currency = Balances;
// 	type VoteLockingPeriod = VoteLockingPeriod;
// 	type MaxVotes = ConstU32<512>;
// 	type MaxTurnout = frame_support::traits::TotalIssuanceOf<Balances, Self::AccountId>;
// 	type Polls = Referenda;
// }

// impl pallet_ranked_collective::Config for Runtime {
// 	type WeightInfo = pallet_ranked_collective::weights::SubstrateWeight<Self>;
// 	type RuntimeEvent = RuntimeEvent;
// 	type PromoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
// 	type DemoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
// 	type Polls = RankedPolls;
// 	type MinRankOfClass = traits::Identity;
// 	type VoteWeight = pallet_ranked_collective::Geometric;
// }

// parameter_types! {
// 	pub const AlarmInterval: BlockNumber = 1;
// 	pub const SubmissionDeposit: Balance = 100 * DOLLARS;
// 	pub const UndecidingTimeout: BlockNumber = 28 * DAYS;
// }

// pub struct TracksInfo;
// impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
// 	type Id = u16;
// 	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;
// 	fn tracks() -> &'static [(Self::Id, pallet_referenda::TrackInfo<Balance, BlockNumber>)] {
// 		static DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 1] = [(
// 			0u16,
// 			pallet_referenda::TrackInfo {
// 				name: "root",
// 				max_deciding: 1,
// 				decision_deposit: 10,
// 				prepare_period: 4,
// 				decision_period: 4,
// 				confirm_period: 2,
// 				min_enactment_period: 4,
// 				min_approval: pallet_referenda::Curve::LinearDecreasing {
// 					length: Perbill::from_percent(100),
// 					floor: Perbill::from_percent(50),
// 					ceil: Perbill::from_percent(100),
// 				},
// 				min_support: pallet_referenda::Curve::LinearDecreasing {
// 					length: Perbill::from_percent(100),
// 					floor: Perbill::from_percent(0),
// 					ceil: Perbill::from_percent(100),
// 				},
// 			},
// 		)];
// 		&DATA[..]
// 	}
// 	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
// 		if let Ok(system_origin) = frame_system::RawOrigin::try_from(id.clone()) {
// 			match system_origin {
// 				frame_system::RawOrigin::Root => Ok(0),
// 				_ => Err(()),
// 			}
// 		} else {
// 			Err(())
// 		}
// 	}
// }

// pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);

// impl pallet_referenda::Config for Runtime {
// 	type WeightInfo = pallet_referenda::weights::SubstrateWeight<Self>;
// 	type RuntimeCall = RuntimeCall;
// 	type RuntimeEvent = RuntimeEvent;
// 	type Scheduler = Scheduler;
// 	type Currency = pallet_balances::Pallet<Self>;
// 	type SubmitOrigin = EnsureSigned<AccountId>;
// 	type CancelOrigin = EnsureRoot<AccountId>;
// 	type KillOrigin = EnsureRoot<AccountId>;
// 	type Slash = ();
// 	type Votes = pallet_conviction_voting::VotesOf<Runtime>;
// 	type Tally = pallet_conviction_voting::TallyOf<Runtime>;
// 	type SubmissionDeposit = SubmissionDeposit;
// 	type MaxQueued = ConstU32<100>;
// 	type UndecidingTimeout = UndecidingTimeout;
// 	type AlarmInterval = AlarmInterval;
// 	type Tracks = TracksInfo;
// 	type Preimages = Preimage;
// }

// impl pallet_referenda::Config<pallet_referenda::Instance2> for Runtime {
// 	type WeightInfo = pallet_referenda::weights::SubstrateWeight<Self>;
// 	type RuntimeCall = RuntimeCall;
// 	type RuntimeEvent = RuntimeEvent;
// 	type Scheduler = Scheduler;
// 	type Currency = pallet_balances::Pallet<Self>;
// 	type SubmitOrigin = EnsureSigned<AccountId>;
// 	type CancelOrigin = EnsureRoot<AccountId>;
// 	type KillOrigin = EnsureRoot<AccountId>;
// 	type Slash = ();
// 	type Votes = pallet_ranked_collective::Votes;
// 	type Tally = pallet_ranked_collective::TallyOf<Runtime>;
// 	type SubmissionDeposit = SubmissionDeposit;
// 	type MaxQueued = ConstU32<100>;
// 	type UndecidingTimeout = UndecidingTimeout;
// 	type AlarmInterval = AlarmInterval;
// 	type Tracks = TracksInfo;
// 	type Preimages = Preimage;
// }

// TODO:

/* pub struct OnAcurastFulfillment;
impl pallet_acurast_fulfillment_receiver::traits::OnFulfillment<Runtime> for OnAcurastFulfillment {
	fn on_fulfillment(
		_from: <Runtime as frame_system::Config>::AccountId,
		_fulfillment: Fulfillment,
	) -> DispatchResultWithInfo<PostDispatchInfo> {
		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
	}
} */

/* impl pallet_acurast_fulfillment_receiver::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnFulfillment = OnAcurastFulfillment;
	type WeightInfo = ();
} */


impl pallet_postit::Config for Runtime {
	type MaxTextLength = ConstU32<160>;
	type OriginCheck = EnsureDipOriginAdapter;
	type OriginSuccess = DipOriginAdapter;
	type RuntimeEvent = RuntimeEvent;
	type Username = Web3Name;
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_dip_consumer, DipConsumer]
		[pallet_relay_store, RelayStore]
	);
}

impl_runtime_apis! {

	impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: cumulus_primitives_aura::Slot,
		) -> bool {
			ConsensusHook::can_build_upon(included_hash, slot)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> SlotDuration {
			SlotDuration::from_millis(SLOT_DURATION)
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: InherentData,
		) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account) as u32
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{BenchmarkError, Benchmarking, BenchmarkBatch};

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {
				fn setup_set_code_requirements(code: &sp_std::vec::Vec<u8>) -> Result<(), BenchmarkError> {
					ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
					Ok(())
				}

				fn verify_set_code() {
					System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
				}
			}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}
