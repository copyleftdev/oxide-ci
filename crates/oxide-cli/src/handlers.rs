//! Command handlers.

use crate::config::CliConfig;
use console::style;
use std::path::Path;

/// Initialize a new pipeline.
pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
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
pub async fn validate(path: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    config: &CliConfig,
    pipeline: Option<String>,
    branch: Option<String>,
    wait: bool,
    watch: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pipeline_name = pipeline.unwrap_or_else(|| "default".to_string());
    let branch_name = branch.unwrap_or_else(|| "main".to_string());

    println!(
        "{} Triggering run for {} on {}",
        style("▶").cyan(),
        style(&pipeline_name).bold(),
        style(&branch_name).dim()
    );

    // TODO: Make API call to trigger run
    println!("  API URL: {}", config.api_url);

    if wait || watch {
        println!("  Waiting for completion...");
        // TODO: Poll for status or stream logs
    }

    println!("{} Run triggered", style("✓").green());
    Ok(())
}

/// View run logs.
pub async fn logs(
    config: &CliConfig,
    run_id: &str,
    follow: bool,
) -> Result<(), Box<dyn std::error::Error>> {
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
pub async fn cancel(config: &CliConfig, run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Cancelling run {}...", style(run_id).bold());
    println!("  API URL: {}", config.api_url);

    // TODO: Make API call to cancel run
    println!("{} Run cancelled", style("✓").green());
    Ok(())
}

/// List agents.
pub async fn list_agents(config: &CliConfig) -> Result<(), Box<dyn std::error::Error>> {
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
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Draining agent {}...", style(agent_id).bold());
    println!("  API URL: {}", config.api_url);

    // TODO: Make API call to drain agent
    println!("{} Agent draining", style("✓").green());
    Ok(())
}

/// Set a secret.
pub async fn set_secret(config: &CliConfig, name: &str) -> Result<(), Box<dyn std::error::Error>> {
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
pub async fn list_secrets(config: &CliConfig) -> Result<(), Box<dyn std::error::Error>> {
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
) -> Result<(), Box<dyn std::error::Error>> {
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
pub async fn list_cache(config: &CliConfig) -> Result<(), Box<dyn std::error::Error>> {
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
) -> Result<(), Box<dyn std::error::Error>> {
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
pub async fn login() -> Result<(), Box<dyn std::error::Error>> {
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
pub fn show_config(config: &CliConfig) -> Result<(), Box<dyn std::error::Error>> {
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
pub fn set_config(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = CliConfig::load().unwrap_or_default();
    config.set(key, value)?;
    config.save()?;

    println!("{} Set {} = {}", style("✓").green(), key, value);
    Ok(())
}
