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
//! The engine-level behaviors — parallel overlap, the workspace bind mount —
//! are asserted here once rather than repeated per ecosystem; the other slices
//! cover what is specific to their toolchain.

use crate::e2e_support::{
    PY_IMAGE, assert_pipeline_failed_at, assert_pipeline_passed, container_step, require_docker,
    run_pipeline, run_pipeline_with_secrets, stage_names,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

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

// ------------------------------------------------------------ hermetic tier --

#[test]
fn python_pipeline_passes_in_a_container() {
    require_docker();

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
        container_step("byte-compile", PY_IMAGE, "python -m compileall -q src"),
        container_step(
            "unit-tests",
            PY_IMAGE,
            "python -m unittest discover -s tests -v"
        ),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_passed(&result, &["compile", "test"]);
}

#[test]
fn failing_python_tests_fail_the_pipeline_and_stop_it() {
    require_docker();

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
        container_step(
            "unit-tests",
            PY_IMAGE,
            "python -m unittest discover -s tests"
        ),
        container_step("build-sdist", PY_IMAGE, "python -c \"print('packaged')\""),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_failed_at(&result, "test", "package");
}

#[test]
fn parallel_steps_overlap_in_wall_clock() {
    require_docker();

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
        container_step(
            "sleep-a",
            PY_IMAGE,
            "python -c \"import time; time.sleep(4)\""
        ),
        container_step(
            "sleep-b",
            PY_IMAGE,
            "python -c \"import time; time.sleep(4)\""
        ),
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
            PY_IMAGE,
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

/// Non-blocking: installs real dependencies from PyPI over the network.
///
/// This is the shape a pinned third-party repository would take. It is
/// `#[ignore]` on purpose — an outage at PyPI, or upstream breaking their own
/// main, must never turn this repository's gate red.
#[test]
#[ignore = "canary: requires network and PyPI; run on a schedule, never as a gate"]
fn canary_python_pipeline_with_real_dependencies() {
    require_docker();

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
            PY_IMAGE,
            "pip install --quiet --root-user-action=ignore --target /workspace/.pydeps pytest hypothesis",
        ),
        container_step(
            "property-tests",
            PY_IMAGE,
            "PYTHONPATH=/workspace/.pydeps python /workspace/.pydeps/bin/pytest tests -q",
        ),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_passed(&result, &["install", "test"]);
}

#[test]
fn secrets_reach_container_steps() {
    require_docker();

    let workspace = TempDir::new().unwrap();
    write_python_project(workspace.path(), true);

    // Shell steps received the pipeline's secrets; container steps were handed
    // an empty map, so every secret silently vanished. Silently is the problem:
    // the step just behaves as though the credential were never configured.
    let mut secrets = HashMap::new();
    secrets.insert("API_TOKEN".to_string(), "s3cr3t-value".to_string());

    let yaml = format!(
        r#"name: python-e2e-secrets
version: "1"
stages:
  - name: check
    steps:
{}"#,
        container_step(
            "read-secret",
            PY_IMAGE,
            "python -c \"import os; assert os.environ['API_TOKEN'] == 's3cr3t-value', 'secret missing from container env'\"",
        ),
    );

    let result = run_pipeline_with_secrets(&yaml, workspace.path(), secrets);
    assert!(
        result.success,
        "secrets must reach the container environment; stages were {:?}",
        stage_names(&result)
    );
}
