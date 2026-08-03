//! End-to-end tests: the engine running real TypeScript pipelines in containers.
//!
//! What is specific to this ecosystem:
//!
//! * Node resolves imports across files at runtime, so this slice exercises a
//!   multi-file project rather than a single script — the workspace mount has
//!   to be coherent, not merely present.
//! * The hermetic tier runs `.ts` sources directly with Node's built-in type
//!   stripping and `node:test`, so it needs no `npm install` and never touches
//!   the npm registry. An npm outage cannot turn this gate red.
//!
//! Type *checking* needs `tsc`, which needs the registry — that lives in the
//! canary tier, where it belongs.

use crate::e2e_support::{
    NODE_IMAGE, assert_pipeline_failed_at, assert_pipeline_passed, container_step, require_docker,
    run_pipeline,
};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Write a small TypeScript library and its tests.
///
/// `passing` selects whether the test suite agrees with the implementation.
fn write_typescript_project(root: &Path, passing: bool) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();

    fs::write(
        root.join("package.json"),
        r#"{
  "name": "calc",
  "version": "0.1.0",
  "type": "module",
  "private": true
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("src/calc.ts"),
        r#"// A deliberately small library: enough to have a real test suite.

export function add(a: number, b: number): number {
  return a + b;
}

export function divide(a: number, b: number): number {
  if (b === 0) {
    throw new RangeError("division by zero");
  }
  return a / b;
}
"#,
    )
    .unwrap();

    let expected_sum = if passing { 5 } else { 6 };
    fs::write(
        root.join("tests/calc.test.ts"),
        format!(
            r#"import {{ test }} from 'node:test';
import assert from 'node:assert/strict';
import {{ add, divide }} from '../src/calc.ts';

test('add', () => {{
  assert.equal(add(2, 3), {expected_sum});
}});

test('divide', () => {{
  assert.equal(divide(9, 3), 3);
}});

test('divide by zero throws', () => {{
  assert.throws(() => divide(1, 0), RangeError);
}});
"#
        ),
    )
    .unwrap();
}

// ------------------------------------------------------------ hermetic tier --

#[test]
fn typescript_pipeline_passes_in_a_container() {
    require_docker();

    let workspace = TempDir::new().unwrap();
    write_typescript_project(workspace.path(), true);

    // Executing a module that only exports is a no-op that exits zero, so the
    // first stage is a real parse/strip check without needing tsc.
    let yaml = format!(
        r#"name: typescript-e2e
version: "1"
stages:
  - name: load
    steps:
{}  - name: test
    depends_on: [load]
    steps:
{}"#,
        container_step(
            "strip-types",
            NODE_IMAGE,
            "node --experimental-strip-types src/calc.ts"
        ),
        container_step(
            "unit-tests",
            NODE_IMAGE,
            "node --experimental-strip-types --test \"tests/*.test.ts\""
        ),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_passed(&result, &["load", "test"]);
}

#[test]
fn failing_typescript_tests_fail_the_pipeline_and_stop_it() {
    require_docker();

    let workspace = TempDir::new().unwrap();
    write_typescript_project(workspace.path(), false);

    let yaml = format!(
        r#"name: typescript-e2e-failure
version: "1"
stages:
  - name: test
    steps:
{}  - name: publish
    depends_on: [test]
    steps:
{}"#,
        container_step(
            "unit-tests",
            NODE_IMAGE,
            "node --experimental-strip-types --test \"tests/*.test.ts\""
        ),
        container_step("pack", NODE_IMAGE, "npm pack --dry-run"),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_failed_at(&result, "test", "publish");
}

// -------------------------------------------------------------- canary tier --

/// Non-blocking: installs TypeScript from the npm registry and type-checks for
/// real, which the hermetic tier deliberately cannot do.
#[test]
#[ignore = "canary: requires network and npm; run on a schedule, never as a gate"]
fn canary_typescript_pipeline_with_real_toolchain() {
    require_docker();

    let workspace = TempDir::new().unwrap();
    write_typescript_project(workspace.path(), true);
    fs::write(
        workspace.path().join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "es2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "strict": true,
    "noEmit": true
  },
  "include": ["src", "tests"]
}
"#,
    )
    .unwrap();

    let yaml = format!(
        r#"name: typescript-e2e-canary
version: "1"
stages:
  - name: install
    steps:
{}  - name: typecheck
    depends_on: [install]
    steps:
{}"#,
        container_step(
            "npm-install",
            NODE_IMAGE,
            "npm install --no-audit --no-fund typescript"
        ),
        container_step("tsc", NODE_IMAGE, "npx tsc --noEmit"),
    );

    let result = run_pipeline(&yaml, workspace.path());
    assert_pipeline_passed(&result, &["install", "typecheck"]);
}
