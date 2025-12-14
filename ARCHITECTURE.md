# Oxide CI — Rust Architecture

> Technical specification for the Oxide CI engine implementation.

## Overview

Oxide CI is structured as a **Cargo workspace** with multiple crates following a hexagonal (ports & adapters) architecture. The system is designed for:

- **High performance**: Rust's zero-cost abstractions
- **Extensibility**: WASM plugin system via Extism
- **Observability**: OpenTelemetry native
- **Scalability**: Event-driven with NATS

---

## Workspace Structure

```
oxide-ci/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── spec/                         # AsyncAPI specification (existing)
├── examples/                     # Pipeline examples (existing)
│
├── crates/
│   ├── oxide-core/               # Core domain types and traits
│   ├── oxide-api/                # HTTP/WebSocket API server
│   ├── oxide-scheduler/          # Pipeline scheduling and orchestration
│   ├── oxide-agent/              # Build agent (runs on worker nodes)
│   ├── oxide-runner/             # Step execution engine
│   ├── oxide-plugins/            # WASM plugin host (Extism)
│   ├── oxide-nats/               # NATS client and event bus
│   ├── oxide-db/                 # Database layer (PostgreSQL)
│   ├── oxide-cache/              # Distributed cache (S3/R2 compatible)
│   ├── oxide-secrets/            # Secret provider integrations
│   ├── oxide-auth/               # OIDC token exchange
│   ├── oxide-licensing/          # Keygen integration
│   ├── oxide-billing/            # Stripe integration
│   ├── oxide-notify/             # Notification channels
│   ├── oxide-trace/              # OpenTelemetry integration
│   ├── oxide-cli/                # CLI tool (`oxide`)
│   └── oxide-sdk/                # Rust SDK for API consumers
│
├── plugins/                      # Built-in WASM plugins
│   ├── checkout/
│   ├── docker-build/
│   ├── upload-artifact/
│   └── ...
│
└── deploy/                       # Deployment configurations
    ├── docker/
    ├── kubernetes/
    └── nomad/
```

---

## Crate Specifications

### `oxide-core`

**Purpose**: Core domain types, traits, and error handling. Zero external dependencies except `serde`.

```toml
[package]
name = "oxide-core"
version = "0.1.0"

[dependencies]
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
```

**Key Types**:
```rust
// Domain IDs (newtypes for type safety)
pub struct PipelineId(Uuid);
pub struct RunId(Uuid);
pub struct StageId(String);
pub struct StepId(String);
pub struct AgentId(Uuid);
pub struct LicenseId(String);

// Pipeline definition (parsed from user YAML)
pub struct PipelineDefinition {
    pub version: String,
    pub name: String,
    pub triggers: Vec<TriggerConfig>,
    pub stages: Vec<StageDefinition>,
    pub cache: Option<CacheConfig>,
    pub timeout_minutes: u32,
}

// Run lifecycle
pub enum RunStatus {
    Queued,
    Running,
    Success,
    Failure,
    Cancelled,
    Timeout,
}

// Events (match AsyncAPI spec)
pub enum Event {
    RunQueued(RunQueuedPayload),
    RunStarted(RunStartedPayload),
    RunCompleted(RunCompletedPayload),
    StepOutput(StepOutputPayload),
    CacheHit(CacheHitPayload),
    // ... all events from spec
}

// Traits for ports (hexagonal architecture)
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: Event) -> Result<()>;
    async fn subscribe(&self, pattern: &str) -> Result<EventStream>;
}

pub trait RunRepository: Send + Sync {
    async fn create(&self, run: &Run) -> Result<RunId>;
    async fn get(&self, id: RunId) -> Result<Option<Run>>;
    async fn update_status(&self, id: RunId, status: RunStatus) -> Result<()>;
}

pub trait SecretProvider: Send + Sync {
    async fn get_secret(&self, reference: &SecretReference) -> Result<SecretValue>;
}

pub trait PluginHost: Send + Sync {
    async fn execute(&self, plugin: &str, input: PluginInput) -> Result<PluginOutput>;
}
```

