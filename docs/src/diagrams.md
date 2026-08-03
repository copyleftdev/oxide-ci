# System Diagrams

A visual tour of the ecosystem: how the crates relate, what happens when a
pipeline runs, and how references, secrets, and the spec flow through the
engine. Every diagram here describes code that exists — not a target state.

> **Rendering:** GitHub renders these natively. The mdBook build shows them as
> code blocks unless the mermaid preprocessor is installed:
> `cargo install mdbook-mermaid && mdbook-mermaid install docs`

## 1. Crate topology

Hexagonal architecture: `oxide-core` owns the domain and declares ports as
traits; adapters implement them and hold no business logic. Dependencies point
inward — nothing in `oxide-core` knows about Postgres, NATS, or Docker.

```mermaid
flowchart LR
    subgraph entry["Entrypoints"]
        direction TB
        cli["oxide-cli<br/><i>the oxide binary</i>"]
        api["oxide-api<br/><i>HTTP + WebSocket</i>"]
        sched["oxide-scheduler<br/><i>DAG resolution, dispatch</i>"]
        agent["oxide-agent<br/><i>distributed worker</i>"]
    end

    subgraph domain["Domain — oxide-core"]
        direction TB
        types["<b>types</b><br/>PipelineDefinition · events<br/>newtype IDs · Error"]
        ports["<b>ports</b> — traits<br/>EventBus · SecretProvider<br/>CacheProvider · PluginHost<br/>Pipeline/Run/Agent repositories"]
    end

    subgraph adapters["Adapters — implement the ports"]
        direction TB
        db["oxide-db<br/><i>PostgreSQL</i>"]
        nats["oxide-nats<br/><i>NATS JetStream</i>"]
        cache["oxide-cache<br/><i>artifacts, deps</i>"]
        secrets["oxide-secrets"]
        runner["oxide-runner<br/><i>container · shell · nix</i>"]
        plugins["oxide-plugins<br/><i>built-ins, WASM host</i>"]
        commercial["oxide-auth · oxide-licensing<br/>oxide-billing · oxide-notify"]
    end

    cli --> types
    api --> types
    sched --> types
    agent --> types

    types --- ports

    ports -.->|implemented by| adapters

    spec["oxide-spec<br/><i>keeps types and<br/>the AsyncAPI spec in step</i>"] -.-> types
    trace["oxide-trace<br/><i>OpenTelemetry</i>"] -.-> entry
```

Read the dotted edges as "satisfies": the arrows in the diagram point from the
trait to its implementors, but the *dependency* points the other way. No adapter
name appears anywhere in `oxide-core`, which is the property the architecture
is protecting.

## 2. What happens when a pipeline runs

The local path, which is what `oxide run` exercises and what the end-to-end
tests assert on.

```mermaid
sequenceDiagram
    actor dev as Developer
    participant cli as oxide-cli
    participant dag as DagBuilder
    participant exec as executor
    participant run as ContainerRunner
    participant docker as Docker daemon

    dev->>cli: oxide run pipeline.yaml
    cli->>cli: parse YAML into PipelineDefinition
    cli->>dag: build(definition)
    dag-->>cli: stage graph
    loop until no stage is ready
        exec->>exec: find stages whose dependencies completed
        exec->>run: execute step (per step, parallel if stage says so)
        run->>docker: inspect image
        alt image absent
            run->>docker: pull image
            docker-->>run: pull progress
        end
        run->>docker: create + start container<br/>workspace bind-mounted at /workspace
        docker-->>run: log stream
        run->>exec: masked output lines
        run->>docker: wait for exit
        docker-->>run: exit code
        run-->>exec: StepResult
    end
    exec-->>cli: PipelineResult
    cli-->>dev: stage and step outcomes
```

A stage failure stops the loop: stages that depend on it never run, and the
failed stage is recorded so the result says *what* failed rather than only
*that* something did.

## 2b. How a run travels the distributed system

The path a run takes when the API, scheduler and agent are running as separate
processes. Each hop is a different process, which is what makes a stall
diagnosable: the trail simply stops at whichever service is not doing its part.

```mermaid
sequenceDiagram
    actor dev as Developer
    participant api as oxide-api
    participant bus as NATS JetStream
    participant sched as oxide-scheduler
    participant agent as oxide-agent

    dev->>api: POST /pipelines/{id}/runs
    api->>api: create the run
    api->>bus: RunQueued
    api-->>dev: 201, status queued

    bus->>sched: RunQueued
    sched->>sched: adopt the run, build the DAG,<br/>queue stages with no dependencies
    sched->>bus: JobDispatched<br/>agent.{agent_id}.job.dispatched<br/>matched_labels · queue_wait_ms · attempt
    bus->>agent: JobDispatched

    alt the agent has capacity
        agent->>bus: JobAccepted
        agent->>agent: run the steps
        agent->>bus: StepStarted / StepCompleted / StageCompleted
        bus->>sched: StageCompleted
        sched->>sched: advance the DAG, queue what is now ready
    else at capacity, draining, or missing a capability
        agent->>bus: JobRejected<br/>typed reason · retryable
        bus->>sched: JobRejected
        sched->>sched: requeue if retryable,<br/>otherwise stop and say why
    end
```

