//! Vibly Chain parachain node CLI.

#![warn(missing_docs)]

use polkadot_sdk::*;

mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;

fn main() -> sc_cli::Result<()> {
    command::run()
}
