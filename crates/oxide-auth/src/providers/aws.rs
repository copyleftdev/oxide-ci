//! AWS STS token exchange.

use super::{CloudCredentials, ProviderError, TokenExchangeProvider};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// AWS credentials from STS AssumeRoleWithWebIdentity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: Option<DateTime<Utc>>,
    pub region: Option<String>,
}

/// AWS identity configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    pub role_arn: String,
    pub session_name: String,
    pub duration_seconds: u32,
    pub external_id: Option<String>,
    pub region: Option<String>,
}

impl Default for AwsConfig {
    fn default() -> Self {
        Self {
            role_arn: String::new(),
            session_name: "oxide-ci".to_string(),
            duration_seconds: 3600,
            external_id: None,
            region: Some("us-east-1".to_string()),
        }
    }
}

/// AWS STS token exchange provider.
pub struct AwsProvider {
    config: AwsConfig,
    client: reqwest::Client,
}

impl AwsProvider {
    pub fn new(config: AwsConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn sts_endpoint(&self) -> String {
        match &self.config.region {
            Some(region) => format!("https://sts.{}.amazonaws.com", region),
            None => "https://sts.amazonaws.com".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct StsResponse {
    #[serde(rename = "AssumeRoleWithWebIdentityResponse")]
    response: AssumeRoleWithWebIdentityResponse,
}

#[derive(Debug, Deserialize)]
struct AssumeRoleWithWebIdentityResponse {
    #[serde(rename = "AssumeRoleWithWebIdentityResult")]
    result: AssumeRoleWithWebIdentityResult,
}

#[derive(Debug, Deserialize)]
struct AssumeRoleWithWebIdentityResult {
    #[serde(rename = "Credentials")]
    credentials: StsCredentials,
}

#[derive(Debug, Deserialize)]
struct StsCredentials {
    #[serde(rename = "AccessKeyId")]
    access_key_id: String,
    #[serde(rename = "SecretAccessKey")]
    secret_access_key: String,
    #[serde(rename = "SessionToken")]
    session_token: String,
    #[serde(rename = "Expiration")]
    expiration: String,
}

#[async_trait]
impl TokenExchangeProvider for AwsProvider {
    async fn exchange(&self, oidc_token: &str) -> Result<CloudCredentials, ProviderError> {
        debug!(role_arn = %self.config.role_arn, "Exchanging OIDC token for AWS credentials");

        let mut params = vec![
            ("Action", "AssumeRoleWithWebIdentity"),
            ("Version", "2011-06-15"),
            ("RoleArn", &self.config.role_arn),
            ("RoleSessionName", &self.config.session_name),
            ("WebIdentityToken", oidc_token),
        ];

        let duration = self.config.duration_seconds.to_string();
        params.push(("DurationSeconds", &duration));

        if let Some(ref external_id) = self.config.external_id {
            params.push(("ExternalId", external_id));
        }

        let response = self
            .client
            .post(self.sts_endpoint())
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::TokenExchange(format!(
                "AWS STS error: {}",
                error_text
            )));
        }

        let sts_response: StsResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::TokenExchange(format!("Failed to parse STS response: {}", e)))?;

        let creds = &sts_response.response.result.credentials;
        let expiration = DateTime::parse_from_rfc3339(&creds.expiration)
            .map(|dt| dt.with_timezone(&Utc))
            .ok();

        Ok(CloudCredentials::Aws(AwsCredentials {
            access_key_id: creds.access_key_id.clone(),
            secret_access_key: creds.secret_access_key.clone(),
            session_token: creds.session_token.clone(),
            expiration,
            region: self.config.region.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_config_default() {
        let config = AwsConfig::default();
        assert_eq!(config.session_name, "oxide-ci");
        assert_eq!(config.duration_seconds, 3600);
    }
}
