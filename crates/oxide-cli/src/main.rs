//! Oxide CI CLI entrypoint.

use clap::Parser;

mod commands;

#[derive(Parser)]
#[command(name = "oxide")]
#[command(author, version, about = "Oxide CI command-line interface", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // TODO: Implement command dispatch
    println!("Oxide CI v{}", env!("CARGO_PKG_VERSION"));

    Ok(())
}
