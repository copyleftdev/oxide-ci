# Usage

Oxide CI is controlled via the `oxide` CLI.

## Running a Pipeline

To run a pipeline defined in a local YAML file:

```bash
oxide run .oxide-ci/pipeline.yaml
```

## Example Pipeline

Here is a simple example of a `pipeline.yaml`:

```yaml
name: Example Pipeline
stages:
  - name: build
    steps:
      - name: Build Project
        run: cargo build
  - name: test
    depends_on: [build]
    steps:
      - name: Run Tests
        run: cargo test
```

## Validating Configuration

You can validate your pipeline configuration before running:

```bash
oxide validate .oxide-ci/pipeline.yaml
```
