//! GCP Workload Identity Federation token exchange.

use super::{CloudCredentials, ProviderError, TokenExchangeProvider};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// GCP credentials from Workload Identity Federation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpCredentials {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub project_id: Option<String>,
}

/// GCP identity configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpConfig {
    pub workload_identity_provider: String,
    pub service_account_email: String,
    pub token_lifetime: String,
    pub project_id: Option<String>,
}

impl Default for GcpConfig {
    fn default() -> Self {
        Self {
            workload_identity_provider: String::new(),
            service_account_email: String::new(),
            token_lifetime: "3600s".to_string(),
            project_id: None,
        }
    }
}

/// GCP Workload Identity Federation token exchange provider.
pub struct GcpProvider {
    config: GcpConfig,
    client: reqwest::Client,
}

impl GcpProvider {
    pub fn new(config: GcpConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct StsTokenRequest {
    grant_type: String,
    audience: String,
    scope: String,
    requested_token_type: String,
    subject_token: String,
    subject_token_type: String,
}

#[derive(Debug, Deserialize)]
struct StsTokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ImpersonateRequest {
    scope: Vec<String>,
    lifetime: String,
}

#[derive(Debug, Deserialize)]
struct ImpersonateResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expireTime")]
    expire_time: String,
}

#[async_trait]
impl TokenExchangeProvider for GcpProvider {
    async fn exchange(&self, oidc_token: &str) -> Result<CloudCredentials, ProviderError> {
        debug!(
            provider = %self.config.workload_identity_provider,
            "Exchanging OIDC token for GCP credentials"
        );

        // Step 1: Exchange OIDC token for GCP STS token
        let sts_request = StsTokenRequest {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            audience: format!(
                "//iam.googleapis.com/{}",
                self.config.workload_identity_provider
            ),
            scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
            requested_token_type: "urn:ietf:params:oauth:token-type:access_token".to_string(),
            subject_token: oidc_token.to_string(),
            subject_token_type: "urn:ietf:params:oauth:token-type:jwt".to_string(),
        };

        let sts_response = self
            .client
            .post("https://sts.googleapis.com/v1/token")
            .json(&sts_request)
            .send()
            .await?;

        if !sts_response.status().is_success() {
            let error_text = sts_response.text().await.unwrap_or_default();
            return Err(ProviderError::TokenExchange(format!(
                "GCP STS error: {}",
                error_text
            )));
        }

        let sts_token: StsTokenResponse = sts_response.json().await?;

        // Step 2: Impersonate service account
        let impersonate_url = format!(
            "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/{}:generateAccessToken",
            self.config.service_account_email
        );

        let impersonate_request = ImpersonateRequest {
            scope: vec!["https://www.googleapis.com/auth/cloud-platform".to_string()],
            lifetime: self.config.token_lifetime.clone(),
        };

        let impersonate_response = self
            .client
            .post(&impersonate_url)
            .bearer_auth(&sts_token.access_token)
            .json(&impersonate_request)
            .send()
            .await?;

        if !impersonate_response.status().is_success() {
            let error_text = impersonate_response.text().await.unwrap_or_default();
            return Err(ProviderError::TokenExchange(format!(
                "GCP impersonation error: {}",
                error_text
            )));
        }

        let impersonate_token: ImpersonateResponse = impersonate_response.json().await?;

        let expires_at = DateTime::parse_from_rfc3339(&impersonate_token.expire_time)
            .map(|dt| dt.with_timezone(&Utc))
            .ok();

        Ok(CloudCredentials::Gcp(GcpCredentials {
            access_token: impersonate_token.access_token,
            token_type: "Bearer".to_string(),
            expires_at,
            project_id: self.config.project_id.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcp_config_default() {
        let config = GcpConfig::default();
        assert_eq!(config.token_lifetime, "3600s");
    }
}