---

### `oxide-api`

**Purpose**: HTTP REST API + WebSocket server for real-time events.

```toml
[package]
name = "oxide-api"

[dependencies]
oxide-core = { path = "../oxide-core" }
oxide-db = { path = "../oxide-db" }
oxide-nats = { path = "../oxide-nats" }
oxide-licensing = { path = "../oxide-licensing" }
oxide-auth = { path = "../oxide-auth" }

axum = "0.8"
axum-extra = { version = "0.10", features = ["typed-header"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "compression-gzip"] }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.26"
serde_json = "1"
tracing = "0.1"
```

**Endpoints**:
```
# Pipelines
POST   /api/v1/pipelines                    # Create pipeline
GET    /api/v1/pipelines                    # List pipelines
GET    /api/v1/pipelines/{id}               # Get pipeline
PUT    /api/v1/pipelines/{id}               # Update pipeline
DELETE /api/v1/pipelines/{id}               # Delete pipeline

# Runs
POST   /api/v1/pipelines/{id}/runs          # Trigger run
GET    /api/v1/runs                         # List runs
GET    /api/v1/runs/{id}                    # Get run
POST   /api/v1/runs/{id}/cancel             # Cancel run
GET    /api/v1/runs/{id}/logs               # Get logs (paginated)

# Agents
GET    /api/v1/agents                       # List agents
GET    /api/v1/agents/{id}                  # Get agent
DELETE /api/v1/agents/{id}                  # Deregister agent

# Secrets
POST   /api/v1/secrets                      # Create secret
GET    /api/v1/secrets                      # List secrets (names only)
DELETE /api/v1/secrets/{name}               # Delete secret

# Approvals
POST   /api/v1/approvals/{id}/approve       # Approve gate
POST   /api/v1/approvals/{id}/reject        # Reject gate

# Webhooks
POST   /api/v1/webhooks/github              # GitHub webhook receiver
POST   /api/v1/webhooks/gitlab              # GitLab webhook receiver

# WebSocket
WS     /api/v1/ws                           # Real-time event stream
```

**WebSocket Protocol**:
```rust
// Client subscribes to channels
{ "action": "subscribe", "channels": ["run.*.started", "run.abc123.*"] }

// Server pushes events
{ "channel": "run.abc123.started", "payload": { ... } }
```

---

### `oxide-scheduler`

**Purpose**: Pipeline scheduling, DAG resolution, agent assignment.

```toml
[package]
name = "oxide-scheduler"

[dependencies]
oxide-core = { path = "../oxide-core" }
oxide-nats = { path = "../oxide-nats" }
oxide-db = { path = "../oxide-db" }
oxide-licensing = { path = "../oxide-licensing" }

tokio = { version = "1", features = ["full"] }
petgraph = "0.7"  # DAG operations
cron = "0.15"     # Cron parsing
```

**Responsibilities**:
1. **Trigger Matching**: Evaluate triggers against incoming events (push, PR, cron)
2. **DAG Resolution**: Build execution graph from `depends_on` relationships
3. **Matrix Expansion**: Expand matrix configurations into individual jobs
4. **Agent Selection**: Match jobs to available agents by labels/capabilities
5. **Queue Management**: Priority queues, concurrency limits, rate limiting
6. **Timeout Enforcement**: Cancel runs exceeding time limits

**Architecture**:
```rust
pub struct Scheduler {
    db: Arc<dyn RunRepository>,
    events: Arc<dyn EventBus>,
    agents: Arc<AgentPool>,
    license: Arc<dyn LicenseValidator>,
}

impl Scheduler {
    /// Main scheduling loop
    pub async fn run(&self) -> Result<()> {
        let mut events = self.events.subscribe("trigger.*").await?;
        
        while let Some(event) = events.next().await {
            match event {
                Event::WebhookReceived(wh) => self.handle_webhook(wh).await?,
                Event::CronTick(tick) => self.handle_cron(tick).await?,
                Event::RunQueued(run) => self.schedule_run(run).await?,
                Event::StageCompleted(stage) => self.advance_dag(stage).await?,
                _ => {}
            }
        }
        Ok(())
    }
    
    /// Build DAG and schedule ready stages
    async fn schedule_run(&self, run: Run) -> Result<()> {
        let dag = self.build_dag(&run.pipeline)?;
        let ready = dag.roots(); // Stages with no dependencies
        
        for stage in ready {
            self.dispatch_stage(run.id, stage).await?;
        }
        Ok(())
    }
}
```

