# Plugin System

Oxide CI supports extensions via a modular plugin system.

## Types of Plugins

### 1. Native Built-in Plugins
These are compiled directly into the binary for maximum performance and stability.

| Plugin | Version | Also answers to | Purpose |
|---|---|---|---|
| `git-checkout` | 1.0.0 | `oxide/checkout` | Clones repositories. |
| `cache` | 1.0.0 | `oxide/cache` | Manages dependency caching (save/restore). |
| `docker-build` | 1.0.0 | `oxide/docker-build` | Builds Docker images. |
| `rust-toolchain` | 1.0.0 | `dtolnay/rust-toolchain` | Installs/configures Rust toolchains. |

Run `oxide plugins list` to see what your binary actually ships with — that
output is generated from the registry, so it can never drift from this table.

### 2. WASM Plugins (Beta)
Oxide CI can load WebAssembly modules to extend functionality dynamically. This allows for safe, sandboxed execution of third-party plugins.

## Using a Plugin

In `pipeline.yaml`:

```yaml
steps:
  - name: Checkout
    plugin: git-checkout
    with:
      repository: https://github.com/user/repo.git
```

`uses:` is an accepted spelling of `plugin:`, for pipelines ported from GitHub
Actions.

## Plugin Versions

A plugin reference is `name[@version]`, using the same spelling as GitHub
Actions:

| Reference | Resolves to |
|---|---|
| `docker-build` | whatever version is installed |
| `docker-build@latest` | whatever version is installed |
| `docker-build@v1` | any `1.x` |
| `docker-build@v1.2` | any `1.2.x` |
| `docker-build@v1.2.3` | exactly `1.2.3` |

The `v` prefix is optional, and a three-component pin means *exactly* that
release — not cargo's caret range.

Asking for a version that isn't installed fails the step with a message naming
what you asked for and what exists:

```
✗ Plugin version not available: docker-build@v99
  (docker-build is installed at 1.0.0; `@v99` does not match it. Try docker-build@v1.)
```

A suffix that isn't a version at all — a git branch or toolchain channel such as
`dtolnay/rust-toolchain@stable` — still runs, so pipelines copied from GitHub
Actions work, but warns that the suffix had no effect:

```
⚠ `rust-toolchain@stable` — `stable` is not a version, so it was ignored;
  rust-toolchain is installed at 1.0.0. Use @v1 to pin the major version.
```

Built-in plugins follow semantic versioning: the major version changes when a
parameter changes meaning or is removed, the minor when one is added.

## The `docker-build` Plugin

| Parameter | Meaning |
|---|---|
| `dockerfile` (or `file`) | Path to the Dockerfile. Default `Dockerfile`. |
| `context` | Build context. Default `.`. |
| `tags` | One tag, a list, or a block scalar with one tag per line. |
| `cache_from` | Image references passed to `docker build --cache-from`. |

`tags` and `cache_from` accept a YAML block scalar, where each line is one
entry. Blank lines and lines starting with `#` are ignored:

```yaml
- name: build
  uses: docker-build@v1
  with:
    file: Dockerfile
    tags: |
      # one per line, comments allowed
      ${{ REGISTRY }}/app:latest
      ${{ REGISTRY }}/app:${{ sha }}
```

Any parameter the plugin doesn't understand is logged as a warning rather than
silently ignored.
