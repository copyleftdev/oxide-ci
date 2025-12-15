//! OIDC token exchange for Oxide CI.
//!
//! This crate provides OIDC-based authentication for cloud providers,
//! enabling keyless authentication to AWS, GCP, and Azure.

pub mod jwt;
pub mod oidc;
pub mod providers;

pub use jwt::{JwtError, JwtSigner, JwtVerifier, OidcClaims, OidcClaimsBuilder, TokenResponse};
pub use oidc::{Jwk, Jwks, OidcDiscoveryDocument};
pub use providers::{
    CloudCredentials, ProviderError, TokenExchangeProvider,
    aws::{AwsConfig, AwsCredentials, AwsProvider},
    azure::{AzureConfig, AzureCredentials, AzureProvider},
    gcp::{GcpConfig, GcpCredentials, GcpProvider},
};
