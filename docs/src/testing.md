# Testing Strategy

Four tiers, distinguished by one question: **what is allowed to turn the gate
red?** A gate that fails for reasons outside the repository trains people to
ignore it, so the tiers that block are exactly the tiers that are hermetic.

## 1. The tiers

```mermaid
flowchart TB
    subgraph blocking["Can block a merge"]
        unit["<b>Unit</b> · milliseconds<br/>inline #91;cfg#40;test#41;#93; modules<br/>pure logic: version specs, image references,<br/>DAG building, interpolation"]
        integ["<b>Integration</b> · seconds · needs Docker<br/>crates/oxide-tests, feature: integration<br/>testcontainers: Postgres, NATS, MinIO<br/>real dependencies, still deterministic"]
        e2e["<b>End-to-end</b> · ~15s · needs Docker<br/>crates/oxide-cli, feature: e2e<br/>real pipelines, real containers,<br/>real toolchains, no package registries"]
    end

    subgraph nonblocking["Never blocks"]
        canary["<b>Canary</b> · minutes · needs network<br/>same shapes, live registries<br/>marked #91;ignore#93;, run on a schedule"]
    end

    unit --> integ --> e2e -.->|"same pipelines,<br/>real dependencies"| canary
```

The canary tier exists because the question *"does Oxide still work against a
live PyPI, crates.io, and npm?"* is worth answering daily and worth **nobody's**
merge being blocked on. An upstream outage is information, not a defect in this
repository.

## 2. What the end-to-end tier proves

Unit tests prove the DAG builder orders stages. End-to-end tests prove the
engine *ran* them in that order, in a container, against a real toolchain, and
reported the truth about what happened.

```mermaid
flowchart LR
    subgraph shared["e2e_support"]
        harness["fixtures · container steps<br/>assert_pipeline_passed<br/>assert_pipeline_failed_at"]
    end

    harness --> py["<b>Python</b><br/>stdlib unittest<br/>python:3.12-slim"]
    harness --> rs["<b>Rust</b><br/>no-dependency crate<br/>cargo --offline"]
    harness --> ts["<b>TypeScript</b><br/>node type stripping<br/>node:22-alpine"]
    harness --> reg["<b>Registry</b><br/>registry:2 with htpasswd"]

    py --> pyc["parallel overlap<br/>workspace mount<br/>secrets delivery"]
    rs --> rsc["build vs test<br/>failure attribution"]
    ts --> tsc["cross-file<br/>module resolution"]
    reg --> regc["authenticated pull<br/>anonymous pull denied"]
```

Each ecosystem covers what is specific to its toolchain. Engine-level behavior —
parallel overlap, the workspace bind mount — is asserted once, in the Python
slice, rather than repeated three times for no extra information.

**Hermetic means hermetic.** Python runs stdlib `unittest` with no `pip
install`. Rust builds a dependency-free crate with `--offline`. TypeScript runs
`.ts` directly through Node's built-in type stripping, so there is no `npm
install`. None of the blocking tiers can be broken by a package registry.

## 3. Every negative test is mutation-tested

A test that cannot fail is worse than no test, because it reports safety it
does not provide. So each failure-path assertion is verified by breaking the
thing it watches and confirming it notices.

```mermaid
flowchart LR
    write["write the assertion"] --> green["suite green"]
    green --> mutate["break what it watches<br/>passing fixture, serial execution,<br/>wrong password, reverted fix"]
    mutate --> check{"does that test<br/>and only that test<br/>fail?"}
    check -->|yes| keep["keep it"]
    check -->|no| rewrite["the test was watching nothing<br/>rewrite it"]
    rewrite --> write
```

Mutations that have been run against this suite: flipping a failing fixture to
passing, setting `parallel: true` to `false`, supplying the wrong registry
password, and reverting the secrets-plumbing fix. In each case exactly the
corresponding test failed.

## 4. When each tier runs

```mermaid
flowchart TD
    edit["edit a file"] --> hook["editor / agent hook<br/>rustfmt on save"]
    hook --> commit["git commit"]
    commit --> pre["pre-commit<br/>fmt · clippy -D warnings · check · test"]
    pre -->|fails| fix["fix and retry"] --> commit
    pre -->|passes| gate["make verify<br/>same checks + e2e compile check<br/>stamps a ledger for the working tree"]
    gate --> e2etier["make e2e<br/>12 hermetic tests, ~15s"]
    e2etier --> merge["merge to main"]
    merge --> dog["dogfood pipeline<br/>oxide run .oxide-ci/pipeline.yaml"]
    sched["scheduled"] --> canary["make e2e-canary<br/>live registries, non-blocking"]
```

`make verify` is the single definition of the gate. `.pre-commit-config.yaml`
and `.oxide-ci/pipeline.yaml` run the same checks, so there is one answer to
"is this green?" rather than three that can disagree.

## 5. Commands

| Intent | Command |
|---|---|
| Full gate before calling something done | `make verify` |
| Fast inner loop | `cargo check -p <crate>` |
| Unit tests only | `cargo test --workspace --lib --bins` |
| Integration tests (Docker) | `cargo test -p oxide-tests --features integration` |
| End-to-end (Docker, minutes on a cold image cache) | `make e2e` |
| Canary against live registries | `make e2e-canary` |
| Validate the AsyncAPI spec | `make lint` |

`--lib --bins` matters: `oxide-cli` is a bin-only crate, so `--lib` alone
silently skips every test in it.

## 6. Conventions

- Unit tests live beside the code in `#[cfg(test)] mod tests`.
- Cross-crate tests live in `crates/oxide-tests/tests/`.
- Use `wiremock` for external HTTP and `testcontainers` for real services.
  Never mock `oxide-core` types — implement the port trait as a test double.
- Test the error path, not only the happy path. A test that can only pass is
  not evidence.
- Fixtures that a test generates belong in a tempdir, not in the repository.

## See also

- [System Diagrams](diagrams.md) — how the pieces fit together
- [Contributing](contributing.md) — workflow and commit conventions
