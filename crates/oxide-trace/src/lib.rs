//! OpenTelemetry integration for Oxide CI.
//!
//! Provides distributed tracing with OTLP export, W3C trace context
//! propagation, and CI/CD-specific span attributes.

pub mod context;
pub mod spans;
pub mod tracer;

pub use context::{
    TraceContext, extract_from_headers, generate_span_id, generate_trace_id, inject_into_headers,
};
pub use spans::{
    CiAttributes, agent_span, cache_span, run_span, secret_span, stage_span, step_span,
};
pub use tracer::{OtlpConfig, Protocol, TracerError, TracingConfig, init_tracer, shutdown_tracer};
