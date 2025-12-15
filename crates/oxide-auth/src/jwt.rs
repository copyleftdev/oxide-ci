//! JWT generation for OIDC tokens.

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("JWT encoding error: {0}")]
    Encoding(#[from] jsonwebtoken::errors::Error),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Token expired")]
    Expired,
}

/// OIDC claims for pipeline authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub nbf: i64,
    pub jti: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_owner: Option<String>,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
}

impl OidcClaims {
    /// Create a new builder for OIDC claims.
    pub fn builder(issuer: impl Into<String>, subject: impl Into<String>, audience: impl Into<String>) -> OidcClaimsBuilder {
        OidcClaimsBuilder::new(issuer, subject, audience)
    }
}

/// Builder for OIDC claims.
pub struct OidcClaimsBuilder {
    issuer: String,
    subject: String,
    audience: String,
    ttl: Duration,
    repository: Option<String>,
    repository_id: Option<String>,
    repository_owner: Option<String>,
    git_ref: Option<String>,
    ref_type: Option<String>,
    sha: Option<String>,
    run_id: Option<String>,
    run_number: Option<u32>,
    pipeline_id: Option<String>,
    pipeline_name: Option<String>,
    actor: Option<String>,
    event_name: Option<String>,
    environment: Option<String>,
}

impl OidcClaimsBuilder {
    pub fn new(issuer: impl Into<String>, subject: impl Into<String>, audience: impl Into<String>) -> Self {
        Self {
            issuer: issuer.into(),
            subject: subject.into(),
            audience: audience.into(),
            ttl: Duration::minutes(5),
            repository: None,
            repository_id: None,
            repository_owner: None,
            git_ref: None,
            ref_type: None,
            sha: None,
            run_id: None,
            run_number: None,
            pipeline_id: None,
            pipeline_name: None,
            actor: None,
            event_name: None,
            environment: None,
        }
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn repository(mut self, repo: impl Into<String>) -> Self {
        self.repository = Some(repo.into());
        self
    }

    pub fn repository_id(mut self, id: impl Into<String>) -> Self {
        self.repository_id = Some(id.into());
        self
    }

    pub fn repository_owner(mut self, owner: impl Into<String>) -> Self {
        self.repository_owner = Some(owner.into());
        self
    }

    pub fn git_ref(mut self, git_ref: impl Into<String>) -> Self {
        self.git_ref = Some(git_ref.into());
        self
    }

    pub fn ref_type(mut self, ref_type: impl Into<String>) -> Self {
        self.ref_type = Some(ref_type.into());
        self
    }

    pub fn sha(mut self, sha: impl Into<String>) -> Self {
        self.sha = Some(sha.into());
        self
    }

    pub fn run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn run_number(mut self, run_number: u32) -> Self {
        self.run_number = Some(run_number);
        self
    }

    pub fn pipeline_id(mut self, id: impl Into<String>) -> Self {
        self.pipeline_id = Some(id.into());
        self
    }

    pub fn pipeline_name(mut self, name: impl Into<String>) -> Self {
        self.pipeline_name = Some(name.into());
        self
    }

    pub fn actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn event_name(mut self, event: impl Into<String>) -> Self {
        self.event_name = Some(event.into());
        self
    }

    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    pub fn build(self) -> OidcClaims {
        let now = Utc::now();
        let exp = now + self.ttl;

        OidcClaims {
            iss: self.issuer,
            sub: self.subject,
            aud: self.audience,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            repository: self.repository,
            repository_id: self.repository_id,
            repository_owner: self.repository_owner,
            git_ref: self.git_ref,
            ref_type: self.ref_type,
            sha: self.sha,
            run_id: self.run_id,
            run_number: self.run_number,
            pipeline_id: self.pipeline_id,
            pipeline_name: self.pipeline_name,
            actor: self.actor,
            event_name: self.event_name,
            environment: self.environment,
        }
    }
}

/// JWT signer for generating OIDC tokens.
pub struct JwtSigner {
    encoding_key: EncodingKey,
    algorithm: Algorithm,
    key_id: Option<String>,
}

impl JwtSigner {
    /// Create a new JWT signer with RS256 algorithm.
    pub fn new_rs256(private_key_pem: &[u8], key_id: Option<String>) -> Result<Self, JwtError> {
        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem)
            .map_err(|e| JwtError::InvalidKey(e.to_string()))?;

        Ok(Self {
            encoding_key,
            algorithm: Algorithm::RS256,
            key_id,
        })
    }

    /// Create a new JWT signer with ES256 algorithm.
    pub fn new_es256(private_key_pem: &[u8], key_id: Option<String>) -> Result<Self, JwtError> {
        let encoding_key = EncodingKey::from_ec_pem(private_key_pem)
            .map_err(|e| JwtError::InvalidKey(e.to_string()))?;

        Ok(Self {
            encoding_key,
            algorithm: Algorithm::ES256,
            key_id,
        })
    }

    /// Sign claims and produce a JWT.
    pub fn sign(&self, claims: &OidcClaims) -> Result<String, JwtError> {
        let mut header = Header::new(self.algorithm);
        header.kid = self.key_id.clone();

        let token = encode(&header, claims, &self.encoding_key)?;
        Ok(token)
    }
}

/// JWT verifier for validating OIDC tokens.
pub struct JwtVerifier {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtVerifier {
    /// Create a new JWT verifier with RS256 algorithm.
    pub fn new_rs256(public_key_pem: &[u8], issuer: &str, audience: &str) -> Result<Self, JwtError> {
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem)
            .map_err(|e| JwtError::InvalidKey(e.to_string()))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);

        Ok(Self {
            decoding_key,
            validation,
        })
    }

    /// Create a new JWT verifier with ES256 algorithm.
    pub fn new_es256(public_key_pem: &[u8], issuer: &str, audience: &str) -> Result<Self, JwtError> {
        let decoding_key = DecodingKey::from_ec_pem(public_key_pem)
            .map_err(|e| JwtError::InvalidKey(e.to_string()))?;

        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);

        Ok(Self {
            decoding_key,
            validation,
        })
    }

    /// Verify and decode a JWT.
    pub fn verify(&self, token: &str) -> Result<OidcClaims, JwtError> {
        let token_data = decode::<OidcClaims>(token, &self.decoding_key, &self.validation)?;
        Ok(token_data.claims)
    }
}

/// OIDC token response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub token_type: String,
    pub expires_at: DateTime<Utc>,
    pub expires_in: i64,
}

impl TokenResponse {
    pub fn new(token: String, expires_at: DateTime<Utc>) -> Self {
        let expires_in = (expires_at - Utc::now()).num_seconds();
        Self {
            token,
            token_type: "Bearer".to_string(),
            expires_at,
            expires_in,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_builder() {
        let claims = OidcClaims::builder(
            "https://token.oxideci.io",
            "repo:acme/app:ref:refs/heads/main:run:123",
            "sts.amazonaws.com",
        )
        .repository("acme/app")
        .git_ref("refs/heads/main")
        .run_id("123")
        .run_number(42)
        .environment("production")
        .build();

        assert_eq!(claims.iss, "https://token.oxideci.io");
        assert_eq!(claims.repository, Some("acme/app".to_string()));
        assert_eq!(claims.run_number, Some(42));
        assert!(claims.exp > claims.iat);
    }
}
