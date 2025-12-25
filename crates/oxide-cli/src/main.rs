//! Oxide CI CLI entrypoint.

use clap::Parser;

mod client;
mod commands;
mod config;
mod dag;
mod executor;
mod handlers;
mod matrix;

#[cfg(test)]
mod artifact_tests;
#[cfg(test)]
mod cache_tests;
#[cfg(test)]
mod examples_tests;
#[cfg(test)]
mod executor_tests;
#[cfg(test)]
mod parallel_tests;
#[cfg(test)]
mod retry_tests;

use commands::{AgentCommands, CacheCommands, Commands, ConfigCommands, SecretCommands};
use config::CliConfig;

#[derive(Parser)]
#[command(name = "oxide")]
#[command(author, version, about = "Oxide CI command-line interface", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config = CliConfig::load().unwrap_or_default();

    match cli.command {
        Commands::Init => handlers::init().await?,
        Commands::Validate { path } => handlers::validate(&path).await?,
        Commands::Run {
            pipeline,
            branch,
            wait,
            watch,
            secrets,
        } => handlers::run_pipeline(&config, pipeline, branch, wait, watch, secrets).await?,
        Commands::Logs { run_id, follow } => handlers::logs(&config, &run_id, follow).await?,
        Commands::Cancel { run_id } => handlers::cancel(&config, &run_id).await?,
        Commands::Agents { command } => match command {
            AgentCommands::List => handlers::list_agents(&config).await?,
            AgentCommands::Drain { agent_id } => handlers::drain_agent(&config, &agent_id).await?,
        },
        Commands::Secrets { command } => match command {
            SecretCommands::Set { name } => handlers::set_secret(&config, &name).await?,
            SecretCommands::List => handlers::list_secrets(&config).await?,
            SecretCommands::Delete { name } => handlers::delete_secret(&config, &name).await?,
        },
        Commands::Cache { command } => match command {
            CacheCommands::List => handlers::list_cache(&config).await?,
            CacheCommands::Clear { prefix } => handlers::clear_cache(&config, prefix).await?,
        },
        Commands::Login => handlers::login().await?,
        Commands::Config { command } => match command {
            ConfigCommands::Show => handlers::show_config(&config)?,
            ConfigCommands::Set { key, value } => handlers::set_config(&key, &value)?,
        },
    }

    Ok(())
}
