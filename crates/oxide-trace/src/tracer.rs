//! Tracer initialization and configuration.

use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    runtime,
    trace::{RandomIdGenerator, Sampler},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Error)]
pub enum TracerError {
    #[error("Failed to initialize tracer: {0}")]
    Init(String),
    #[error("Failed to export traces: {0}")]
    Export(String),
}

/// OTLP exporter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpConfig {
    pub endpoint: String,
    pub protocol: Protocol,
    pub headers: std::collections::HashMap<String, String>,
    pub timeout_seconds: u64,
    pub batch_size: usize,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            protocol: Protocol::Grpc,
            headers: std::collections::HashMap::new(),
            timeout_seconds: 10,
            batch_size: 512,
        }
    }
}

/// OTLP protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Grpc,
    HttpProtobuf,
    HttpJson,
}

/// Tracing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    pub enabled: bool,
    pub service_name: String,
    pub service_version: String,
    pub sample_rate: f64,
    pub otlp: Option<OtlpConfig>,
    pub resource_attributes: std::collections::HashMap<String, String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_name: "oxide-ci".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            sample_rate: 1.0,
            otlp: None,
            resource_attributes: std::collections::HashMap::new(),
        }
    }
}

/// Initialize the OpenTelemetry tracer with the given configuration.
pub fn init_tracer(config: &TracingConfig) -> Result<(), TracerError> {
    if !config.enabled {
        // Just init basic tracing without OTLP
        init_basic_tracing();
        return Ok(());
    }

    let resource = build_resource(config);

    match &config.otlp {
        Some(otlp_config) => init_otlp_tracer(config, otlp_config, resource),
        None => {
            init_basic_tracing();
            Ok(())
        }
    }
}

fn build_resource(config: &TracingConfig) -> Resource {
    let mut attrs = vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
    ];

    for (key, value) in &config.resource_attributes {
        attrs.push(KeyValue::new(key.clone(), value.clone()));
    }

    Resource::new(attrs)
}

fn init_otlp_tracer(
    config: &TracingConfig,
    otlp_config: &OtlpConfig,
    resource: Resource,
) -> Result<(), TracerError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&otlp_config.endpoint)
        .with_timeout(std::time::Duration::from_secs(otlp_config.timeout_seconds))
        .build()
        .map_err(|e| TracerError::Init(e.to_string()))?;

    let sampler = if config.sample_rate >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sample_rate <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sample_rate)
    };

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_sampler(sampler)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("oxide-ci");
    global::set_tracer_provider(provider);

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(telemetry_layer)
        .init();

    Ok(())
}

fn init_basic_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// Shutdown the tracer and flush remaining spans.
pub fn shutdown_tracer() {
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TracingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.service_name, "oxide-ci");
        assert_eq!(config.sample_rate, 1.0);
    }

    #[test]
    fn test_otlp_config_default() {
        let config = OtlpConfig::default();
        assert_eq!(config.endpoint, "http://localhost:4317");
        assert_eq!(config.protocol, Protocol::Grpc);
    }
}
