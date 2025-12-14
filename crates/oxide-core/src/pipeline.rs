//! Pipeline definition types.
//!
//! These types represent the user-authored pipeline YAML configuration.

use crate::ids::PipelineId;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PipelineDefinition {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub triggers: Vec<TriggerConfig>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    pub stages: Vec<StageDefinition>,
    #[serde(default)]
    pub cache: Option<CacheConfig>,
    #[serde(default)]
    pub artifacts: Option<ArtifactConfig>,
    #[serde(default = "default_timeout")]
    pub timeout_minutes: u32,
    #[serde(default)]
    pub concurrency: Option<ConcurrencyConfig>,
}

fn default_timeout() -> u32 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TriggerConfig {
    #[serde(rename = "type")]
    pub trigger_type: TriggerType,
    #[serde(default)]
    pub branches: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub paths_ignore: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub cron: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Push,
    PullRequest,
    Cron,
    Manual,
    Api,
    Webhook,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StageDefinition {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub condition: Option<ConditionExpression>,
    #[serde(default)]
    pub environment: Option<ExecutionEnvironment>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    pub steps: Vec<StepDefinition>,
    #[serde(default)]
    pub parallel: bool,
    #[serde(default)]
    pub timeout_minutes: Option<u32>,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub agent: Option<AgentSelector>,
    #[serde(default)]
    pub matrix: Option<MatrixConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StepDefinition {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub plugin: Option<String>,
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default = "default_shell")]
    pub shell: String,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub environment: Option<ExecutionEnvironment>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub secrets: Vec<SecretReference>,
    #[serde(default)]
    pub condition: Option<ConditionExpression>,
    #[serde(default = "default_step_timeout")]
    pub timeout_minutes: u32,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(default)]
    pub outputs: Vec<String>,
}

fn default_shell() -> String {
    "bash".to_string()
}

fn default_step_timeout() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConditionExpression {
    #[serde(rename = "if")]
    pub if_expr: Option<String>,
    pub unless: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecutionEnvironment {
    #[serde(rename = "type", default = "default_env_type")]
    pub env_type: EnvironmentType,
    #[serde(default)]
    pub container: Option<ContainerConfig>,
    #[serde(default)]
    pub firecracker: Option<FirecrackerConfig>,
    #[serde(default)]
    pub nix: Option<NixConfig>,
}

fn default_env_type() -> EnvironmentType {
    EnvironmentType::Container
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentType {
    Container,
    Firecracker,
    Nix,
    Host,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContainerConfig {
    pub image: String,
    #[serde(default)]
    pub registry: Option<RegistryAuth>,
    #[serde(default = "default_shell")]
    pub shell: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub entrypoint: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<VolumeMount>,
    #[serde(default = "default_network")]
    pub network: String,
    #[serde(default)]
    pub privileged: bool,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub resources: Option<ResourceLimits>,
}

fn default_network() -> String {
    "bridge".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegistryAuth {
    pub url: Option<String>,
    pub username: Option<String>,
    pub password_secret: Option<String>,
    #[serde(default)]
    pub aws_ecr: bool,
    #[serde(default)]
    pub gcp_gcr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default = "default_volume_type")]
    pub volume_type: String,
}

fn default_volume_type() -> String {
    "bind".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceLimits {
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub disk: Option<String>,
    pub gpu: Option<GpuConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GpuConfig {
    #[serde(default)]
    pub count: u32,
    pub vendor: Option<String>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FirecrackerConfig {
    pub kernel: String,
    pub rootfs: String,
    #[serde(default = "default_vcpu")]
    pub vcpu_count: u32,
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
    #[serde(default = "default_disk")]
    pub disk_size_gb: u32,
    #[serde(default = "default_true")]
    pub network: bool,
    #[serde(default = "default_boot_timeout")]
    pub boot_timeout_seconds: u32,
}

fn default_vcpu() -> u32 {
    2
}
fn default_memory() -> u32 {
    2048
}
fn default_disk() -> u32 {
    10
}
fn default_true() -> bool {
    true
}
fn default_boot_timeout() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NixConfig {
    #[serde(default)]
    pub flake: Option<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub shell_hook: Option<String>,
    #[serde(default = "default_true")]
    pub pure: bool,
    #[serde(default = "default_true")]
    pub sandbox: bool,
    #[serde(default)]
    pub substituters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretReference {
    pub name: String,
    pub source: SecretSource,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default = "default_true")]
    pub masked: bool,
    #[serde(default = "default_true")]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretSource {
    pub provider: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_delay")]
    pub delay_seconds: u32,
    #[serde(default = "default_true")]
    pub exponential_backoff: bool,
    #[serde(default)]
    pub retry_on: Vec<String>,
}

fn default_max_attempts() -> u32 {
    1
}
fn default_delay() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConcurrencyConfig {
    pub group: String,
    #[serde(default)]
    pub cancel_in_progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentSelector {
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CacheConfig {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub restore_keys: Vec<String>,
    #[serde(default = "default_ttl")]
    pub ttl_days: u32,
}

fn default_ttl() -> u32 {
    7
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactConfig {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default = "default_retention")]
    pub retention_days: u32,
    #[serde(default = "default_compression")]
    pub compression: String,
}

fn default_retention() -> u32 {
    30
}
fn default_compression() -> String {
    "zstd".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MatrixConfig {
    pub dimensions: HashMap<String, Vec<serde_json::Value>>,
    #[serde(default)]
    pub include: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub exclude: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default = "default_true")]
    pub fail_fast: bool,
    #[serde(default)]
    pub max_parallel: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Pipeline {
    pub id: PipelineId,
    pub name: String,
    pub definition: PipelineDefinition,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
