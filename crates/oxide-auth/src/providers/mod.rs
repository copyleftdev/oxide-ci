//! Cloud provider token exchange implementations.

pub mod aws;
pub mod azure;
pub mod gcp;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Token exchange failed: {0}")]
    TokenExchange(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Credentials expired")]
    Expired,
}

/// Cloud credentials with expiration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum CloudCredentials {
    Aws(aws::AwsCredentials),
    Gcp(gcp::GcpCredentials),
    Azure(azure::AzureCredentials),
}

impl CloudCredentials {
    /// Check if credentials are expired or about to expire.
    pub fn is_expired(&self) -> bool {
        let buffer = chrono::Duration::minutes(5);
        match self {
            CloudCredentials::Aws(creds) => {
                creds.expiration.map(|exp| Utc::now() + buffer > exp).unwrap_or(false)
            }
            CloudCredentials::Gcp(creds) => {
                creds.expires_at.map(|exp| Utc::now() + buffer > exp).unwrap_or(false)
            }
            CloudCredentials::Azure(creds) => {
                creds.expires_at.map(|exp| Utc::now() + buffer > exp).unwrap_or(false)
            }
        }
    }
}

/// Token exchange provider trait.
#[async_trait]
pub trait TokenExchangeProvider: Send + Sync {
    /// Exchange an OIDC token for cloud credentials.
    async fn exchange(&self, oidc_token: &str) -> Result<CloudCredentials, ProviderError>;
}

/// Credential cache entry.
#[derive(Debug, Clone)]
pub struct CachedCredentials {
    pub credentials: CloudCredentials,
    pub cached_at: DateTime<Utc>,
}

impl CachedCredentials {
    pub fn new(credentials: CloudCredentials) -> Self {
        Self {
            credentials,
            cached_at: Utc::now(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.credentials.is_expired()
    }
}
