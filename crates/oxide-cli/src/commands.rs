//! CLI command definitions.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new pipeline
    Init,
    
    /// Validate pipeline configuration
    Validate {
        /// Path to pipeline file
        #[arg(default_value = "oxide.yaml")]
        path: String,
    },
    
    /// Trigger a pipeline run
    Run {
        /// Pipeline name or ID
        pipeline: Option<String>,
        
        /// Branch to build
        #[arg(short, long)]
        branch: Option<String>,
        
        /// Wait for completion
        #[arg(short, long)]
        wait: bool,
        
        /// Stream logs
        #[arg(long)]
        watch: bool,
    },
    
    /// View run logs
    Logs {
        /// Run ID
        run_id: String,
        
        /// Follow logs in real-time
        #[arg(short, long)]
        follow: bool,
    },
    
    /// Cancel a run
    Cancel {
        /// Run ID
        run_id: String,
    },
    
    /// Manage agents
    Agents {
        #[command(subcommand)]
        command: AgentCommands,
    },
    
    /// Manage secrets
    Secrets {
        #[command(subcommand)]
        command: SecretCommands,
    },
    
    /// Manage cache
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },
    
    /// Authenticate with Oxide CI
    Login,
    
    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum AgentCommands {
    /// List agents
    List,
    
    /// Drain an agent
    Drain {
        /// Agent ID
        agent_id: String,
    },
}

#[derive(Subcommand)]
pub enum SecretCommands {
    /// Set a secret
    Set {
        /// Secret name
        name: String,
    },
    
    /// List secrets
    List,
    
    /// Delete a secret
    Delete {
        /// Secret name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// List cache entries
    List,
    
    /// Clear cache
    Clear {
        /// Cache key prefix
        #[arg(short, long)]
        prefix: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    
    /// Set configuration value
    Set {
        /// Key
        key: String,
        
        /// Value
        value: String,
    },
}
