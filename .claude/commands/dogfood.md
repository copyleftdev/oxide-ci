---
description: Run Oxide CI on itself and triage what the engine reports
argument-hint: "[pipeline file, default .oxide-ci/dogfood.yaml]"
allowed-tools: Bash(cargo run -p oxide-cli:*), Bash(cargo build:*), Read, Grep, Glob
---

Pipelines available: !`ls .oxide-ci/`

Run the engine against itself:

```
cargo run -p oxide-cli -- run ${ARGUMENTS:-.oxide-ci/dogfood.yaml}
```

`dogfood.yaml` is the cheap smoke test. `pipeline.yaml` is the full build/test/quality run and needs a warm `target/` plus Docker for the integration stage — say so before starting it from cold rather than letting it run for many minutes silently.

When a stage fails, separate the two failure classes explicitly:

- **The engine is wrong** — bad DAG ordering, step output interpolation, plugin resolution, parallel stage handling, exit-code propagation. This is a bug in Oxide CI; capture the pipeline YAML and the engine's output and open a ticket.
- **The code under test is wrong** — clippy/test failures in the workspace. Fix those normally.

Report which class each failure falls into. Do not edit `.oxide-ci/*.yaml` to make a run pass without saying why the change is correct.