---

### `oxide-agent`

**Purpose**: Build agent that runs on worker nodes. Connects to scheduler, executes jobs.

```toml
[package]
name = "oxide-agent"

[dependencies]
oxide-core = { path = "../oxide-core" }
oxide-runner = { path = "../oxide-runner" }
oxide-nats = { path = "../oxide-nats" }
oxide-plugins = { path = "../oxide-plugins" }
oxide-cache = { path = "../oxide-cache" }
oxide-secrets = { path = "../oxide-secrets" }
oxide-trace = { path = "../oxide-trace" }

tokio = { version = "1", features = ["full"] }
sysinfo = "0.33"  # System metrics
```

**Lifecycle**:
```rust
pub struct Agent {
    id: AgentId,
    config: AgentConfig,
    events: Arc<dyn EventBus>,
    runner: Arc<Runner>,
}

impl Agent {
    pub async fn run(&self) -> Result<()> {
        // 1. Register with scheduler
        self.register().await?;
        
        // 2. Start heartbeat loop
        let heartbeat = self.spawn_heartbeat();
        
        // 3. Listen for job assignments
        let jobs = self.events.subscribe(&format!("agent.{}.job", self.id)).await?;
        
        while let Some(job) = jobs.next().await {
            // 4. Execute job
            let result = self.runner.execute(job).await;
            
            // 5. Report result
            self.events.publish(Event::JobCompleted(result)).await?;
        }
        
        heartbeat.abort();
        Ok(())
    }
    
    async fn spawn_heartbeat(&self) -> JoinHandle<()> {
        let events = self.events.clone();
        let id = self.id;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let metrics = sysinfo::System::new_all();
                events.publish(Event::AgentHeartbeat(AgentHeartbeatPayload {
                    agent_id: id,
                    status: AgentStatus::Idle,
                    system_metrics: SystemMetrics::from(metrics),
                    timestamp: Utc::now(),
                })).await.ok();
            }
        })
    }
}
```

---

### `oxide-runner`

**Purpose**: Step execution engine. Handles containers, Firecracker, Nix, shell commands.

```toml
[package]
name = "oxide-runner"

[dependencies]
oxide-core = { path = "../oxide-core" }
oxide-plugins = { path = "../oxide-plugins" }
oxide-cache = { path = "../oxide-cache" }
oxide-secrets = { path = "../oxide-secrets" }
oxide-trace = { path = "../oxide-trace" }

tokio = { version = "1", features = ["full", "process"] }
bollard = "0.18"           # Docker API
firecracker-rs = "0.1"     # Firecracker control (if available, or raw API)
nix-compat = "0.1"         # Nix integration
async-stream = "0.3"
```

**Execution Environments**:
```rust
#[async_trait]
pub trait ExecutionEnvironment: Send + Sync {
    async fn prepare(&self, config: &EnvironmentConfig) -> Result<()>;
    async fn execute(&self, command: &Command) -> Result<ExecStream>;
    async fn cleanup(&self) -> Result<()>;
}

pub struct ContainerEnvironment {
    docker: bollard::Docker,
    container_id: Option<String>,
}

pub struct FirecrackerEnvironment {
    vm: Option<VmHandle>,
    config: FirecrackerConfig,
}

pub struct NixEnvironment {
    flake: String,
    pure: bool,
}

pub struct HostEnvironment;  // Direct execution (dangerous, needs entitlement)
```

