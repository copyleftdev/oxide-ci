# Installation

## Prerequisites

- **Rust**: Ensure you have a recent version of Rust installed (1.70+ recommended).
- **Docker**: Required for running containerized steps and integration tests.

## Building from Source

clone the repository and build:

```bash
git clone https://github.com/copyleftdev/oxide-ci.git
cd oxide-ci
cargo build --release
```

The binary will be located at `target/release/oxide`.

## Installation (Local)

You can install the CLI locally from source (as it is not yet on crates.io):

```bash
cargo install --path crates/oxide-cli
```