The scheduler **adopts** the run rather than creating it, and deliberately
publishes nothing when it does — re-announcing a run it just heard about would
feed itself forever.

A run stuck in `queued` is almost always the second hop or the third: no
scheduler running, or no agent whose labels match.

## 3. Resolving a `uses:` reference

How `uses: docker-build@v1` becomes a plugin, and where each failure mode
diverges.

```mermaid
flowchart TD
    ref["uses: name@version"] --> parse["PluginRef::parse<br/>split name from version"]
    parse --> lookup{"name matches a<br/>BUILTINS entry<br/>or alias?"}
    lookup -->|no| notfound["PluginNotFound<br/>lists every built-in with its version"]
    lookup -->|yes| vspec["VersionSpec::parse"]

    vspec --> kind{"what kind<br/>of suffix?"}
    kind -->|"absent or @latest"| any["Any<br/>accepts what is installed"]
    kind -->|"@v1, @v1.2, @v1.2.3"| pin["Pin<br/>major / minor / exact"]
    kind -->|"@stable, @main"| notver["NotAVersion<br/>a branch or channel"]

    any --> resolved
    pin --> match{"matches the<br/>installed version?"}
    match -->|yes| resolved["ResolvedPlugin"]
    match -->|no| mismatch["PluginVersionMismatch<br/>names what was asked for<br/>and what exists"]
    notver --> warn["resolves, with a warning<br/>that the suffix had no effect"]
    warn --> resolved
```

The `NotAVersion` branch is a deliberate compromise: `dtolnay/rust-toolchain@stable`
is a branch name, not a version, and pipelines ported from GitHub Actions
arrive with it. Failing would break them; ignoring it silently is what caused
[#50](https://github.com/copyleftdev/oxide-ci/issues/50). So it runs and says so.

## 4. Secrets, and where they get masked

Both the path secrets take into a step and the path their values take back out.
The masking step is not decoration: without it, delivering secrets to container
steps would turn every echoed value into a log leak.

```mermaid
flowchart LR
    subgraph inbound["Inbound"]
        flag["oxide run --secrets K=V"] --> cfg["ExecutorConfig.secrets"]
        cfg --> ectx["ExecutionContext"]
        ectx --> sctx["StepContext.secrets"]
        sctx --> env["container environment"]
        sctx --> creds["registry credentials<br/>password_secret resolved by name"]
    end

    subgraph outbound["Outbound"]
        stdout["step stdout / stderr"] --> mask["mask_secrets"]
        mask --> term["terminal"]
    end

    env -.->|"a step may echo one"| stdout
    creds -.->|"used to pull"| registry[("private registry")]
```

## 5. Spec and code, kept in agreement

Event payloads have two representations. `oxide-spec` exists so they cannot
drift apart quietly.

```mermaid
flowchart LR
    subgraph specside["spec/"]
        schema["schemas/*.yaml"]
        channel["channels/*.yaml"]
        message["messages/*.yaml"]
        index["_index.yaml"]
    end

    subgraph codeside["crates/"]
        types["oxide-core::events<br/>Rust payload types"]
        link["spec_link!<br/>type ↔ schema binding"]
        validator["SpecValidator"]
    end

    schema --> link
    types --> link
    link --> validator
    validator --> test["correlation tests<br/>fail on drift"]
    schema --> lint["make lint<br/>asyncapi validate"]
    channel --> lint
    message --> lint
    index --> lint
```

## 6. Event flow at runtime

Distributed mode: components communicate over NATS JetStream using the event
shapes the spec defines.

```mermaid
flowchart LR
    api["oxide-api"] -->|run/queued| bus{{"NATS JetStream"}}
    bus -->|run/queued| sched["oxide-scheduler"]
    sched -->|job/dispatched| bus
    bus -->|job/dispatched| agent["oxide-agent"]
    agent -->|step/started| bus
    agent -->|step/completed| bus
    bus -->|run/completed| api
    bus --> db[("PostgreSQL<br/>history")]
    agent --> cache[("MinIO / S3<br/>artifacts, cache")]
```

## See also

- [Testing Strategy](testing.md) — the tiers, and which of them can block a merge
- [Plugin System](plugins.md) — reference syntax and version semantics
- [Crate Ecosystem](crates.md) — what each crate is responsible for
