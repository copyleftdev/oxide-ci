//! Secret provider trait and implementations.

use async_trait::async_trait;
use oxide_core::Result;
use std::collections::HashMap;

/// A secret value with metadata.
#[derive(Debug, Clone)]
pub struct SecretValue {
    pub value: String,
    pub version: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Trait for secret providers.
#[async_trait]
pub trait SecretProvider: Send + Sync {
    /// Get a secret by name.
    async fn get(&self, name: &str) -> Result<SecretValue>;

    /// Check if a secret exists.
    async fn exists(&self, name: &str) -> Result<bool>;

    /// List available secrets (names only).
    async fn list(&self) -> Result<Vec<String>>;

    /// Provider name for logging.
    fn name(&self) -> &str;
}

/// Environment variable secret provider.
pub struct EnvProvider {
    prefix: Option<String>,
}

impl EnvProvider {
    pub fn new(prefix: Option<String>) -> Self {
        Self { prefix }
    }

    fn resolve_name(&self, name: &str) -> String {
        match &self.prefix {
            Some(p) => format!("{}_{}", p, name),
            None => name.to_string(),
        }
    }
}

impl Default for EnvProvider {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl SecretProvider for EnvProvider {
    async fn get(&self, name: &str) -> Result<SecretValue> {
        let env_name = self.resolve_name(name);
        std::env::var(&env_name)
            .map(|value| SecretValue {
                value,
                version: None,
                created_at: None,
            })
            .map_err(|_| oxide_core::Error::SecretNotFound(name.to_string()))
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        let env_name = self.resolve_name(name);
        Ok(std::env::var(&env_name).is_ok())
    }

    async fn list(&self) -> Result<Vec<String>> {
        let prefix = self.prefix.as_deref().unwrap_or("");
        Ok(std::env::vars()
            .filter_map(|(k, _)| {
                if prefix.is_empty() || k.starts_with(prefix) {
                    Some(k)
                } else {
                    None
                }
            })
            .collect())
    }

    fn name(&self) -> &str {
        "env"
    }
}

/// File-based secret provider (for development).
pub struct FileProvider {
    secrets: HashMap<String, String>,
}

impl FileProvider {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
        }
    }

    pub fn from_map(secrets: HashMap<String, String>) -> Self {
        Self { secrets }
    }

    pub async fn load_from_file(path: &std::path::Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to read secrets file: {}", e))
        })?;

        let secrets: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to parse secrets: {}", e)))?;

        Ok(Self { secrets })
    }
}

impl Default for FileProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretProvider for FileProvider {
    async fn get(&self, name: &str) -> Result<SecretValue> {
        self.secrets
            .get(name)
            .map(|value| SecretValue {
                value: value.clone(),
                version: None,
                created_at: None,
            })
            .ok_or_else(|| oxide_core::Error::SecretNotFound(name.to_string()))
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        Ok(self.secrets.contains_key(name))
    }

    async fn list(&self) -> Result<Vec<String>> {
        Ok(self.secrets.keys().cloned().collect())
    }

    fn name(&self) -> &str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_env_provider() {
        // SAFETY: This test runs in isolation and doesn't rely on this env var elsewhere
        unsafe { std::env::set_var("TEST_SECRET_123", "secret_value") };
        let provider = EnvProvider::default();

        let value = provider.get("TEST_SECRET_123").await.unwrap();
        assert_eq!(value.value, "secret_value");

        assert!(provider.exists("TEST_SECRET_123").await.unwrap());
        assert!(!provider.exists("NONEXISTENT_SECRET").await.unwrap());
    }

    #[tokio::test]
    async fn test_file_provider() {
        let mut secrets = HashMap::new();
        secrets.insert("DB_PASSWORD".to_string(), "hunter2".to_string());

        let provider = FileProvider::from_map(secrets);

        let value = provider.get("DB_PASSWORD").await.unwrap();
        assert_eq!(value.value, "hunter2");

        assert!(provider.exists("DB_PASSWORD").await.unwrap());
        assert!(!provider.exists("NONEXISTENT").await.unwrap());
    }
}
