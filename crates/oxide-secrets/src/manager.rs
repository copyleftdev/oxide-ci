//! Secret manager for resolving and caching secrets.

use crate::providers::{SecretProvider, SecretValue};
use oxide_core::Result;
use oxide_core::pipeline::SecretReference;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Configuration for the secret manager.
#[derive(Debug, Clone)]
pub struct SecretManagerConfig {
    /// Cache TTL in seconds.
    pub cache_ttl_seconds: u64,
    /// Whether to mask secrets in logs.
    pub mask_in_logs: bool,
    /// Default provider name.
    pub default_provider: String,
}

impl Default for SecretManagerConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: 300, // 5 minutes
            mask_in_logs: true,
            default_provider: "env".to_string(),
        }
    }
}

/// Cached secret entry.
struct CachedSecret {
    value: SecretValue,
    cached_at: std::time::Instant,
}

/// Secret manager for resolving secrets from multiple providers.
pub struct SecretManager {
    config: SecretManagerConfig,
    providers: HashMap<String, Arc<dyn SecretProvider>>,
    cache: RwLock<HashMap<String, CachedSecret>>,
}

impl SecretManager {
    /// Create a new secret manager.
    pub fn new(config: SecretManagerConfig) -> Self {
        Self {
            config,
            providers: HashMap::new(),
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Register a secret provider.
    pub fn register_provider(&mut self, name: &str, provider: Arc<dyn SecretProvider>) {
        info!(provider = %name, "Registering secret provider");
        self.providers.insert(name.to_string(), provider);
    }

    /// Resolve a single secret reference.
    pub async fn resolve(&self, reference: &SecretReference) -> Result<String> {
        let provider_name = &reference.source.provider;
        let cache_key = format!("{}:{}", provider_name, reference.name);

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key)
                && cached.cached_at.elapsed().as_secs() < self.config.cache_ttl_seconds
            {
                debug!(name = %reference.name, "Secret cache hit");
                return Ok(cached.value.value.clone());
            }
        }

        // Get provider
        let provider_key = if provider_name.is_empty() {
            &self.config.default_provider
        } else {
            provider_name
        };

        let provider = self
            .providers
            .get(provider_key)
            .ok_or_else(|| oxide_core::Error::SecretProviderNotConfigured(provider_key.clone()))?;

        // Get secret name (use key if provided, otherwise use name)
        let secret_name = reference.key.as_deref().unwrap_or(&reference.name);

        let value = provider.get(secret_name).await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                CachedSecret {
                    value: value.clone(),
                    cached_at: std::time::Instant::now(),
                },
            );
        }

        debug!(name = %reference.name, provider = %provider_key, "Secret resolved");
        Ok(value.value)
    }

    /// Resolve multiple secrets and return as environment map.
    pub async fn resolve_all(
        &self,
        references: &[SecretReference],
    ) -> Result<HashMap<String, String>> {
        let mut result = HashMap::new();

        for reference in references {
            let value = self.resolve(reference).await?;
            // Use the secret name as the env var name
            result.insert(reference.name.clone(), value);
        }

        Ok(result)
    }

    /// Mask a string by replacing secret values with asterisks.
    pub async fn mask_string(&self, input: &str) -> String {
        if !self.config.mask_in_logs {
            return input.to_string();
        }

        let cache = self.cache.read().await;
        let mut output = input.to_string();

        for cached in cache.values() {
            if !cached.value.value.is_empty() && cached.value.value.len() > 3 {
                output = output.replace(&cached.value.value, "***");
            }
        }

        output
    }

    /// Clear the secret cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Secret cache cleared");
    }

    /// Get cache statistics.
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

impl Default for SecretManager {
    fn default() -> Self {
        Self::new(SecretManagerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::FileProvider;
    use oxide_core::pipeline::SecretSource;

    #[tokio::test]
    async fn test_resolve_secret() {
        let mut secrets = HashMap::new();
        secrets.insert("DB_PASSWORD".to_string(), "hunter2".to_string());

        let mut manager = SecretManager::default();
        manager.register_provider("file", Arc::new(FileProvider::from_map(secrets)));

        let reference = SecretReference {
            name: "DB_PASSWORD".to_string(),
            source: SecretSource {
                provider: "file".to_string(),
                path: None,
                version: None,
            },
            key: None,
            masked: true,
            required: true,
        };

        let value = manager.resolve(&reference).await.unwrap();
        assert_eq!(value, "hunter2");
    }

    #[tokio::test]
    async fn test_mask_string() {
        let mut secrets = HashMap::new();
        secrets.insert("PASSWORD".to_string(), "hunter2".to_string());

        let mut manager = SecretManager::default();
        manager.register_provider("file", Arc::new(FileProvider::from_map(secrets)));

        let reference = SecretReference {
            name: "PASSWORD".to_string(),
            source: SecretSource {
                provider: "file".to_string(),
                path: None,
                version: None,
            },
            key: None,
            masked: true,
            required: true,
        };

        // Resolve to populate cache
        manager.resolve(&reference).await.unwrap();

        // Mask
        let masked = manager.mask_string("password=hunter2").await;
        assert_eq!(masked, "password=***");
    }
}
