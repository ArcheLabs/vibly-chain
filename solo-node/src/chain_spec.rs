use polkadot_sdk::*;
use sc_service::ChainType;
use sp_core::{ed25519, sr25519, Pair};
use std::str::FromStr;
use vibly_solo_runtime as runtime;

pub type ChainSpec = sc_service::GenericChainSpec;

fn account_id_from_seed(seed: &str) -> runtime::AccountId {
    sp_keyring::Sr25519Keyring::from_str(seed)
        .expect("known development seed")
        .to_account_id()
}

fn aura_from_seed(seed: &str) -> runtime::AuraId {
    runtime::AuraId::from(sr25519::Pair::from_string(&format!("//{seed}"), None)
        .expect("known aura seed")
        .public())
}

fn grandpa_from_seed(seed: &str) -> runtime::fg_primitives::AuthorityId {
    runtime::fg_primitives::AuthorityId::from(
        ed25519::Pair::from_string(&format!("//{seed}"), None)
            .expect("known grandpa seed")
            .public(),
    )
}

fn properties() -> sc_chain_spec::Properties {
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "UNIT".into());
    properties.insert("tokenDecimals".into(), 12.into());
    properties.insert("ss58Format".into(), 42.into());
    properties
}

fn genesis(authorities: Vec<&str>, guardians: Vec<&str>, endowed: Vec<&str>) -> serde_json::Value {
    let sudo = account_id_from_seed("Alice");
    let aura = authorities.iter().map(|seed| aura_from_seed(seed)).collect();
    let grandpa = authorities.iter().map(|seed| grandpa_from_seed(seed)).collect();
    let guardians = guardians.iter().map(|seed| account_id_from_seed(seed)).collect();
    let endowed_accounts = endowed.iter().map(|seed| account_id_from_seed(seed)).collect();

    serde_json::to_value(runtime::genesis_config_presets::development_config(
        sudo,
        aura,
        grandpa,
        guardians,
        endowed_accounts,
    ))
    .expect("solo genesis config serializes")
}

pub fn development_chain_spec() -> ChainSpec {
    ChainSpec::builder(
        runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        None,
    )
    .with_name("Vibly Solo Development")
    .with_id("solo-dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config(genesis(
        vec!["Alice"],
        vec!["Alice", "Bob", "Charlie"],
        vec!["Alice", "Bob", "Charlie", "Dave", "Eve", "Ferdie"],
    ))
    .with_protocol_id("vibly-solo-dev")
    .with_properties(properties())
    .build()
}

pub fn local_chain_spec() -> ChainSpec {
    ChainSpec::builder(
        runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        None,
    )
    .with_name("Vibly Solo Local Testnet")
    .with_id("solo-local")
    .with_chain_type(ChainType::Local)
    .with_genesis_config(genesis(
        vec!["Alice", "Bob"],
        vec!["Alice", "Bob", "Charlie"],
        vec!["Alice", "Bob", "Charlie", "Dave", "Eve", "Ferdie"],
    ))
    .with_protocol_id("vibly-solo-local")
    .with_properties(properties())
    .build()
}
