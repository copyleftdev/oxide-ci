//! Trace context propagation (W3C format).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// W3C Trace Context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub trace_flags: String,
    pub trace_state: Option<String>,
}

impl TraceContext {
    /// Create a new trace context.
    pub fn new(trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id: None,
            trace_flags: "01".to_string(), // sampled
            trace_state: None,
        }
    }

    /// Create from W3C traceparent header value.
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() < 4 {
            return None;
        }

        let version = parts[0];
        if version != "00" {
            return None; // Unsupported version
        }

        Some(Self {
            trace_id: parts[1].to_string(),
            span_id: parts[2].to_string(),
            parent_span_id: None,
            trace_flags: parts[3].to_string(),
            trace_state: None,
        })
    }

    /// Convert to W3C traceparent header value.
    pub fn to_traceparent(&self) -> String {
        format!("00-{}-{}-{}", self.trace_id, self.span_id, self.trace_flags)
    }

    /// Set parent span ID.
    pub fn with_parent(mut self, parent_span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }

    /// Set trace state.
    pub fn with_trace_state(mut self, state: impl Into<String>) -> Self {
        self.trace_state = Some(state.into());
        self
    }

    /// Check if this trace is sampled.
    pub fn is_sampled(&self) -> bool {
        self.trace_flags.ends_with('1')
    }
}

/// Extract trace context from HTTP headers.
pub fn extract_from_headers(headers: &HashMap<String, String>) -> Option<TraceContext> {
    let traceparent = headers.get("traceparent")?;
    let mut ctx = TraceContext::from_traceparent(traceparent)?;

    if let Some(tracestate) = headers.get("tracestate") {
        ctx.trace_state = Some(tracestate.clone());
    }

    Some(ctx)
}

/// Inject trace context into HTTP headers.
pub fn inject_into_headers(ctx: &TraceContext, headers: &mut HashMap<String, String>) {
    headers.insert("traceparent".to_string(), ctx.to_traceparent());

    if let Some(ref state) = ctx.trace_state {
        headers.insert("tracestate".to_string(), state.clone());
    }
}

/// Generate a new random trace ID (32 hex chars).
pub fn generate_trace_id() -> String {
    format!("{:032x}", rand_u128())
}

/// Generate a new random span ID (16 hex chars).
pub fn generate_span_id() -> String {
    format!("{:016x}", rand_u64())
}

fn rand_u128() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Simple PRNG based on time - in production use a proper random source
    nanos as u128 ^ (nanos.wrapping_mul(0x9E3779B97F4A7C15) as u128)
}

fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (nanos as u64) ^ (nanos.wrapping_mul(0x9E3779B97F4A7C15) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traceparent_roundtrip() {
        let ctx = TraceContext::new(
            "4bf92f3577b34da6a3ce929d0e0e4736",
            "00f067aa0ba902b7",
        );

        let header = ctx.to_traceparent();
        assert_eq!(header, "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01");

        let parsed = TraceContext::from_traceparent(&header).unwrap();
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert_eq!(parsed.span_id, ctx.span_id);
    }

    #[test]
    fn test_extract_inject_headers() {
        let ctx = TraceContext::new("abc123", "def456")
            .with_trace_state("vendor=value");

        let mut headers = HashMap::new();
        inject_into_headers(&ctx, &mut headers);

        assert!(headers.contains_key("traceparent"));
        assert!(headers.contains_key("tracestate"));

        let extracted = extract_from_headers(&headers).unwrap();
        assert_eq!(extracted.trace_state, Some("vendor=value".to_string()));
    }

    #[test]
    fn test_is_sampled() {
        let sampled = TraceContext::new("abc", "def");
        assert!(sampled.is_sampled());

        let mut not_sampled = TraceContext::new("abc", "def");
        not_sampled.trace_flags = "00".to_string();
        assert!(!not_sampled.is_sampled());
    }
}
