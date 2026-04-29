use polkadot_sdk::*;

use sc_cli::{ChainSpec, Result, SubstrateCli};

use crate::{
    chain_spec,
    cli::{Cli, Subcommand},
    service::new_partial,
};

fn load_spec(id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
    Ok(match id {
        "dev" | "solo-dev" => Box::new(chain_spec::development_chain_spec()),
        "" | "local" | "solo-local" => Box::new(chain_spec::local_chain_spec()),
        path => Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
    })
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "vibly-solo-node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        "Vibly standalone solo-chain node".into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/ArcheLabs/vibly-chain/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2026
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id)
    }
}

macro_rules! construct_async_run {
    (|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
        let runner = $cli.create_runner($cmd)?;
        runner.async_run(|$config| {
            let $components = new_partial(&$config)?;
            let task_manager = $components.task_manager;
            { $( $code )* }.map(|v| (v, task_manager))
        })
    }}
}

pub fn run() -> Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        #[allow(deprecated)]
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::ExportChainSpec(cmd)) => {
            let chain_spec = cli.load_spec(&cmd.chain)?;
            cmd.run(chain_spec)
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            construct_async_run!(|components, cli, cmd, _config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, config.database))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, config.chain_spec))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, _config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::Revert(cmd)) => {
            construct_async_run!(|components, cli, cmd, _config| {
                Ok(cmd.run(components.client, components.backend, None))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        None => {
            let runner = cli.create_runner(&cli.run)?;
            runner.run_node_until_exit(|config| async move {
                crate::service::start_node(config).map(|r| r.0).map_err(Into::into)
            })
        }
    }
}
