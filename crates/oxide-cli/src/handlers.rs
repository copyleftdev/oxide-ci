//! Command handlers.

use crate::config::CliConfig;
use console::style;
use std::path::Path;

/// Initialize a new pipeline.
pub async fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new("oxide.yaml");

    if path.exists() {
        println!("{} oxide.yaml already exists", style("!").yellow());
        return Ok(());
    }

    let template = r#"name: my-pipeline
version: "1.0"

triggers:
  - type: push
    branches: ["main"]

stages:
  - name: build
    steps:
      - name: checkout
        plugin: oxide/checkout@v1
      
      - name: build
        run: |
          echo "Building..."
          # Add your build commands here
"#;

    std::fs::write(path, template)?;
    println!("{} Created oxide.yaml", style("✓").green());
    Ok(())
}

/// Validate a pipeline configuration.
pub async fn validate(path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let content = std::fs::read_to_string(path)?;

    // Try to parse as YAML
    let definition: oxide_core::pipeline::PipelineDefinition = serde_yaml::from_str(&content)?;

    println!(
        "{} Pipeline \"{}\" is valid",
        style("✓").green(),
        definition.name
    );
    println!("  Stages: {}", definition.stages.len());

    for stage in &definition.stages {
        println!("    - {} ({} steps)", stage.name, stage.steps.len());
    }

    Ok(())
}

/// Trigger a pipeline run.
pub async fn run_pipeline(
    _config: &CliConfig,
    pipeline: Option<String>,
    _branch: Option<String>,
    _wait: bool,
    _watch: bool,
    secrets: Vec<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::executor::{self, ExecutorConfig};


    // Find pipeline file
    let pipeline_path = executor::find_pipeline_file(pipeline.as_deref());
    let Some(path) = pipeline_path else {
        println!("{} No pipeline file found. Try:", style("✗").red());
        println!("  - .oxide-ci/pipeline.yaml");
        println!("  - oxide.yaml");
        println!("  - Or specify path: oxide run --pipeline <path>");
        return Ok(());
    };

    println!(
        "{} Loading pipeline from {}",
        style("•").dim(),
        style(path.display()).dim()
    );

    // Load and parse pipeline
    let definition = executor::load_pipeline(&path)?;

    // Execute locally
    let mut exec_config = ExecutorConfig::default();

    // Load secrets from .env if present
    if let Ok(content) = std::fs::read_to_string(".env") {
         println!("{} Loading secrets from .env", style("•").dim());
         for line in content.lines() {
             let line = line.trim();
             if line.is_empty() || line.starts_with('#') { continue; }
             if let Some((k, v)) = line.split_once('=') {
                 exec_config.secrets.insert(k.trim().to_string(), v.trim().to_string());
             }
         }
    }

    // Load CLI secrets
    for s in secrets {
        if let Some((k, v)) = s.split_once('=') {
            exec_config.secrets.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    let result = executor::execute_pipeline(&definition, &exec_config, None).await?;

    if !result.success {
        std::process::exit(1);
    }

    Ok(())
}

/// View run logs.
pub async fn logs(
    config: &CliConfig,
    run_id: &str,
    follow: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Fetching logs for run {}...", style(run_id).bold());
    println!("  API URL: {}", config.api_url);

    if follow {
        println!("  Following logs (Ctrl+C to stop)...");
        // TODO: Stream logs via WebSocket
    }

    // TODO: Fetch and display logs
    println!("{} No logs available yet", style("i").blue());
    Ok(())
}

/// Cancel a run.
pub async fn cancel(
    config: &CliConfig,
    run_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Cancelling run {}...", style(run_id).bold());
    println!("  API URL: {}", config.api_url);

    // TODO: Make API call to cancel run
    println!("{} Run cancelled", style("✓").green());
    Ok(())
}

/// List agents.
pub async fn list_agents(
    config: &CliConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Listing agents...");
    println!("  API URL: {}", config.api_url);

    // TODO: Fetch and display agents
    println!("{} No agents registered", style("i").blue());
    Ok(())
}

/// Drain an agent.
pub async fn drain_agent(
    config: &CliConfig,
    agent_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Draining agent {}...", style(agent_id).bold());
    println!("  API URL: {}", config.api_url);

    // TODO: Make API call to drain agent
    println!("{} Agent draining", style("✓").green());
    Ok(())
}

/// Set a secret.
pub async fn set_secret(
    config: &CliConfig,
    name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use dialoguer::Password;

    let value = Password::new()
        .with_prompt(format!("Enter value for {}", name))
        .interact()?;

    println!("Setting secret {}...", style(name).bold());
    println!("  API URL: {}", config.api_url);
    println!("  Value length: {} chars", value.len());

    // TODO: Make API call to set secret
    println!("{} Secret {} created", style("✓").green(), name);
    Ok(())
}

/// List secrets.
pub async fn list_secrets(
    config: &CliConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Listing secrets...");
    println!("  API URL: {}", config.api_url);

    // TODO: Fetch and display secrets
    println!("{} No secrets configured", style("i").blue());
    Ok(())
}

/// Delete a secret.
pub async fn delete_secret(
    _config: &CliConfig,
    name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use dialoguer::Confirm;

    let confirmed = Confirm::new()
        .with_prompt(format!("Delete secret {}?", name))
        .default(false)
        .interact()?;

    if !confirmed {
        println!("{} Cancelled", style("!").yellow());
        return Ok(());
    }

    println!("Deleting secret {}...", style(name).bold());

    // TODO: Make API call to delete secret
    println!("{} Secret {} deleted", style("✓").green(), name);
    Ok(())
}

/// List cache entries.
pub async fn list_cache(
    config: &CliConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Listing cache entries...");
    println!("  API URL: {}", config.api_url);

    // TODO: Fetch and display cache entries
    println!("{} No cache entries", style("i").blue());
    Ok(())
}

/// Clear cache.
pub async fn clear_cache(
    config: &CliConfig,
    prefix: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match &prefix {
        Some(p) => println!("Clearing cache with prefix {}...", style(p).bold()),
        None => println!("Clearing all cache..."),
    }
    println!("  API URL: {}", config.api_url);

    // TODO: Make API call to clear cache
    println!("{} Cache cleared", style("✓").green());
    Ok(())
}

/// Login.
pub async fn login() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use dialoguer::Input;

    let token: String = Input::new()
        .with_prompt("Enter API token")
        .interact_text()?;

    let mut config = CliConfig::load().unwrap_or_default();
    config.token = Some(token);
    config.save()?;

    println!("{} Logged in successfully", style("✓").green());
    Ok(())
}

/// Show configuration.
pub fn show_config(config: &CliConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Current configuration:");
    println!("  api_url: {}", config.api_url);
    println!(
        "  token: {}",
        if config.token.is_some() {
            "***"
        } else {
            "(not set)"
        }
    );
    println!(
        "  project: {}",
        config.project.as_deref().unwrap_or("(not set)")
    );
    println!("  output_format: {:?}", config.output_format);

    if let Ok(path) = CliConfig::config_path() {
        println!("\nConfig file: {}", path.display());
    }

    Ok(())
}

/// Set configuration.
pub fn set_config(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = CliConfig::load().unwrap_or_default();
    config.set(key, value)?;
    config.save()?;

    println!("{} Set {} = {}", style("✓").green(), key, value);
    Ok(())
}
