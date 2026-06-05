use polkadot_sdk::*;
use sc_service::ChainType;
use sp_core::crypto::Ss58Codec;
use sp_core::{ed25519, sr25519, Pair};
use std::str::FromStr;
use vibly_solo_runtime as runtime;

pub type ChainSpec = sc_service::GenericChainSpec;

const INCENTIVIZED_TESTNET_GUARDIANS: [&str; 2] = [
    "13HBCidDHXxqpt5W6X4TgYYFSDmpSrhyr1J7ENedkMELAih2",
    "12gj7WyRWVAbmUXk1nLoGaK1Um47tcmxdd138DSuYZgngetP",
];
const SUDO_ACCOUNT: &str = "13HEeQf9n7wrmNCLPbxR5RSj8ZUMEn48K7sqrxUqYbY9ssVs";

fn account_id_from_seed(seed: &str) -> runtime::AccountId {
    sp_keyring::Sr25519Keyring::from_str(seed)
        .expect("known development seed")
        .to_account_id()
}

fn account_id_from_ss58(address: &str) -> runtime::AccountId {
    runtime::AccountId::from_ss58check(address).expect("valid guardian ss58 address")
}

fn aura_from_seed(seed: &str) -> runtime::AuraId {
    runtime::AuraId::from(
        sr25519::Pair::from_string(&format!("//{seed}"), None)
            .expect("known aura seed")
            .public(),
    )
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

fn unique_accounts(
    accounts: impl IntoIterator<Item = runtime::AccountId>,
) -> Vec<runtime::AccountId> {
    let mut unique = Vec::new();
    for account in accounts {
        if !unique.contains(&account) {
            unique.push(account);
        }
    }
    unique
}

fn genesis(
    authorities: Vec<&str>,
    guardian_seeds: Vec<&str>,
    endowed_seeds: Vec<&str>,
    guardian_addresses: Vec<&str>,
    endowed_addresses: Vec<&str>,
) -> serde_json::Value {
    let sudo = account_id_from_ss58(SUDO_ACCOUNT);
    let aura = authorities
        .iter()
        .map(|seed| aura_from_seed(seed))
        .collect();
    let grandpa = authorities
        .iter()
        .map(|seed| grandpa_from_seed(seed))
        .collect();
    let guardians = unique_accounts(
        guardian_seeds
            .iter()
            .map(|seed| account_id_from_seed(seed))
            .chain(
                guardian_addresses
                    .iter()
                    .map(|address| account_id_from_ss58(address)),
            ),
    );
    let endowed_accounts = unique_accounts(
        endowed_seeds
            .iter()
            .map(|seed| account_id_from_seed(seed))
            .chain(
                endowed_addresses
                    .iter()
                    .map(|address| account_id_from_ss58(address)),
            )
            .chain([sudo.clone()]),
    );

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
    .with_name("Monolith")
    .with_id("vibly-monolith")
    .with_chain_type(ChainType::Live)
    .with_genesis_config(genesis(
        vec!["Alice"],
        vec![],
        vec![],
        INCENTIVIZED_TESTNET_GUARDIANS.to_vec(),
        vec![INCENTIVIZED_TESTNET_GUARDIANS[0]],
    ))
    .with_protocol_id("vibly-monolith")
    .with_properties(properties())
    .build()
}

pub fn local_chain_spec() -> ChainSpec {
    ChainSpec::builder(
        runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        None,
    )
    .with_name("Lumen")
    .with_id("vibly-lumen")
    .with_chain_type(ChainType::Local)
    .with_genesis_config(genesis(
        vec!["Alice", "Bob"],
        vec!["Alice", "Bob", "Charlie"],
        vec!["Alice", "Bob", "Charlie", "Dave", "Eve", "Ferdie"],
        vec![],
        vec![],
    ))
    .with_protocol_id("vibly-lumen")
    .with_properties(properties())
    .build()
}
