# Rust Fuzzing & Security Example

A Rust application with comprehensive fuzzing and security testing pipeline.

## Security Tools Used

| Tool | Purpose |
|------|---------|
| **cargo-audit** | Check for known vulnerabilities in dependencies |
| **cargo-deny** | Lint dependencies (licenses, duplicates, advisories) |
| **cargo-outdated** | Find outdated dependencies |
| **cargo-fuzz** | Fuzz testing with libFuzzer |
| **clippy** | Static analysis with pedantic lints |

## Fuzz Targets

```
fuzz/fuzz_targets/
├── fuzz_parse_user.rs      # JSON parsing fuzzer
├── fuzz_validate_email.rs  # Email validation fuzzer
└── fuzz_parse_config.rs    # Config parsing fuzzer
```

## Pipeline Stages

1. **setup** — Install security tools
2. **build** — Compile debug and release builds
3. **test** — Run unit tests
4. **security** — Audit, deny check, outdated check
5. **static-analysis** — Clippy with pedantic, format check
6. **fuzz-check** — Verify fuzz targets, initialize corpus

## Run Security Pipeline

```bash
oxide run
```

## Run Fuzzing Locally

```bash
# Install nightly and cargo-fuzz
rustup install nightly
cargo +nightly install cargo-fuzz

# Run a fuzzer for 60 seconds
cargo +nightly fuzz run fuzz_parse_user -- -max_total_time=60

# Run with more iterations
cargo +nightly fuzz run fuzz_validate_email -- -runs=100000
```

## Example Output

```
=== Security Audit (cargo-audit) ===
    Fetching advisory database from `https://github.com/RustSec/advisory-db`
    Scanning Cargo.lock for vulnerabilities (2 crate dependencies)
    0 vulnerabilities found

=== Clippy Lints ===
    Checking rust-fuzzing-example v0.1.0
    Finished dev [unoptimized + debuginfo]
```

## Library Functions Being Tested

| Function | Risk | Fuzz Target |
|----------|------|-------------|
| `parse_user_input` | JSON parsing, UTF-8 | `fuzz_parse_user` |
| `validate_email` | String manipulation | `fuzz_validate_email` |
| `parse_config` | Line parsing | `fuzz_parse_config` |
| `calculate_checksum` | Integer overflow | (in unit tests) |
