//! Integration test infrastructure for Oxide CI.
//!
//! This crate provides testcontainers-based infrastructure for running
//! integration tests against real services (PostgreSQL, NATS, MinIO).
//!
//! # Usage
//!
//! ```ignore
//! use oxide_tests::TestContext;
//!
//! #[tokio::test]
//! async fn test_something() {
//!     let ctx = TestContext::new().await;
//!     // Use ctx.db, ctx.nats, ctx.api_client, etc.
//! }
//! ```

pub mod containers;
pub mod context;
pub mod fixtures;
pub mod helpers;

pub use context::TestContext;
pub use fixtures::*;
pub use helpers::*;

/// Initialize test logging (call once per test binary).
pub fn init_test_logging() {
    use tracing_subscriber::{fmt, EnvFilter};
    
    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn,oxide_tests=debug")),
        )
        .with_test_writer()
        .try_init();
}
