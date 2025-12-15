# Plugin Development

Plugins are WebAssembly modules using the [Extism](https://extism.org/) framework.

## Quick Start

```rust
use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Input {
    repository: String,
    ref_name: String,
}

#[derive(Serialize)]
struct Output {
    success: bool,
    message: String,
}

#[plugin_fn]
pub fn run(input: Json<Input>) -> FnResult<Json<Output>> {
    // Plugin logic here
    Ok(Json(Output {
        success: true,
        message: format!("Checked out {}", input.ref_name),
    }))
}
```

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Plugin Manifest

`plugin.yaml`:
```yaml
name: my-plugin
version: 1.0.0
description: My custom plugin
inputs:
  - name: repository
    required: true
  - name: ref_name
    default: main
outputs:
  - name: commit_sha
```

## Using Plugins

```yaml
steps:
  - name: checkout
    uses: my-plugin@v1
    with:
      repository: ${{ repository }}
      ref_name: ${{ branch }}
```

## Built-in Plugins

- `git-checkout@v1` - Git clone and checkout
- `cache@v1` - Cache save/restore
- `artifact@v1` - Upload/download artifacts
- `aws-auth@v1` - AWS OIDC authentication
