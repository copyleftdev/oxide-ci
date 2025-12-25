# Plugin System

Oxide CI supports extensions via a modular plugin system.

## Types of Plugins

### 1. Native Built-in Plugins
These are compiled directly into the binary for maximum performance and stability.
- **`git-checkout`**: Clones repositories.
- **`cache`**: Manages dependency caching (save/restore).
- **`docker-build`**: Builds Docker images.
- **`rust-toolchain`**: Installs/configures Rust toolchains.

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
