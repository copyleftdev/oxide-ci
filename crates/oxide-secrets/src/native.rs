//! Native secret provider with AES-256-GCM encryption.

use crate::providers::{SecretProvider, SecretValue};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use async_trait::async_trait;
use oxide_core::Result;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::debug;

/// Native secret provider with encryption at rest.
pub struct NativeProvider {
    cipher: Aes256Gcm,
    secrets: RwLock<HashMap<String, EncryptedSecret>>,
}

/// An encrypted secret stored in memory.
struct EncryptedSecret {
    ciphertext: Vec<u8>,
    nonce: [u8; 12],
    version: u32,
}

impl NativeProvider {
    /// Create a new native provider with a 32-byte key.
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(key).expect("valid key length");
        Self {
            cipher,
            secrets: RwLock::new(HashMap::new()),
        }
    }

    /// Create from a master key string (will be hashed to 32 bytes).
    pub fn from_master_key(master_key: &str) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(master_key.as_bytes());
        let key: [u8; 32] = hasher.finalize().into();
        Self::new(&key)
    }

    /// Store a secret (encrypts it).
    pub fn store(&self, name: &str, value: &str) -> Result<()> {
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, value.as_bytes())
            .map_err(|e| oxide_core::Error::Internal(format!("Encryption failed: {}", e)))?;

        let mut secrets = self.secrets.write().unwrap();
        let version = secrets.get(name).map(|s| s.version + 1).unwrap_or(1);

        secrets.insert(
            name.to_string(),
            EncryptedSecret {
                ciphertext,
                nonce: nonce_bytes,
                version,
            },
        );

        debug!(name = %name, version, "Secret stored");
        Ok(())
    }

    /// Delete a secret.
    pub fn delete(&self, name: &str) -> bool {
        let mut secrets = self.secrets.write().unwrap();
        secrets.remove(name).is_some()
    }
}

#[async_trait]
impl SecretProvider for NativeProvider {
    async fn get(&self, name: &str) -> Result<SecretValue> {
        let secrets = self.secrets.read().unwrap();
        let encrypted = secrets
            .get(name)
            .ok_or_else(|| oxide_core::Error::SecretNotFound(name.to_string()))?;

        let nonce = Nonce::from_slice(&encrypted.nonce);
        let plaintext = self
            .cipher
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| oxide_core::Error::Internal(format!("Decryption failed: {}", e)))?;

        let value = String::from_utf8(plaintext)
            .map_err(|e| oxide_core::Error::Internal(format!("Invalid UTF-8: {}", e)))?;

        Ok(SecretValue {
            value,
            version: Some(encrypted.version.to_string()),
            created_at: None,
        })
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        let secrets = self.secrets.read().unwrap();
        Ok(secrets.contains_key(name))
    }

    async fn list(&self) -> Result<Vec<String>> {
        let secrets = self.secrets.read().unwrap();
        Ok(secrets.keys().cloned().collect())
    }

    fn name(&self) -> &str {
        "native"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_native_provider() {
        let provider = NativeProvider::from_master_key("test-master-key");

        // Store a secret
        provider.store("DB_PASSWORD", "hunter2").unwrap();

        // Retrieve it
        let value = provider.get("DB_PASSWORD").await.unwrap();
        assert_eq!(value.value, "hunter2");
        assert_eq!(value.version, Some("1".to_string()));

        // Update it
        provider.store("DB_PASSWORD", "newpassword").unwrap();
        let value = provider.get("DB_PASSWORD").await.unwrap();
        assert_eq!(value.value, "newpassword");
        assert_eq!(value.version, Some("2".to_string()));

        // Check exists
        assert!(provider.exists("DB_PASSWORD").await.unwrap());
        assert!(!provider.exists("NONEXISTENT").await.unwrap());

        // Delete
        assert!(provider.delete("DB_PASSWORD"));
        assert!(!provider.exists("DB_PASSWORD").await.unwrap());
    }
}
