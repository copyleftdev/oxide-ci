//! Keygen.sh API client for online license validation.

use crate::types::{License, LicenseStatus, MachineFingerprint, ValidationResult};
use oxide_core::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Keygen API configuration.
#[derive(Debug, Clone)]
pub struct KeygenConfig {
    /// Keygen account ID.
    pub account_id: String,
    /// Product ID.
    pub product_id: String,
    /// API base URL.
    pub api_url: String,
    /// Verify key for signature verification.
    pub verify_key: Option<String>,
}

impl Default for KeygenConfig {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            product_id: String::new(),
            api_url: "https://api.keygen.sh".to_string(),
            verify_key: None,
        }
    }
}

/// Keygen API client.
pub struct KeygenClient {
    config: KeygenConfig,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct KeygenResponse<T> {
    data: Option<T>,
    errors: Option<Vec<KeygenError>>,
}

#[derive(Debug, Deserialize)]
struct KeygenError {
    title: String,
    detail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeygenLicense {
    id: String,
    attributes: KeygenLicenseAttributes,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeygenLicenseAttributes {
    key: String,
    status: String,
    name: Option<String>,
    expiry: Option<String>,
    created: String,
}

#[derive(Debug, Serialize)]
struct ValidateLicenseRequest {
    meta: ValidateMeta,
}

#[derive(Debug, Serialize)]
struct ValidateMeta {
    key: String,
    scope: Option<ValidateScope>,
}

#[derive(Debug, Serialize)]
struct ValidateScope {
    fingerprint: Option<String>,
    product: Option<String>,
}

impl KeygenClient {
    /// Create a new Keygen client.
    pub fn new(config: KeygenConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Validate a license key online.
    pub async fn validate(&self, license_key: &str) -> Result<ValidationResult> {
        info!(
            key_prefix = &license_key[..8.min(license_key.len())],
            "Validating license online"
        );

        let fingerprint = MachineFingerprint::current();

        let request = ValidateLicenseRequest {
            meta: ValidateMeta {
                key: license_key.to_string(),
                scope: Some(ValidateScope {
                    fingerprint: Some(fingerprint.id.clone()),
                    product: Some(self.config.product_id.clone()),
                }),
            },
        };

        let url = format!(
            "{}/v1/accounts/{}/licenses/actions/validate-key",
            self.config.api_url, self.config.account_id
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/vnd.api+json")
            .header("Accept", "application/vnd.api+json")
            .json(&request)
            .send()
            .await
            .map_err(|e| oxide_core::Error::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            error!(status = %status, "License validation failed");
            return Ok(ValidationResult::failure(format!("API error: {}", status)));
        }

        let body: KeygenResponse<KeygenLicense> = response
            .json()
            .await
            .map_err(|e| oxide_core::Error::Serialization(e.to_string()))?;

        if let Some(errors) = body.errors {
            let error_msg = errors
                .iter()
                .map(|e| e.detail.as_deref().unwrap_or(&e.title))
                .collect::<Vec<_>>()
                .join("; ");
            warn!(error = %error_msg, "License validation rejected");
            return Ok(ValidationResult::failure(error_msg));
        }

        let data = body.data.ok_or_else(|| {
            oxide_core::Error::Internal("No license data in response".to_string())
        })?;

        let status = match data.attributes.status.as_str() {
            "ACTIVE" => LicenseStatus::Active,
            "INACTIVE" => LicenseStatus::Inactive,
            "EXPIRED" => LicenseStatus::Expired,
            "SUSPENDED" => LicenseStatus::Suspended,
            "BANNED" => LicenseStatus::Banned,
            _ => LicenseStatus::Inactive,
        };

        let license = License {
            id: data.id,
            key: data.attributes.key,
            status,
            name: data
                .attributes
                .name
                .unwrap_or_else(|| "Unknown".to_string()),
            entitlements: vec![],
            metadata: Default::default(),
            expires_at: data.attributes.expiry.and_then(|e| e.parse().ok()),
            created_at: data
                .attributes
                .created
                .parse()
                .unwrap_or_else(|_| chrono::Utc::now()),
            validated_at: Some(chrono::Utc::now()),
        };

        if status != LicenseStatus::Active {
            debug!(status = ?status, "License is not active");
            return Ok(ValidationResult::failure(format!(
                "License status: {:?}",
                status
            )));
        }

        info!(license_id = %license.id, "License validated successfully");
        Ok(ValidationResult::success(license, false))
    }

    /// Check if a specific entitlement is available.
    pub fn has_entitlement(license: &License, code: &str) -> bool {
        license.entitlements.iter().any(|e| e.code == code)
    }

    /// Check entitlement usage against limit.
    pub fn check_usage(license: &License, code: &str, required: u64) -> bool {
        license
            .entitlements
            .iter()
            .any(|e| e.code == code && e.limit.is_none_or(|limit| e.usage + required <= limit))
    }
}
