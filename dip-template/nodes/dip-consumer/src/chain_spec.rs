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

use cumulus_primitives_core::ParaId;
use dip_consumer_runtime_template::{
	AccountId, AuraId, BalancesConfig, CollatorSelectionConfig, ParachainInfoConfig, RuntimeGenesisConfig,
	opaque::SessionKeys,
	SessionConfig, Signature, SudoConfig, SystemConfig, EXISTENTIAL_DEPOSIT, SS58_PREFIX, WASM_BINARY,
	StakingConfig, AssetsConfig, BabeConfig,
};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup, Properties};
use sc_service::{ChainType, GenericChainSpec};
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_runtime::Perbill;
use pallet_grandpa::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;

const PARA_ID: u32 = 4003;

pub type ChainSpec = GenericChainSpec<RuntimeGenesisConfig, Extensions>;
type AccountPublic = <Signature as Verify>::Signer;

pub(crate) fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	pub relay_chain: String,
	pub para_id: u32,
}

impl Extensions {
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	get_from_seed::<AuraId>(seed)
}

pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn template_session_keys(
	aura: AuraId,
	grandpa: GrandpaId,
 	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId, 
) -> SessionKeys {
	SessionKeys { aura, grandpa, im_online, authority_discovery}
}

pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, AuraId, GrandpaId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(s), 
		get_account_id_from_seed::<sr25519::Public>(s), 
		get_collator_keys_from_seed(s), 
		get_from_seed::<GrandpaId>(s), 
		get_from_seed::<ImOnlineId>(s), 
		get_from_seed::<AuthorityDiscoveryId>(s)
	)
}
pub fn get_root_account() -> AccountId {
	let json_data = &include_bytes!("../../seed/balances.json")[..];
	let additional_accounts_with_balance: Vec<(AccountId, u128)> =
		serde_json::from_slice(json_data).unwrap_or_default();

	additional_accounts_with_balance[0].0.clone()
}
// pub fn template_session_keys(keys: AuraId) -> SessionKeys {
// 	SessionKeys { aura: keys }
// }

fn testnet_genesis(
//	invulnerables: Vec<(AccountId, AuraId)>,
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		AuraId,
		GrandpaId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,

	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> RuntimeGenesisConfig {
	let invulnerables = initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>();
	
	RuntimeGenesisConfig {
		system: SystemConfig {
			code: WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			..Default::default()
		},
		parachain_system: Default::default(),
		parachain_info: ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		sudo: SudoConfig {
			key: Some(endowed_accounts.first().unwrap().clone()),
		},
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		transaction_payment: Default::default(),
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().collect(),
			candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
			..Default::default()
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						template_session_keys(x.2.clone(), x.3.clone() , x.4.clone(), x.5.clone()),
					)
				})
				.collect::<Vec<_>>(),
			// keys: invulnerables
			// 	.into_iter()
			// 	.map(|(acc, aura)| (acc.clone(), acc, template_session_keys(aura)))
			// 	.collect(),
		},
		aura: Default::default(),
		aura_ext: Default::default(),

		staking: StakingConfig {
			stakers: Vec::new(), // FIXIT
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			slash_reward_fraction: Perbill::from_percent(10),
			canceled_payout: 42u128, // FIXIT
			force_era: Default::default(), // FIXIT
			max_nominator_count: None, // FIXIT
			max_validator_count: None, // FIXIT
			min_nominator_bond: 42u128, // FIXIT
			min_validator_bond: 42u128, // FIXIT
		},
		alliance_motion: Default::default(),	
		assets: AssetsConfig {
			assets: vec![(1, root_key.clone(), true, 1)], // Genesis assets: id, owner, is_sufficient, min_balance
			metadata: vec![(1, "XUSD".as_bytes().to_vec(), "XUSD".as_bytes().to_vec(), 0)], // Genesis metadata: id, name, symbol, decimals
			accounts: endowed_accounts.iter().cloned().map(|x| (1, x.clone(), 1_000_000)).collect::<Vec<_>>(),
		},
		authority_discovery: Default::default(),
		// babe: Default::default(),
		babe: BabeConfig {
			epoch_config: Some(dip_consumer_runtime_template::BABE_GENESIS_EPOCH_CONFIG),
			..Default::default()
		},
		council: Default::default(),
		democracy: Default::default(),
		grandpa: Default::default(),
		im_online: Default::default(),
		nomination_pools: Default::default(),
		pool_assets: Default::default(),
		technical_committee: Default::default(),
		treasury: Default::default(),
	}
}

pub fn development_config() -> ChainSpec {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "XCAV".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("ss58Format".into(), SS58_PREFIX.into());

	ChainSpec::from_genesis(
		"DIP consumer dev",
		"dip-consumer-dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				vec![
					authority_keys_from_seed("Alice"),
					authority_keys_from_seed("Bob")
					//(
					// get_account_id_from_seed::<sr25519::Public>("Alice"),
					// get_collator_keys_from_seed("Alice"),
					//)
				],
				// Sudo account
				get_root_account(),
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				],
				PARA_ID.into(),
			)
		},
		Vec::new(),
		None,
		"dip-consumer-dev".into(),
		None,
		None,
		Extensions {
			relay_chain: "paseo".into(),
			para_id: PARA_ID,
		},
	)
}
