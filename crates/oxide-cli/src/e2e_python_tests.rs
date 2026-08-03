//! End-to-end tests: the engine running real Python pipelines in containers.
//!
//! These are the tests that only an e2e can make. Unit tests prove the DAG
//! builder orders stages; these prove the engine actually *ran* them in that
//! order, in a container, against a real interpreter, and reported the truth
//! about what happened.
//!
//! Two tiers, by design:
//!
//! * **Hermetic (default)** — a generated Python library with stdlib-only
//!   `unittest` tests, executed in `python:3.12-slim` with no package
//!   installs. Deterministic: it cannot fail because PyPI had an incident or
//!   somebody else's `main` broke. This is the tier that is safe to gate on.
//! * **Canary (`#[ignore]`)** — the same shape with real dependency installs
//!   over the network. Informative, never blocking. Run it on a schedule.
//!
//! ```text
//! cargo test -p oxide-cli --features e2e -- --test-threads=1
//! cargo test -p oxide-cli --features e2e -- --ignored          # canary
//! ```
//!
//! Requires a Docker daemon. The feature flag is the opt-in, so a missing
//! daemon fails loudly rather than passing vacuously.

use crate::executor::{ExecutorConfig, PipelineResult, execute_pipeline};
use oxide_core::pipeline::PipelineDefinition;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const PY_IMAGE: &str = "python:3.12-slim";

// ---------------------------------------------------------------- harness --

fn require_docker() {
    let ok = Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    assert!(
        ok,
        "these tests need a running Docker daemon; they are behind the `e2e` feature so this is a \
         real failure, not a reason to skip"
    );
}

/// Pull an image before the pipeline runs.
///
/// The engine does not do this itself: `crates/oxide-runner/src/container.rs`
/// creates and starts containers but never calls Docker's image-create
/// endpoint, so a missing image surfaces as a container-creation error rather
/// than a pull. Until that is fixed, the harness pre-pulls so these tests
/// measure pipeline behavior instead of image-cache state.
fn ensure_image(image: &str) {
    let present = Command::new("docker")
        .args(["image", "inspect", image])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if present {
        return;
    }
    let pulled = Command::new("docker")
        .args(["pull", "--quiet", image])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(pulled, "failed to pull {image}");
}

/// Write a small, dependency-free Python library and its tests.
///
/// `passing` selects whether the test suite agrees with the implementation, so
/// the same project shape drives both the success and failure paths.
fn write_python_project(root: &Path, passing: bool) {
    fs::create_dir_all(root.join("src/calc")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();

    fs::write(
        root.join("src/calc/__init__.py"),
        r#"# A deliberately small library: enough to have a real test suite.

def add(a, b):
    return a + b


def divide(a, b):
    if b == 0:
        raise ZeroDivisionError("division by zero")
    return a / b
"#,
    )
    .unwrap();

    let expected_sum = if passing { 5 } else { 6 };
    fs::write(
        root.join("tests/test_calc.py"),
        format!(
            r#"import unittest
import sys, os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from calc import add, divide


class TestCalc(unittest.TestCase):
    def test_add(self):
        self.assertEqual(add(2, 3), {expected_sum})

    def test_divide(self):
        self.assertEqual(divide(9, 3), 3)

    def test_divide_by_zero_raises(self):
        with self.assertRaises(ZeroDivisionError):
            divide(1, 0)


if __name__ == "__main__":
    unittest.main()
"#
        ),
    )
    .unwrap();
}

/// A step that runs `command` inside the Python image.
///
/// The command goes in a block scalar so quoting inside it never has to be
/// escaped for YAML.
fn container_step(name: &str, command: &str) -> String {
    format!(
        r#"      - name: {name}
        run: |
          {command}
        environment:
          type: container
          container:
            image: {PY_IMAGE}
"#
    )
}

fn run_pipeline(yaml: &str, workspace: &Path) -> PipelineResult {
    let definition: PipelineDefinition =
        serde_yaml::from_str(yaml).expect("pipeline fixture should parse");
    let config = ExecutorConfig {
        workspace: workspace.to_path_buf(),
        variables: HashMap::new(),
        secrets: HashMap::new(),
        verbose: false,
    };
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(execute_pipeline(&definition, &config, None))
        .expect("executor should return a result, even for a failing pipeline")
}

fn stage_names(result: &PipelineResult) -> Vec<&str> {
    result
        .stages
        .iter()
        .map(|(name, _)| name.as_str())
        .collect()
}

fn stage<'a>(result: &'a PipelineResult, name: &str) -> &'a crate::executor::StageResult {
    result
        .stages
        .iter()
        .find(|(stage_name, _)| stage_name == name)
        .map(|(_, stage)| stage)
        .unwrap_or_else(|| {
            panic!(
                "stage `{name}` did not run; stages were {:?}",
                stage_names(result)
            )
        })
}

// ------------------------------------------------------------ hermetic tier --

