# CLI Reference

The `oxide` CLI is the primary interface for Oxide CI.

## Commands

### Global Options
- `-h, --help`: Print help information.
- `-V, --version`: Print version information.

### `oxide run`
Trigger a pipeline run locally or remotely.

```bash
oxide run [OPTIONS] <FILE>
```
- `<FILE>`: Path to `pipeline.yaml`.

### `oxide validate`
Validate the syntax and structure of a pipeline configuration.

```bash
oxide validate <FILE>
```

### `oxide secrets`
Manage secrets stored in the vault.

```bash
oxide secrets set <KEY> <VALUE>
oxide secrets get <KEY>
oxide secrets list
```

### `oxide cache`
Manage the build cache.

```bash
oxide cache clean
oxide cache stats
```

### `oxide init`
Initialize a new pipeline configuration in the current directory.
