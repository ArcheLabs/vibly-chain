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
    /// Key management CLI utilities.
    #[command(subcommand)]
    Key(sc_cli::KeySubcommand),
}

const AFTER_HELP_EXAMPLE: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</></>
   <bold>vibly-solo-node --dev --tmp</>
           Launch the local standalone testnet for development and E2E.
   <bold>vibly-solo-node --chain lumen</>
           Launch the Lumen public testnet chain spec.
   <bold>vibly-solo-node export-chain-spec --chain monolith</>
           Export the Monolith incentivized testnet chainspec.
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