#[test]
fn python_pipeline_passes_in_a_container() {
    require_docker();
    ensure_image(PY_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_python_project(workspace.path(), true);

    let yaml = format!(
        r#"name: python-e2e
version: "1"
stages:
  - name: compile
    steps:
{}  - name: test
    depends_on: [compile]
    steps:
{}"#,
        container_step("byte-compile", "python -m compileall -q src"),
        container_step("unit-tests", "python -m unittest discover -s tests -v"),
    );

    let result = run_pipeline(&yaml, workspace.path());

    assert!(
        result.success,
        "pipeline should pass: {:?}",
        stage_names(&result)
    );
    // The DAG was declared compile -> test; prove the engine honored it rather
    // than merely accepting the declaration.
    assert_eq!(stage_names(&result), vec!["compile", "test"]);
    for (name, stage) in &result.stages {
        assert!(stage.success, "stage `{name}` should have passed");
        for (step_name, step) in &stage.steps {
            assert!(step.success, "step `{step_name}` should have passed");
            assert_eq!(step.exit_code, 0, "step `{step_name}` exit code");
        }
    }
}

#[test]
fn failing_python_tests_fail_the_pipeline_and_stop_it() {
    require_docker();
    ensure_image(PY_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_python_project(workspace.path(), false);

    let yaml = format!(
        r#"name: python-e2e-failure
version: "1"
stages:
  - name: test
    steps:
{}  - name: package
    depends_on: [test]
    steps:
{}"#,
        container_step("unit-tests", "python -m unittest discover -s tests"),
        container_step("build-sdist", "python -c \"print('packaged')\""),
    );

    let result = run_pipeline(&yaml, workspace.path());

    // A green pipeline over red tests is the worst failure a CI engine can
    // have, so this assertion matters more than the happy path.
    assert!(
        !result.success,
        "pipeline must fail when the test suite fails"
    );

    let test_stage = stage(&result, "test");
    assert!(!test_stage.success, "the test stage must be marked failed");
    let (_, failing_step) = test_stage
        .steps
        .iter()
        .find(|(_, step)| !step.success)
        .expect("the failing step must be recorded");
    assert_ne!(
        failing_step.exit_code, 0,
        "failing step needs a non-zero exit code"
    );

    assert!(
        !stage_names(&result).contains(&"package"),
        "a stage depending on a failed stage must not run; stages were {:?}",
        stage_names(&result)
    );
}

#[test]
fn parallel_steps_overlap_in_wall_clock() {
    require_docker();
    ensure_image(PY_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_python_project(workspace.path(), true);

    // Two four-second sleeps: serial execution cannot finish under eight
    // seconds, so the threshold distinguishes real concurrency from a stage
    // that merely claims to be parallel.
    let yaml = format!(
        r#"name: python-e2e-parallel
version: "1"
stages:
  - name: fanout
    parallel: true
    steps:
{}{}"#,
        container_step("sleep-a", "python -c \"import time; time.sleep(4)\""),
        container_step("sleep-b", "python -c \"import time; time.sleep(4)\""),
    );

    let result = run_pipeline(&yaml, workspace.path());

    assert!(result.success, "parallel stage should pass");
    assert!(
        result.duration_ms < 7_000,
        "two 4s steps took {}ms; they ran serially",
        result.duration_ms
    );
}

#[test]
fn workspace_is_visible_inside_the_container() {
    require_docker();
    ensure_image(PY_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_python_project(workspace.path(), true);
    fs::write(workspace.path().join("marker.txt"), "mounted").unwrap();

    // The runner bind-mounts the workspace at /workspace. If that ever breaks,
    // every other test here would still pass against an empty directory —
    // Python would just find no tests to run.
    let yaml = format!(
        r#"name: python-e2e-mount
version: "1"
stages:
  - name: check
    steps:
{}"#,
        container_step(
            "read-workspace",
            "python -c \"assert open('/workspace/marker.txt').read() == 'mounted'\"",
        ),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert!(
        result.success,
        "the workspace must be mounted at /workspace inside the container"
    );
}

// -------------------------------------------------------------- canary tier --

/// Non-blocking: installs a real dependency from PyPI over the network.
///
/// This is the shape a pinned third-party repository would take. It is
/// `#[ignore]` on purpose — an outage at PyPI, or upstream breaking their own
/// main, must never turn this repository's gate red.
#[test]
#[ignore = "canary: requires network and PyPI; run on a schedule, never as a gate"]
fn canary_python_pipeline_with_real_dependencies() {
    require_docker();
    ensure_image(PY_IMAGE);

    let workspace = TempDir::new().unwrap();
    write_python_project(workspace.path(), true);
    fs::write(
        workspace.path().join("tests/test_property.py"),
        r#"import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from hypothesis import given, strategies as st
from calc import add


@given(st.integers(), st.integers())
def test_add_is_commutative(a, b):
    assert add(a, b) == add(b, a)
"#,
    )
    .unwrap();

    let yaml = format!(
        r#"name: python-e2e-canary
version: "1"
stages:
  - name: install
    steps:
{}  - name: test
    depends_on: [install]
    steps:
{}"#,
        // Each step gets a fresh container, so anything installed into the
        // image is gone by the next step. Only the mounted workspace persists,
        // which is why the install targets it explicitly.
        container_step(
            "pip-install",
            "pip install --quiet --root-user-action=ignore --target /workspace/.pydeps pytest hypothesis",
        ),
        container_step(
            "property-tests",
            "PYTHONPATH=/workspace/.pydeps python /workspace/.pydeps/bin/pytest tests -q",
        ),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert!(
        result.success,
        "canary pipeline failed: {:?}",
        stage_names(&result)
    );
}