**Step Execution**:
```rust
pub struct Runner {
    plugins: Arc<dyn PluginHost>,
    secrets: Arc<dyn SecretProvider>,
    cache: Arc<dyn CacheProvider>,
    tracer: Arc<Tracer>,
}

impl Runner {
    pub async fn execute_step(&self, step: &StepDefinition, ctx: &RunContext) -> Result<StepResult> {
        let span = self.tracer.span("step.execute")
            .attribute("step.name", &step.name)
            .start();
        
        // 1. Restore cache
        if let Some(cache_cfg) = &ctx.cache {
            self.cache.restore(cache_cfg).await?;
        }
        
        // 2. Inject secrets
        let env = self.secrets.resolve_all(&step.secrets).await?;
        
        // 3. Execute
        let result = if let Some(plugin) = &step.plugin {
            self.plugins.execute(plugin, step.into()).await?
        } else if let Some(run) = &step.run {
            self.execute_shell(run, &env, &step.shell).await?
        } else {
            return Err(Error::InvalidStep("no plugin or run command"));
        };
        
        // 4. Save cache
        if let Some(cache_cfg) = &ctx.cache {
            self.cache.save(cache_cfg).await?;
        }
        
        span.end();
        Ok(result)
    }
}
```

---

### `oxide-plugins`

**Purpose**: WASM plugin host using Extism.

```toml
[package]
name = "oxide-plugins"

[dependencies]
oxide-core = { path = "../oxide-core" }

extism = "1"
tokio = { version = "1", features = ["full"] }
```

**Plugin Interface**:
```rust
pub struct PluginHost {
    plugins: DashMap<String, Mutex<extism::Plugin>>,
    registry_url: String,
}

impl PluginHost {
    /// Load plugin from registry or local path
    pub async fn load(&self, name: &str) -> Result<()> {
        let wasm = if name.starts_with("oxide/") {
            self.fetch_from_registry(name).await?
        } else {
            std::fs::read(name)?
        };
        
        let manifest = extism::Manifest::new([extism::Wasm::data(wasm)])
            .with_allowed_hosts(["*"])  // Configure per-plugin
            .with_memory_max(128 * 1024 * 1024);  // 128MB limit
        
        let plugin = extism::Plugin::new(&manifest, [], true)?;
        self.plugins.insert(name.to_string(), Mutex::new(plugin));
        Ok(())
    }
    
    /// Execute plugin function
    pub async fn call(&self, name: &str, input: &PluginInput) -> Result<PluginOutput> {
        let plugin = self.plugins.get(name)
            .ok_or(Error::PluginNotFound(name.to_string()))?;
        
        let mut plugin = plugin.lock().await;
        let input_json = serde_json::to_vec(input)?;
        let output = plugin.call::<&[u8], &[u8]>("run", &input_json)?;
        let result: PluginOutput = serde_json::from_slice(output)?;
        Ok(result)
    }
}
```

**Plugin SDK (for plugin authors)**:
```rust
// plugins/checkout/src/lib.rs
use extism_pdk::*;
use oxide_plugin_sdk::*;

#[plugin_fn]
pub fn run(input: Json<PluginInput>) -> FnResult<Json<PluginOutput>> {
    let repo = input.get_var("repository")?;
    let ref_ = input.get_var("ref").unwrap_or("HEAD".to_string());
    
    // Clone repository
    let status = std::process::Command::new("git")
        .args(["clone", "--depth=1", "--branch", &ref_, &repo, "."])
        .status()?;
    
    Ok(Json(PluginOutput {
        success: status.success(),
        outputs: HashMap::new(),
    }))
}
```

---

### `oxide-nats`

**Purpose**: NATS client wrapper, event publishing/subscribing.

```toml
[package]
name = "oxide-nats"

[dependencies]
oxide-core = { path = "../oxide-core" }

async-nats = "0.38"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

**Implementation**:
```rust
pub struct NatsEventBus {
    client: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
}

