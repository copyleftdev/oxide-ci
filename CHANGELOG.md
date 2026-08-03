# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-03

First tagged release. Early alpha: APIs and pipeline syntax may change.

### Added
- **Engine**: Local pipeline execution with DAG stage resolution, parallel stages, matrix expansion, retries, conditions, and step outputs.
- **Plugins**: Native plugin system — `git-checkout`, `cache`, `docker-build`, `rust-toolchain` — each answering to a namespaced alias such as `oxide/docker-build`.
- **Plugin versions**: `uses: name@version` is resolved rather than ignored. `@v1` pins a major, `@v1.2` a minor, `@v1.2.3` an exact release, and a bare name or `@latest` takes what is installed.
- **CLI**: `oxide plugins list` shows the built-in plugins, their versions, and their aliases.
- **Dogfooding**: Self-hosting pipeline (`.oxide-ci/pipeline.yaml`) — Oxide CI builds and tests itself.
- **Compat**: GitHub Actions compatibility layer, including `uses:` as a spelling of `plugin:` and `file:` for `dockerfile:`.
- **Spec**: AsyncAPI 3.0 contract in `spec/`, with `spec_link!` correlation between Rust types and schemas.
- **Docs**: mdBook documentation and Wiki sync.

### Fixed
- **Plugin resolution**: `uses: docker-build@v1` failed with "Plugin not found" while the same message advertised `docker-build` as supported. Version suffixes are now parsed instead of being matched literally ([#50](https://github.com/copyleftdev/oxide-ci/issues/50)).
- **docker-build tags**: a `tags: |` block scalar kept its trailing newline and reached docker as a single argument. `tags` and `cache_from` now parse one entry per line, trimmed, ignoring blanks and comments.
- **docker-build parameters**: `cache_from` was accepted and silently discarded; it is now passed as `--cache-from`. Unknown parameters warn rather than disappearing.
- Integration tests (API drift, method renames).
- Crate cloning and fixture usage.

### Changed
- Licensed under MIT consistently across `LICENSE`, the workspace and all crate manifests, and the AsyncAPI spec.

[Unreleased]: https://github.com/copyleftdev/oxide-ci/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/copyleftdev/oxide-ci/releases/tag/v0.1.0
