//! End-to-end tests: the engine running real Rust pipelines in containers.
//!
//! What is specific to this ecosystem, and worth a slice of its own:
//!
//! * A compile step and a test step are genuinely different failure modes. A
//!   Rust pipeline can go red because the code did not build or because the
//!   tests disagreed with it, and the engine must attribute each correctly —
//!   `cargo` reports both as a non-zero exit.
//! * `cargo test` writes `target/` into the mounted workspace, so this also
//!   exercises the engine leaving build output behind for later stages.
//!
//! Hermetic tier: the fixture crate has no dependencies and runs with
//! `--offline`, so crates.io is never contacted and a registry outage cannot
//! turn the gate red.

use crate::e2e_support::{
    RUST_IMAGE, assert_pipeline_failed_at, assert_pipeline_passed, container_step, ensure_image,
    require_docker, run_pipeline, stage,
};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Outcomes the fixture crate can be generated with.
#[derive(Clone, Copy)]
enum RustFixture {
    /// Compiles, tests agree with the implementation.
    Passing,
    /// Compiles, but a test asserts the wrong answer.
    FailingTests,
    /// Does not compile at all.
    BrokenBuild,
}

/// Write a dependency-free Rust crate. No dependencies is what makes the
/// hermetic tier hermetic: `cargo --offline` never needs a registry.
fn write_rust_project(root: &Path, fixture: RustFixture) {
    fs::create_dir_all(root.join("src")).unwrap();

    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "widget"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    )
    .unwrap();

    let body = match fixture {
        RustFixture::BrokenBuild => {
            // Missing semicolon and an undefined symbol: fails to compile
            // rather than failing a test.
            r#"pub fn add(a: i64, b: i64) -> i64 {
    a + b + undefined_symbol()
}
"#
            .to_string()
        }
        RustFixture::Passing | RustFixture::FailingTests => {
            let expected = if matches!(fixture, RustFixture::Passing) {
                5
            } else {
                6
            };
            format!(
                r#"//! A deliberately small crate: enough to have a real test suite.

pub fn add(a: i64, b: i64) -> i64 {{
    a + b
}}

pub fn divide(a: i64, b: i64) -> Option<i64> {{
    if b == 0 {{ None }} else {{ Some(a / b) }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn adds() {{
        assert_eq!(add(2, 3), {expected});
    }}

    #[test]
    fn divides() {{
        assert_eq!(divide(9, 3), Some(3));
    }}

    #[test]
    fn division_by_zero_is_none() {{
        assert_eq!(divide(1, 0), None);
    }}
}}
"#
            )
        }
    };

    fs::write(root.join("src/lib.rs"), body).unwrap();
}

// ------------------------------------------------------------ hermetic tier --

#[test]
fn rust_pipeline_passes_in_a_container() {
    require_docker();
    ensure_image(RUST_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_rust_project(workspace.path(), RustFixture::Passing);

    let yaml = format!(
        r#"name: rust-e2e
version: "1"
stages:
  - name: build
    steps:
{}  - name: test
    depends_on: [build]
    steps:
{}"#,
        container_step("cargo-check", RUST_IMAGE, "cargo check --offline"),
        container_step("cargo-test", RUST_IMAGE, "cargo test --offline"),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_passed(&result, &["build", "test"]);

    // cargo wrote into the mounted workspace, so a later stage on the same
    // workspace can build on what an earlier one produced.
    assert!(
        workspace.path().join("target").is_dir(),
        "cargo build output should persist in the workspace"
    );
}

#[test]
fn failing_rust_tests_fail_the_pipeline_and_stop_it() {
    require_docker();
    ensure_image(RUST_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_rust_project(workspace.path(), RustFixture::FailingTests);

    let yaml = format!(
        r#"name: rust-e2e-failure
version: "1"
stages:
  - name: test
    steps:
{}  - name: package
    depends_on: [test]
    steps:
{}"#,
        container_step("cargo-test", RUST_IMAGE, "cargo test --offline"),
        container_step(
            "cargo-package",
            RUST_IMAGE,
            "cargo package --offline --no-verify"
        ),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_failed_at(&result, "test", "package");
}

#[test]
fn a_build_failure_is_attributed_to_the_build_stage() {
    require_docker();
    ensure_image(RUST_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_rust_project(workspace.path(), RustFixture::BrokenBuild);

    // Compilation failure and test failure are different things. The engine
    // must blame the stage that actually broke, not simply report that
    // something somewhere went wrong.
    let yaml = format!(
        r#"name: rust-e2e-build-failure
version: "1"
stages:
  - name: build
    steps:
{}  - name: test
    depends_on: [build]
    steps:
{}"#,
        container_step("cargo-check", RUST_IMAGE, "cargo check --offline"),
        container_step("cargo-test", RUST_IMAGE, "cargo test --offline"),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_failed_at(&result, "build", "test");
    assert!(
        !stage(&result, "build").success,
        "the build stage owns this failure"
    );
}

// -------------------------------------------------------------- canary tier --

/// Non-blocking: resolves and compiles a real dependency from crates.io.
#[test]
#[ignore = "canary: requires network and crates.io; run on a schedule, never as a gate"]
fn canary_rust_pipeline_with_real_dependencies() {
    require_docker();
    ensure_image(RUST_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_rust_project(workspace.path(), RustFixture::Passing);
    fs::write(
        workspace.path().join("Cargo.toml"),
        r#"[package]
name = "widget"
version = "0.1.0"
edition = "2021"

[dependencies]
serde_json = "1"
"#,
    )
    .unwrap();
    fs::create_dir_all(workspace.path().join("tests")).unwrap();
    fs::write(
        workspace.path().join("tests/dep_test.rs"),
        // Outer delimiter is `r##` because the fixture itself contains `"#`.
        r##"#[test]
fn serde_json_round_trips() {
    let value: serde_json::Value = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert_eq!(value["a"], 1);
}
"##,
    )
    .unwrap();

    let yaml = format!(
        r#"name: rust-e2e-canary
version: "1"
stages:
  - name: fetch
    steps:
{}  - name: test
    depends_on: [fetch]
    steps:
{}"#,
        container_step("cargo-fetch", RUST_IMAGE, "cargo fetch"),
        container_step("cargo-test", RUST_IMAGE, "cargo test"),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_passed(&result, &["fetch", "test"]);
}