impl NatsEventBus {
    pub async fn connect(url: &str) -> Result<Self> {
        let client = async_nats::connect(url).await?;
        let jetstream = async_nats::jetstream::new(client.clone());
        
        // Ensure streams exist
        jetstream.get_or_create_stream(async_nats::jetstream::stream::Config {
            name: "OXIDE_EVENTS".to_string(),
            subjects: vec!["run.>".to_string(), "agent.>".to_string(), "cache.>".to_string()],
            retention: async_nats::jetstream::stream::RetentionPolicy::Limits,
            max_age: Duration::from_secs(86400 * 7),  // 7 days
            ..Default::default()
        }).await?;
        
        Ok(Self { client, jetstream })
    }
}

#[async_trait]
impl EventBus for NatsEventBus {
    async fn publish(&self, event: Event) -> Result<()> {
        let subject = event.subject();  // e.g., "run.abc123.started"
        let payload = serde_json::to_vec(&event)?;
        self.jetstream.publish(subject, payload.into()).await?;
        Ok(())
    }
    
    async fn subscribe(&self, pattern: &str) -> Result<EventStream> {
        let consumer = self.jetstream
            .create_consumer_on_stream(
                async_nats::jetstream::consumer::pull::Config {
                    filter_subject: pattern.to_string(),
                    ..Default::default()
                },
                "OXIDE_EVENTS",
            )
            .await?;
        
        Ok(EventStream::new(consumer))
    }
}
```

---

### `oxide-db`

**Purpose**: PostgreSQL database layer with SQLx.

```toml
[package]
name = "oxide-db"

[dependencies]
oxide-core = { path = "../oxide-core" }

sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "json"] }
tokio = { version = "1", features = ["full"] }
```

**Schema** (migrations):
```sql
-- migrations/001_initial.sql

CREATE TABLE pipelines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    definition JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL REFERENCES pipelines(id),
    run_number INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    trigger JSONB NOT NULL,
    git_ref TEXT,
    git_sha TEXT,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(pipeline_id, run_number)
);

CREATE TABLE stages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES runs(id),
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER
);

CREATE TABLE steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stage_id UUID NOT NULL REFERENCES stages(id),
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    exit_code INTEGER,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER
);

CREATE TABLE step_logs (
    id BIGSERIAL PRIMARY KEY,
    step_id UUID NOT NULL REFERENCES steps(id),
    stream TEXT NOT NULL,  -- stdout/stderr
    line_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    labels TEXT[] NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'offline',
    last_heartbeat_at TIMESTAMPTZ,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE secrets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    scope TEXT NOT NULL,  -- organization/project/pipeline
    scope_id UUID,
    encrypted_value BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(name, scope, scope_id)
);

CREATE TABLE cache_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key TEXT NOT NULL UNIQUE,
    size_bytes BIGINT NOT NULL,
    storage_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

-- Indexes
CREATE INDEX idx_runs_pipeline ON runs(pipeline_id);
CREATE INDEX idx_runs_status ON runs(status);
CREATE INDEX idx_stages_run ON stages(run_id);
CREATE INDEX idx_steps_stage ON steps(stage_id);
CREATE INDEX idx_step_logs_step ON step_logs(step_id);
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_cache_key ON cache_entries(key);
```

---

### `oxide-secrets`

**Purpose**: Multi-provider secret management.

```toml
[package]
name = "oxide-secrets"

[dependencies]
oxide-core = { path = "../oxide-core" }

tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
aws-sdk-secretsmanager = "1"
aws-sdk-ssm = "1"
gcp-secret-manager = "0.1"
vaultrs = "0.7"
aes-gcm = "0.10"
```

**Architecture**:
```rust
pub struct SecretManager {
    providers: HashMap<String, Arc<dyn SecretProvider>>,
    native: NativeSecretStore,  // Oxide's own encrypted storage
}

