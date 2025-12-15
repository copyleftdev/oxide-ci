//! OIDC token generation and exchange.

use serde::{Deserialize, Serialize};

/// OIDC discovery document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscoveryDocument {
    pub issuer: String,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub jwks_uri: String,
    pub response_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub claims_supported: Vec<String>,
}

impl OidcDiscoveryDocument {
    /// Create a new discovery document for Oxide CI.
    pub fn new(issuer: &str, jwks_uri: &str) -> Self {
        Self {
            issuer: issuer.to_string(),
            authorization_endpoint: None,
            token_endpoint: Some(format!("{}/token", issuer)),
            jwks_uri: jwks_uri.to_string(),
            response_types_supported: vec!["id_token".to_string()],
            subject_types_supported: vec!["public".to_string()],
            id_token_signing_alg_values_supported: vec![
                "RS256".to_string(),
                "ES256".to_string(),
            ],
            claims_supported: vec![
                "sub".to_string(),
                "aud".to_string(),
                "exp".to_string(),
                "iat".to_string(),
                "iss".to_string(),
                "jti".to_string(),
                "nbf".to_string(),
                "repository".to_string(),
                "repository_owner".to_string(),
                "ref".to_string(),
                "sha".to_string(),
                "run_id".to_string(),
                "pipeline_id".to_string(),
                "actor".to_string(),
                "environment".to_string(),
            ],
        }
    }
}

/// JSON Web Key for OIDC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    pub kty: String,
    pub kid: String,
    #[serde(rename = "use")]
    pub key_use: String,
    pub alg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
}

/// JSON Web Key Set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

impl Jwks {
    pub fn new() -> Self {
        Self { keys: vec![] }
    }

    pub fn add_key(&mut self, key: Jwk) {
        self.keys.push(key);
    }
}

impl Default for Jwks {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_document() {
        let doc = OidcDiscoveryDocument::new(
            "https://token.oxideci.io",
            "https://token.oxideci.io/.well-known/jwks.json",
        );

        assert_eq!(doc.issuer, "https://token.oxideci.io");
        assert!(doc.id_token_signing_alg_values_supported.contains(&"RS256".to_string()));
    }
}
