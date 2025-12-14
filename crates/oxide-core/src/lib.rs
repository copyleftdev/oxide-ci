//! Oxide CI Core
//!
//! Core domain types, traits, and error handling for Oxide CI.
//! This crate has minimal dependencies and defines the shared vocabulary
//! used across all other crates.

pub mod agent;
pub mod cache;
pub mod error;
pub mod events;
pub mod ids;
pub mod pipeline;
pub mod ports;
pub mod run;
pub mod secrets;

pub use error::{Error, Result};
pub use ids::*;