impl SecretManager {
    pub fn new(config: &SecretsConfig) -> Result<Self> {
        let mut providers: HashMap<String, Arc<dyn SecretProvider>> = HashMap::new();
        
        if let Some(vault) = &config.vault {
            providers.insert("vault".into(), Arc::new(VaultProvider::new(vault)?));
        }
        if let Some(aws) = &config.aws {
            providers.insert("aws_secrets_manager".into(), Arc::new(AwsSecretsProvider::new(aws)?));
            providers.insert("aws_ssm".into(), Arc::new(AwsSsmProvider::new(aws)?));
        }
        // ... other providers
        
        Ok(Self { providers, native: NativeSecretStore::new()? })
    }
    
    pub async fn resolve(&self, reference: &SecretReference) -> Result<String> {
        match &reference.source.provider {
            "oxide" => self.native.get(&reference.source.path).await,
            provider => {
                let p = self.providers.get(provider)
                    .ok_or(Error::UnknownProvider(provider.clone()))?;
                p.get_secret(&reference.source.path).await
            }
        }
    }
}
```

---

### `oxide-licensing`

**Purpose**: Keygen license validation.

```toml
[package]
name = "oxide-licensing"

[dependencies]
oxide-core = { path = "../oxide-core" }

reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
ed25519-dalek = "2"  # Offline license validation
```

**Implementation**:
```rust
pub struct KeygenClient {
    account_id: String,
    api_url: String,
    public_key: ed25519_dalek::VerifyingKey,
    http: reqwest::Client,
}

impl KeygenClient {
    /// Validate license online
    pub async fn validate(&self, license_key: &str, machine_id: &str) -> Result<License> {
        let resp = self.http
            .post(&format!("{}/licenses/actions/validate-key", self.api_url))
            .json(&json!({
                "meta": {
                    "key": license_key,
                    "scope": { "fingerprint": machine_id }
                }
            }))
            .send()
            .await?;
        
        let result: KeygenResponse = resp.json().await?;
        
        match result.meta.code {
            "VALID" => Ok(License::from(result.data)),
            "SUSPENDED" => Err(Error::LicenseSuspended),
            "EXPIRED" => Err(Error::LicenseExpired),
            code => Err(Error::LicenseInvalid(code.to_string())),
        }
    }
    
    /// Offline validation using signed license file
    pub fn validate_offline(&self, license_file: &[u8]) -> Result<License> {
        // Verify Ed25519 signature
        // Parse and validate claims
        unimplemented!()
    }
}
```

---

### `oxide-billing`

**Purpose**: Stripe billing integration.

```toml
[package]
name = "oxide-billing"

[dependencies]
oxide-core = { path = "../oxide-core" }

stripe-rust = "26"
tokio = { version = "1", features = ["full"] }
```

**Metered Billing**:
```rust
pub struct BillingService {
    stripe: stripe::Client,
}

impl BillingService {
    /// Report usage for metered billing
    pub async fn report_usage(&self, subscription_id: &str, quantity: i64) -> Result<()> {
        let subscription = stripe::Subscription::retrieve(&self.stripe, subscription_id, &[]).await?;
        
        // Find metered subscription item
        let metered_item = subscription.items.data.iter()
            .find(|item| item.price.as_ref().map(|p| p.recurring.as_ref().map(|r| r.usage_type == Some(stripe::UsageType::Metered))).flatten().unwrap_or(false))
            .ok_or(Error::NoMeteredPlan)?;
        
        stripe::UsageRecord::create(
            &self.stripe,
            &metered_item.id,
            stripe::CreateUsageRecord {
                quantity,
                timestamp: Some(chrono::Utc::now().timestamp()),
                action: Some(stripe::UsageRecordAction::Increment),
            },
        ).await?;
        
        Ok(())
    }
    
    /// Handle Stripe webhooks
    pub async fn handle_webhook(&self, payload: &str, signature: &str) -> Result<()> {
        let event = stripe::Webhook::construct_event(payload, signature, &self.webhook_secret)?;
        
        match event.type_ {
            stripe::EventType::InvoicePaymentFailed => {
                // Suspend license
            }
            stripe::EventType::CustomerSubscriptionDeleted => {
                // Revoke license
            }
            _ => {}
        }
        
        Ok(())
    }
}
```

---

### `oxide-cli`

**Purpose**: Command-line interface for developers.

```toml
[package]
name = "oxide-cli"

