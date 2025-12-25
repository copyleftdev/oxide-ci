# Contributing to Oxide CI

Thank you for your interest in contributing to Oxide CI!

## Getting Started

1. Fork the repository.
2. Clone your fork: `git clone https://github.com/your-username/oxide-ci.git`
3. Install Rust (stable).

## Workflow

1. Create a branch: `git checkout -b feature/my-feature`
2. Make changes.
3. Test locally:
   - `cargo test --workspace`
   - `oxide run .oxide-ci/pipeline.yaml` (Dogfood)
4. Push and open a PR.

## Style

- Run `cargo fmt` before committing.
- Run `cargo clippy` to check for lints.
