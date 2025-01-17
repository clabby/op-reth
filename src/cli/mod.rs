use clap::{Parser, Subcommand};
use reth::{
    cli::{Logs, Verbosity},
    runner::CliRunner,
};

pub mod db;

pub mod blocks;
pub mod dirs;
pub mod genesis;
pub mod node;
pub mod receipts;
pub mod state;

pub fn run() -> eyre::Result<()> {
    dotenv::dotenv().ok();
    let opt = Cli::parse();

    let (layer, _guard) = opt.logs.layer();
    reth_tracing::init(vec![layer, reth_tracing::stdout(opt.verbosity.directive())]);

    let runner = CliRunner::default();

    match opt.command {
        Commands::Genesis(command) => runner.run_command_until_exit(|ctx| command.execute(ctx)),
        Commands::Receipts(command) => runner.run_command_until_exit(|ctx| command.execute(ctx)),
        Commands::State(command) => runner.run_command_until_exit(|ctx| command.execute(ctx)),
        Commands::Blocks(command) => runner.run_command_until_exit(|ctx| command.execute(ctx)),
        Commands::Run => runner.run_command_until_exit(|_| node::run()),
    }
}

/// Commands to be executed
#[derive(Subcommand)]
pub enum Commands {
    /// Load the genesis state
    #[command(name = "genesis")]
    Genesis(genesis::Command),
    /// Load Receipts
    #[command(name = "receipts")]
    Receipts(receipts::Command),
    /// Load the world state trie
    #[command(name = "state")]
    State(state::Command),
    /// Load Blocks
    #[command(name = "blocks")]
    Blocks(blocks::Command),
    /// Run
    #[command(name = "run")]
    Run,
}

#[derive(Parser)]
#[command(author, version = "0.1", about = "Reth", long_about = None)]
struct Cli {
    /// The command to run
    #[clap(subcommand)]
    command: Commands,

    #[clap(flatten)]
    logs: Logs,

    #[clap(flatten)]
    verbosity: Verbosity,
}
