use polkadot_sdk::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    #[deprecated(
        note = "build-spec command will be removed after 1/04/2026. Use export-chain-spec instead"
    )]
    BuildSpec(sc_cli::BuildSpecCmd),
    ExportChainSpec(sc_cli::ExportChainSpecCmd),
    CheckBlock(sc_cli::CheckBlockCmd),
    ExportBlocks(sc_cli::ExportBlocksCmd),
    ExportState(sc_cli::ExportStateCmd),
    ImportBlocks(sc_cli::ImportBlocksCmd),
    Revert(sc_cli::RevertCmd),
    PurgeChain(sc_cli::PurgeChainCmd),
}

const AFTER_HELP_EXAMPLE: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</></>
   <bold>vibly-solo-node --dev --tmp</>
           Launch a temporary standalone development chain.
   <bold>vibly-solo-node --chain solo-local --alice</>
           Launch a local testnet node using Alice's authority keys.
   <bold>vibly-solo-node export-chain-spec --chain solo-dev</>
           Export the solo development chainspec.
 "#
);

#[derive(Debug, clap::Parser)]
#[command(
    propagate_version = true,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
#[clap(after_help = AFTER_HELP_EXAMPLE)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[command(flatten)]
    pub run: sc_cli::RunCmd,
}
