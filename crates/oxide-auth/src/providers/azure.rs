//! Azure AD Workload Identity Federation token exchange.

use super::{CloudCredentials, ProviderError, TokenExchangeProvider};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Azure credentials from Workload Identity Federation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureCredentials {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub subscription_id: Option<String>,
    pub tenant_id: Option<String>,
}

/// Azure identity configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AzureConfig {
    pub client_id: String,
    pub tenant_id: String,
    pub subscription_id: Option<String>,
}

/// Azure AD Workload Identity Federation token exchange provider.
pub struct AzureProvider {
    config: AzureConfig,
    client: reqwest::Client,
}

impl AzureProvider {
    pub fn new(config: AzureConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn token_endpoint(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.config.tenant_id
        )
    }
}

#[derive(Debug, Deserialize)]
struct AzureTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

#[async_trait]
impl TokenExchangeProvider for AzureProvider {
    async fn exchange(&self, oidc_token: &str) -> Result<CloudCredentials, ProviderError> {
        debug!(
            client_id = %self.config.client_id,
            tenant_id = %self.config.tenant_id,
            "Exchanging OIDC token for Azure credentials"
        );

        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("scope", "https://management.azure.com/.default"),
            (
                "client_assertion_type",
                "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
            ),
            ("client_assertion", oidc_token),
            ("grant_type", "client_credentials"),
        ];

        let response = self
            .client
            .post(self.token_endpoint())
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::TokenExchange(format!(
                "Azure AD error: {}",
                error_text
            )));
        }

        let token_response: AzureTokenResponse = response.json().await?;

        let expires_at = Some(Utc::now() + Duration::seconds(token_response.expires_in));

        Ok(CloudCredentials::Azure(AzureCredentials {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            expires_at,
            subscription_id: self.config.subscription_id.clone(),
            tenant_id: Some(self.config.tenant_id.clone()),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_token_endpoint() {
        let config = AzureConfig {
            client_id: "test-client".to_string(),
            tenant_id: "test-tenant".to_string(),
            subscription_id: None,
        };
        let provider = AzureProvider::new(config);
        assert!(provider.token_endpoint().contains("test-tenant"));
    }
}
