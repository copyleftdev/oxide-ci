//! Secret types.

use crate::ids::SecretId;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Secret {
    pub id: SecretId,
    pub name: String,
    pub scope: SecretScope,
    pub scope_id: Option<String>,
    pub provider: SecretProvider,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecretScope {
    Organization,
    Project,
    Pipeline,
    Environment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecretProvider {
    Oxide,
    Vault,
    AwsSsm,
    AwsSecretsManager,
    GcpSecretManager,
    AzureKeyvault,
    Environment,
}

#[derive(Debug, Clone)]
pub struct SecretValue {
    pub value: String,
    pub masked: bool,
}

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            masked: true,
        }
    }

    pub fn unmasked(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            masked: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VaultConfig {
    pub address: String,
    pub namespace: Option<String>,
    pub auth_method: VaultAuthMethod,
    pub role: Option<String>,
    pub mount_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VaultAuthMethod {
    Jwt,
    AppRole,
    Kubernetes,
    Token,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AwsSecretsConfig {
    pub region: String,
    pub role_arn: Option<String>,
    pub use_oidc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GcpSecretsConfig {
    pub project_id: String,
    pub use_workload_identity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AzureKeyVaultConfig {
    pub vault_url: String,
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub use_managed_identity: bool,
}