[dependencies]
oxide-core = { path = "../oxide-core" }
oxide-sdk = { path = "../oxide-sdk" }

clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
indicatif = "0.17"
console = "0.15"
dialoguer = "0.11"
```

**Commands**:
```
oxide
├── init                    # Initialize pipeline in current directory
├── validate                # Validate pipeline YAML
├── run                     # Trigger pipeline run
│   ├── --branch <branch>
│   ├── --wait              # Wait for completion
│   └── --watch             # Stream logs
├── logs <run-id>           # View run logs
├── cancel <run-id>         # Cancel run
├── agents
│   ├── list                # List agents
│   └── drain <agent-id>    # Drain agent
├── secrets
│   ├── set <name>          # Set secret
│   ├── list                # List secrets
│   └── delete <name>       # Delete secret
├── cache
│   ├── list                # List cache entries
│   └── clear               # Clear cache
├── login                   # Authenticate
└── config                  # Manage configuration
```

---

## Service Deployment

### Minimal Deployment (Single Node)

```
┌─────────────────────────────────────────────────────────┐
│                     Single Node                          │
│  ┌─────────┐  ┌───────────┐  ┌─────────┐  ┌─────────┐  │
│  │oxide-api│  │oxide-sched│  │oxide-agt│  │  NATS   │  │
│  └────┬────┘  └─────┬─────┘  └────┬────┘  └────┬────┘  │
│       │             │             │            │        │
│       └─────────────┴─────────────┴────────────┘        │
│                         │                                │
│                    ┌────┴────┐                          │
│                    │PostgreSQL│                          │
│                    └─────────┘                          │
└─────────────────────────────────────────────────────────┘
```

### Production Deployment

```
                    ┌─────────────────┐
                    │   Load Balancer │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
   ┌────┴────┐          ┌────┴────┐          ┌────┴────┐
   │oxide-api│          │oxide-api│          │oxide-api│
   └────┬────┘          └────┬────┘          └────┬────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
                    ┌────────┴────────┐
                    │   NATS Cluster   │
                    │  (JetStream HA)  │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
   ┌────┴─────┐        ┌─────┴─────┐        ┌─────┴────┐
   │oxide-sched│       │oxide-sched│       │oxide-sched│
   │ (leader)  │       │ (standby) │       │ (standby) │
   └───────────┘       └───────────┘       └───────────┘

   ┌─────────────────────────────────────────────────────┐
   │                    Agent Pool                        │
   │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────┐ │
   │  │oxide-agt│  │oxide-agt│  │oxide-agt│  │  ...   │ │
   │  │ linux   │  │ macos   │  │firecrckr│  │        │ │
   │  └─────────┘  └─────────┘  └─────────┘  └────────┘ │
   └─────────────────────────────────────────────────────┘

   ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐
   │ PostgreSQL  │  │ S3/R2 Cache │  │ OpenTelemetry   │
   │  (Primary)  │  │   Storage   │  │   Collector     │
   └─────────────┘  └─────────────┘  └─────────────────┘
```

---

## Build & Development

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run API server locally
cargo run -p oxide-api

# Run agent locally  
cargo run -p oxide-agent

# Build release binaries
cargo build --workspace --release

# Build Docker images
docker build -t oxide-api -f deploy/docker/Dockerfile.api .
docker build -t oxide-agent -f deploy/docker/Dockerfile.agent .
```

---

## Next Steps

1. **Set up Cargo workspace** with initial crate structure
2. **Implement `oxide-core`** with domain types
3. **Implement `oxide-nats`** for event bus
4. **Implement `oxide-db`** with migrations
5. **Build `oxide-api`** with basic CRUD endpoints
6. **Build `oxide-scheduler`** with DAG resolution
7. **Build `oxide-agent`** with container execution
8. **Build `oxide-runner`** with plugin host
9. **Build `oxide-cli`** with essential commands
10. **Integration testing** with docker-compose
