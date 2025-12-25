# Configuration Reference

Oxide CI pipelines are defined in YAML. The default filename is `pipeline.yaml` or `.oxide-ci/pipeline.yaml`.

## Pipeline Structure

```yaml
version: "1"
name: String
description: String (Optional)
timeout_minutes: Integer (Default: 60)
variables: { key: value }
stages: [ ... ]
```

## Stages

Stages organize jobs/steps. By default, stages run sequentially unless `depends_on` is unspecified (parallel) or specified explicitly.

```yaml
- name: build
  display_name: Build Stage
  depends_on: [other_stage_name]
  parallel: Boolean # Run steps in parallel?
  condition: "branch == 'main'"
  steps: [ ... ]
```

## Steps

Steps are the unit of execution.

```yaml
- name: Step Name
  # Use a plugin
  plugin: git-checkout
  with:
    repository: ...
  
  # OR run a command
  run: echo "Hello"
  shell: bash
  working_directory: ./src

  # Controls
  timeout_minutes: 30
  continue_on_error: false
  condition: { ... }
```

## Triggers

Triggers define when the pipeline runs (if supported by the scheduler).

```yaml
triggers:
  - push:
      branches: [main]
      paths: ["src/**"]
  - pull_request:
      branches: [main]
```
