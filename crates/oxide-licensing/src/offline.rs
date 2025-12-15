//! Offline license validation with signed license files.

use crate::types::{License, LicenseStatus, ValidationResult};
use base64::Engine;
use chrono::Utc;
use oxide_core::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Offline license file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    /// License data (base64 encoded).
    pub data: String,
    /// Signature (base64 encoded).
    pub signature: String,
    /// Schema version.
    pub version: u32,
}

/// Offline license validator.
#[derive(Default)]
pub struct OfflineValidator {
    /// Ed25519 public key for signature verification.
    verify_key: Option<Vec<u8>>,
}

impl OfflineValidator {
    /// Create a new offline validator.
    pub fn new(verify_key: Option<&str>) -> Result<Self> {
        let key = verify_key
            .map(|k| {
                base64::engine::general_purpose::STANDARD
                    .decode(k)
                    .map_err(|e| {
                        oxide_core::Error::LicenseInvalid(format!("Invalid verify key: {}", e))
                    })
            })
            .transpose()?;

        Ok(Self { verify_key: key })
    }

    /// Validate a license file.
    pub fn validate(&self, license_file: &LicenseFile) -> Result<ValidationResult> {
        info!("Validating license offline");

        // Decode license data
        let data = base64::engine::general_purpose::STANDARD
            .decode(&license_file.data)
            .map_err(|e| {
                oxide_core::Error::LicenseInvalid(format!("Invalid license data: {}", e))
            })?;

        // Verify signature if we have a key
        if let Some(ref verify_key) = self.verify_key {
            let signature = base64::engine::general_purpose::STANDARD
                .decode(&license_file.signature)
                .map_err(|e| {
                    oxide_core::Error::LicenseInvalid(format!("Invalid signature: {}", e))
                })?;

            if !self.verify_signature(&data, &signature, verify_key) {
                warn!("License signature verification failed");
                return Ok(ValidationResult::failure("Invalid license signature"));
            }
            debug!("License signature verified");
        }

        // Parse license data
        let license: License = serde_json::from_slice(&data).map_err(|e| {
            oxide_core::Error::LicenseInvalid(format!("Invalid license format: {}", e))
        })?;

        // Check expiration
        if let Some(expires_at) = license.expires_at
            && expires_at < Utc::now()
        {
            warn!(expires_at = %expires_at, "License has expired");
            return Ok(ValidationResult::failure("License has expired"));
        }

        // Check status
        if license.status != LicenseStatus::Active {
            return Ok(ValidationResult::failure(format!(
                "License status: {:?}",
                license.status
            )));
        }

        info!(license_id = %license.id, "Offline license validated");
        Ok(ValidationResult::success(license, true))
    }

    /// Load and validate a license file from disk.
    pub async fn validate_file(&self, path: &std::path::Path) -> Result<ValidationResult> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            oxide_core::Error::LicenseInvalid(format!("Failed to read license file: {}", e))
        })?;

        let license_file: LicenseFile = serde_json::from_str(&content).map_err(|e| {
            oxide_core::Error::LicenseInvalid(format!("Invalid license file format: {}", e))
        })?;

        self.validate(&license_file)
    }

    fn verify_signature(&self, data: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
        use ed25519_dalek::{Signature, VerifyingKey};

        let Ok(key_bytes): std::result::Result<[u8; 32], _> = public_key.try_into() else {
            return false;
        };

        let Ok(verifying_key) = VerifyingKey::from_bytes(&key_bytes) else {
            return false;
        };

        let Ok(sig_bytes): std::result::Result<[u8; 64], _> = signature.try_into() else {
            return false;
        };

        let signature = Signature::from_bytes(&sig_bytes);

        use ed25519_dalek::Verifier;
        verifying_key.verify(data, &signature).is_ok()
    }
}

/// Create a signed license file (for testing/development).
#[cfg(test)]
pub fn create_test_license(license: &License, signing_key: &[u8; 32]) -> LicenseFile {
    use ed25519_dalek::SigningKey;

    let data = serde_json::to_vec(license).unwrap();
    let signing_key = SigningKey::from_bytes(signing_key);

    use ed25519_dalek::Signer;
    let signature = signing_key.sign(&data);

    LicenseFile {
        data: base64::engine::general_purpose::STANDARD.encode(&data),
        signature: base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()),
        version: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::License;

    #[test]
    fn test_offline_validation() {
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;

        // Generate keypair
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        // Create license
        let license = License {
            id: "test-123".to_string(),
            key: "TEST-KEY".to_string(),
            status: LicenseStatus::Active,
            name: "Test License".to_string(),
            entitlements: vec![],
            metadata: Default::default(),
            expires_at: Some(Utc::now() + chrono::Duration::days(30)),
            created_at: Utc::now(),
            validated_at: None,
        };

        // Create signed license file
        let license_file = create_test_license(&license, &signing_key.to_bytes());

        // Validate
        let verify_key = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());
        let validator = OfflineValidator::new(Some(&verify_key)).unwrap();
        let result = validator.validate(&license_file).unwrap();

        assert!(result.valid);
        assert!(result.offline);
    }
}
