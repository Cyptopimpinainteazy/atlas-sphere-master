use atlas_sphere_runtime::{
	opaque::Block,
	AccountId, AuraConfig, BalancesConfig, GrandpaConfig, RuntimeGenesisConfig, Signature, WASM_BINARY,
};
use sc_chain_spec::{ChainSpecExtension, Properties};
use sc_service::{ChainSpec as ServiceChainSpec, ChainType};
use serde::{Deserialize, Serialize};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use parity_scale_codec::Encode;
use std::{collections::BTreeSet, path::PathBuf};

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules, customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Generic chain specification type for Atlas Sphere
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

const DEFAULT_PROTOCOL_ID: &str = "atlas";
const ATLAS: u128 = 1_000_000_000_000;
const ENDOWMENT: u128 = 1_000_000 * ATLAS;

type AccountPublic = <Signature as Verify>::Signer;

/// Load a chain specification by name or path
///
/// Supports predefined configurations (dev, local, staging) or loads from a JSON file.
pub fn load_spec(id: &str) -> Result<Box<dyn ServiceChainSpec>, String> {
	match id {
		"" | "dev" => Ok(Box::new(development_config()?)),
		"local" => Ok(Box::new(local_testnet_config()?)),
		"staging" => Ok(Box::new(staging_config()?)),
		path => Ok(Box::new(ChainSpec::from_json_file(PathBuf::from(path))?)),
	}
}

/// Create a development chain specification with Alice as the sole authority
pub fn development_config() -> Result<ChainSpec, String> {
	let initial_authorities = vec![authority_keys_from_seed("Alice")];
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
	];

	Ok(ChainSpec::builder(wasm_binary_unwrap(), Extensions::default())
		.with_name("Atlas Sphere Development")
		.with_id("atlas_sphere_dev")
		.with_chain_type(ChainType::Development)
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.with_properties(default_properties())
		.with_genesis_config_patch(atlas_sphere_genesis(
			initial_authorities,
			endowed_accounts,
		))
		.build())
}

/// Create a local testnet chain specification with Alice and Bob as authorities
pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let initial_authorities = vec![
		authority_keys_from_seed("Alice"),
		authority_keys_from_seed("Bob"),
	];
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
	];

	Ok(ChainSpec::builder(wasm_binary_unwrap(), Extensions::default())
		.with_name("Atlas Sphere Local Testnet")
		.with_id("atlas_sphere_local")
		.with_chain_type(ChainType::Local)
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.with_properties(default_properties())
		.with_genesis_config_patch(atlas_sphere_genesis(
			initial_authorities,
			endowed_accounts,
		))
		.build())
}

/// Create a staging chain specification with three distributed authorities
pub fn staging_config() -> Result<ChainSpec, String> {
	let initial_authorities = vec![
		authority_keys_from_seed("AtlasAlpha"),
		authority_keys_from_seed("AtlasBeta"),
		authority_keys_from_seed("AtlasGamma"),
	];
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("AtlasFoundation"),
		get_account_id_from_seed::<sr25519::Public>("AtlasEcosystem"),
		get_account_id_from_seed::<sr25519::Public>("AtlasCommunity"),
	];

	Ok(ChainSpec::builder(wasm_binary_unwrap(), Extensions::default())
		.with_name("Atlas Sphere Staging")
		.with_id("atlas_sphere_staging")
		.with_chain_type(ChainType::Live)
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.with_properties(default_properties())
		.with_genesis_config_patch(atlas_sphere_genesis(
			initial_authorities,
			endowed_accounts,
		))
		.build())
}

fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect("Atlas Sphere WASM binary must be available when building the node")
}

fn atlas_sphere_genesis(
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	endowed_accounts: Vec<AccountId>,
) -> serde_json::Value {
	let mut endowed: BTreeSet<AccountId> = endowed_accounts.into_iter().collect();

	// Add authority accounts to endowed set
	for (aura, _) in initial_authorities.iter() {
		// Derive account from Aura public key
		let mut account_bytes = [0u8; 32];
		account_bytes.copy_from_slice(&aura.encode()[..32]);
		let account_id = AccountId::from(account_bytes);
		endowed.insert(account_id);
	}

	let balances = endowed
		.iter()
		.cloned()
		.map(|account| (account, ENDOWMENT))
		.collect::<Vec<_>>();

	let aura_authorities: Vec<AuraId> = initial_authorities
		.iter()
		.map(|(aura, _): &(AuraId, GrandpaId)| aura.clone())
		.collect();

	let grandpa_authorities: Vec<(GrandpaId, u64)> = initial_authorities
		.into_iter()
		.map(|(_, grandpa)| (grandpa, 1))
		.collect();

	let genesis_config = RuntimeGenesisConfig {
		system: Default::default(),
		balances: BalancesConfig { balances },
		aura: AuraConfig {
			authorities: aura_authorities,
		},
		grandpa: GrandpaConfig {
			authorities: grandpa_authorities,
			_config: Default::default(),
		},
		transaction_payment: Default::default(),
		council: Default::default(),
	};

	serde_json::to_value(genesis_config)
		.expect("Atlas Sphere genesis config is serializable to JSON")
}

fn default_properties() -> Properties {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "ATLAS".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("ss58Format".into(), 42.into());
	properties
}

fn authority_keys_from_seed(seed: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(seed), get_from_seed::<GrandpaId>(seed))
}

fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<TPublic>,
	TPublic::Pair: Pair,
	TPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

fn get_from_seed<TPublic: Public>(seed: &str) -> TPublic
where
	TPublic::Pair: Pair,
	TPublic: From<<TPublic::Pair as Pair>::Public>,
{
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static seeds are valid; qed")
		.public()
		.into()
}