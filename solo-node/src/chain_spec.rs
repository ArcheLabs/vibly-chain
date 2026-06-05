use polkadot_sdk::frame_support::traits::Get;
use polkadot_sdk::*;
use sc_service::ChainType;
use sp_core::crypto::Ss58Codec;
use sp_core::{ed25519, sr25519, Pair};
use std::str::FromStr;
use vibly_solo_runtime as runtime;

pub type ChainSpec = sc_service::GenericChainSpec;

const PUBLIC_GUARDIANS: [&str; 2] = [
    "13HBCidDHXxqpt5W6X4TgYYFSDmpSrhyr1J7ENedkMELAih2",
    "12gj7WyRWVAbmUXk1nLoGaK1Um47tcmxdd138DSuYZgngetP",
];
const SUDO_ACCOUNT: &str = "13HEeQf9n7wrmNCLPbxR5RSj8ZUMEn48K7sqrxUqYbY9ssVs";
const LUMEN_FAUCET_ACCOUNT: &str = "148iJudjCvDNzD7FMpLs7o2gR4Fjpwu4MqMehT6153wH2ymi";
const OPERATIONS_RESERVE_ACCOUNT: &str = "13VnD1QYuZvwzt6i9qSD7Tx1683GQTBsgvJ2QnQvERTXVi3H";

const OPERATIONAL_BALANCE: runtime::Balance = 1_000_000 * runtime::UNIT;
const LUMEN_FAUCET_BALANCE: runtime::Balance = 600_000_000 * runtime::UNIT;
const MONOLITH_REWARD_POOL_BALANCE: runtime::Balance = 30_000_000 * runtime::UNIT;
const MONOLITH_CLAIM_POOL_BALANCE: runtime::Balance = 50_000_000 * runtime::UNIT;

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
    properties.insert("tokenSymbol".into(), "VIB".into());
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

fn merge_balances(
    balances: impl IntoIterator<Item = (runtime::AccountId, runtime::Balance)>,
) -> Vec<(runtime::AccountId, runtime::Balance)> {
    let mut merged: Vec<(runtime::AccountId, runtime::Balance)> = Vec::new();
    for (account, balance) in balances {
        if let Some((_, existing_balance)) =
            merged.iter_mut().find(|(existing, _)| existing == &account)
        {
            *existing_balance = existing_balance.saturating_add(balance);
        } else {
            merged.push((account, balance));
        }
    }
    merged
}

fn balance_from_seed(
    seed: &str,
    balance: runtime::Balance,
) -> (runtime::AccountId, runtime::Balance) {
    (account_id_from_seed(seed), balance)
}

fn balance_from_ss58(
    address: &str,
    balance: runtime::Balance,
) -> (runtime::AccountId, runtime::Balance) {
    (account_id_from_ss58(address), balance)
}

fn genesis(
    authorities: Vec<&str>,
    guardians: Vec<runtime::AccountId>,
    balances: Vec<(runtime::AccountId, runtime::Balance)>,
    enable_rewards: bool,
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
    let reward_config = enable_rewards.then(runtime::genesis_config_presets::default_reward_config);
    let reward_settlement_publisher = enable_rewards.then_some(sudo.clone());

    serde_json::to_value(runtime::genesis_config_presets::custom_config(
        sudo,
        aura,
        grandpa,
        guardians,
        balances,
        reward_config,
        reward_settlement_publisher,
    ))
    .expect("solo genesis config serializes")
}

pub fn monolith_chain_spec() -> ChainSpec {
    let guardians = unique_accounts(
        PUBLIC_GUARDIANS
            .iter()
            .map(|address| account_id_from_ss58(address)),
    );
    let balances = merge_balances([
        balance_from_ss58(SUDO_ACCOUNT, OPERATIONAL_BALANCE),
        balance_from_seed("Alice", OPERATIONAL_BALANCE),
        balance_from_ss58(OPERATIONS_RESERVE_ACCOUNT, OPERATIONAL_BALANCE),
        balance_from_ss58(PUBLIC_GUARDIANS[0], OPERATIONAL_BALANCE),
        balance_from_ss58(PUBLIC_GUARDIANS[1], OPERATIONAL_BALANCE),
        (
            runtime::RewardReserveAccount::get(),
            MONOLITH_REWARD_POOL_BALANCE,
        ),
        (
            runtime::ClaimReserveAccount::get(),
            MONOLITH_CLAIM_POOL_BALANCE,
        ),
    ]);
    ChainSpec::builder(
        runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        None,
    )
    .with_name("Monolith")
    .with_id("vibly-monolith")
    .with_chain_type(ChainType::Live)
    .with_genesis_config(genesis(vec!["Alice"], guardians, balances, true))
    .with_protocol_id("vibly-monolith")
    .with_properties(properties())
    .build()
}

pub fn lumen_chain_spec() -> ChainSpec {
    let guardians = unique_accounts(
        PUBLIC_GUARDIANS
            .iter()
            .map(|address| account_id_from_ss58(address)),
    );
    let balances = merge_balances([
        balance_from_ss58(SUDO_ACCOUNT, OPERATIONAL_BALANCE),
        balance_from_seed("Alice", OPERATIONAL_BALANCE),
        balance_from_seed("Bob", OPERATIONAL_BALANCE),
        balance_from_ss58(PUBLIC_GUARDIANS[0], OPERATIONAL_BALANCE),
        balance_from_ss58(PUBLIC_GUARDIANS[1], OPERATIONAL_BALANCE),
        balance_from_ss58(LUMEN_FAUCET_ACCOUNT, LUMEN_FAUCET_BALANCE),
    ]);
    ChainSpec::builder(
        runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        None,
    )
    .with_name("Lumen")
    .with_id("vibly-lumen")
    .with_chain_type(ChainType::Live)
    .with_genesis_config(genesis(vec!["Alice", "Bob"], guardians, balances, false))
    .with_protocol_id("vibly-lumen")
    .with_properties(properties())
    .build()
}

pub fn local_testnet_chain_spec() -> ChainSpec {
    let sudo = account_id_from_seed("Alice");
    let guardians = unique_accounts(
        ["Alice", "Bob", "Charlie"]
            .iter()
            .map(|seed| account_id_from_seed(seed)),
    );
    let endowed_accounts = unique_accounts(
        ["Alice", "Bob", "Charlie", "Dave", "Eve", "Ferdie"]
            .iter()
            .map(|seed| account_id_from_seed(seed))
            .chain([sudo]),
    );
    ChainSpec::builder(
        runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        None,
    )
    .with_name("Local Testnet")
    .with_id("vibly-local")
    .with_chain_type(ChainType::Local)
    .with_genesis_config(
        serde_json::to_value(runtime::genesis_config_presets::local_testnet_config(
            account_id_from_seed("Alice"),
            vec![aura_from_seed("Alice"), aura_from_seed("Bob")],
            vec![grandpa_from_seed("Alice"), grandpa_from_seed("Bob")],
            guardians,
            endowed_accounts,
        ))
        .expect("solo local genesis config serializes"),
    )
    .with_protocol_id("vibly-local")
    .with_properties(properties())
    .build()
}
