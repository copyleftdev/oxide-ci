# Oxide CI

Event-driven CI/CD engine in Rust. Cargo workspace, 18 crates, edition 2024, alpha (v0.1.0).
Hexagonal architecture; NATS JetStream event bus; PostgreSQL/SQLx; Axum HTTP+WS; WASM plugins via Extism.

## Non-negotiables

1. **Spec first.** Every event payload must match a schema in `spec/schemas/*.yaml`. Changing an event without changing the spec (or vice versa) is a bug.
2. **`oxide-core` has no external integrations.** Domain types, traits (ports), errors only. Adapters (`oxide-db`, `oxide-nats`, `oxide-cache`, `oxide-secrets`, …) implement core's ports and hold zero business logic.
3. **The gate must pass before a change is done**: `make verify` (fmt + clippy `-D warnings` + workspace lib tests). A Stop hook blocks the turn if Rust changed and the gate hasn't passed for that exact tree.
4. **Every change traces to a GitHub issue.** Conventional commit with `Refs #N` / `Closes #N`.

## Commands

| Task | Command |
|---|---|
| Full gate (do this before saying "done") | `make verify` or `/gate` |
| Fast type check | `cargo check --workspace` |
| Unit tests | `cargo test --workspace --lib` |
| Integration tests (needs Docker) | `cargo test -p oxide-tests --features integration` |
| End-to-end container tests (needs Docker) | `make e2e` |
| E2E canary vs live registries (non-blocking) | `make e2e-canary` |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format | `cargo fmt --all` |
| Validate AsyncAPI spec | `make lint` (`npx asyncapi validate spec/asyncapi.yaml`) |
| Dogfood the engine on itself | `cargo run -p oxide-cli -- run .oxide-ci/dogfood.yaml` |

There is **no GitHub Actions CI** — it was removed deliberately (commit `a600fe9`); CI is dogfooded through `.oxide-ci/*.yaml`. Don't add workflows to `.github/workflows/` without being asked.

## Crate map

```
oxide-core       domain types, ports, errors — zero external deps
oxide-spec       AsyncAPI <-> Rust correlation (spec_link!, SpecValidator)
oxide-api        Axum HTTP/WebSocket server
oxide-scheduler  DAG resolution, job dispatch
oxide-agent      distributed execution worker
oxide-runner     step execution
oxide-plugins    plugin host: docker, git, cache, rust_toolchain, WASM registry
oxide-nats       event bus adapter          oxide-db        PostgreSQL adapter
oxide-cache      artifact/dep cache         oxide-secrets   secret backends
oxide-auth       authn/z                    oxide-licensing licensing
oxide-billing    billing                    oxide-notify    notifications
oxide-trace      OTel/tracing setup         oxide-cli       `oxide` binary
oxide-tests      integration harness (testcontainers: postgres, nats, minio)
```

## Conventions

**Types** — newtype IDs (`pub struct PipelineId(Uuid)`); derive `Debug, Clone, Serialize, Deserialize`; `#[serde(rename_all = "snake_case")]` on enums; `#[non_exhaustive]` on public enums that may grow.

**Errors** — `thiserror` in `oxide-core/src/error.rs`; return `Result<T, Error>`, never `Box<dyn Error>`; log at the boundary, not deep in the stack. `anyhow` only in binaries.

**Async** — `async fn` over `-> impl Future`; `async-trait` for trait methods; `tokio::select!` for concurrency; handle cancellation.

**Deps** — always `workspace = true` in crate manifests; add the version once in the root `Cargo.toml`.

**Tests** — unit tests inline in the module; cross-crate tests in `crates/oxide-tests/tests/`; `#[tokio::test]` for async; `wiremock` for external HTTP (Stripe, Keygen, Vault); `testcontainers` for Postgres/NATS; never mock `oxide-core` types.

**Docs** — `//!` module header on every file; `///` on public items.

## Adding an event (the common change)

1. Schema in `spec/schemas/*.yaml` → reference it from `spec/schemas/_index.yaml`.
2. Channel + message in `spec/channels/`, `spec/messages/` and their `_index.yaml`.
   Channel `{domain}/{event}`, message `PascalCase`, payload `{Message}Payload`, past-tense verbs.
3. Rust type in `oxide-core/src/events.rs`.
4. `oxide_spec::spec_link!(Type, schema = "...", file = "schemas/....yaml")` + a correlation test.
5. `make lint && make verify`.

`/new-event` runs this end to end.

## Git workflow (solo dev)

Branch `feat/issue-N-slug` or `fix/issue-N-slug` from latest `main`; merge straight to `main` (no PR); delete the branch. Commits are conventional and atomic: `feat(oxide-api): add pipeline CRUD endpoints - Closes #5`. Never `git commit --no-verify` — pre-commit runs the same gate and a hook blocks the bypass.

## Gotchas

- `Cargo.lock` is gitignored; don't hand-edit it or commit it.
- No `target/` cache checked in — the first `cargo build` of a session is multi-minute. Prefer `cargo check -p <crate>` while iterating and run the full gate once.
- Integration tests need a running Docker daemon; they're behind the `integration` feature and skipped by default.
- E2E tests (`crates/oxide-cli/src/e2e_{python,rust,typescript,registry}_tests.rs` on `e2e_support.rs`, feature `e2e`) run real pipelines in containers across three ecosystems. Two tiers: hermetic tests are safe to gate on; canary tests hit live package registries and are `#[ignore]` so upstream outages can never turn the gate red.
- `.venv/` + `requirements.txt` exist only for `scripts/*.py` (logo/diagram generation), not for the product.
- `.windsurf/rules/` mirrors this file for Windsurf. This file is the source of truth; if you change a rule here that lives there too, update both.

## Project agents and commands

`/gate` verify · `/ticket N` work an issue end to end · `/new-event` add a spec event · `/dogfood` run Oxide on itself
Agents: `rust-reviewer` (diff review), `spec-correlator` (spec↔code drift), `test-author` (tests), `ticket-scribe` (gh issue hygiene).
