#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "synchronous CLI tests control child processes and isolated reports"
)]
#![expect(
    unused_crate_dependencies,
    reason = "this process-boundary target uses only the dependencies required by its own command"
)]

//! Exercise development commands through their process boundary.
//!
//! The tests inspect command help, validate percentages and source scopes, and
//! reject malformed options before any generated report is replaced. Isolated
//! sentinel files expose accidental cleanup. Release validation also confirms
//! that the default command prints the shared semver without publishing
//! artifacts.

use std::process::Command;

/// The development command lists supported commands without running a build.
#[test_case::test_case("coverage"; "coverage command")]
#[test_case::test_case("site"; "site command")]
#[test_case::test_case("lint"; "lint command")]
#[test_case::test_case("bench"; "bench command")]
#[test_case::test_case("release"; "release command")]
fn help_lists_development_commands(command: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--help")
        .output()
        .expect("run xtask help");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(help.contains(command), "missing command {command}");
}

/// Invalid coverage percentages fail before replacing instrumented outputs.
#[test_case::test_case("0junk"; "trailing text")]
#[test_case::test_case("100.1"; "above maximum")]
#[test_case::test_case("-1"; "negative")]
#[test_case::test_case("NaN"; "not a number")]
#[test_case::test_case("1e2"; "exponent")]
#[test_case::test_case(".5"; "missing integer digits")]
#[test_case::test_case("1."; "missing fractional digits")]
fn rejects_invalid_percentage_before_cleanup(value: &str) {
    let directory = tempfile::tempdir().expect("coverage output fixture");
    // Existing build output must survive percentage rejection.
    let sentinel = directory.path().join("debug/sentinel");
    std::fs::create_dir_all(sentinel.parent().expect("sentinel directory"))
        .expect("create old output");
    std::fs::write(&sentinel, "retained").expect("write old output");
    // Validation must finish before the child reaches generated-output cleanup.
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["coverage", "--min-lines", value])
        .env("COVERAGE_TARGET_DIR", directory.path())
        .output()
        .expect("run coverage CLI");
    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(sentinel).expect("retained sentinel"),
        "retained"
    );
}

/// Missing option values and unavailable source directories preserve old
/// reports.
#[test_case::test_case(&["--min-lines"]; "missing percentage")]
#[test_case::test_case(&["--path"]; "missing path")]
#[test_case::test_case(&["--path", "/missing-dylints-review-source"]; "nonexistent directory")]
fn rejects_invalid_options_before_cleanup(arguments: &[&str]) {
    let directory = tempfile::tempdir().expect("coverage output fixture");
    // Existing reports must survive malformed options and invalid scopes.
    let sentinel = directory.path().join("report/sentinel");
    std::fs::create_dir_all(sentinel.parent().expect("sentinel directory"))
        .expect("create old report");
    std::fs::write(&sentinel, "retained").expect("write old report");
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("coverage")
        .args(arguments)
        .env("COVERAGE_TARGET_DIR", directory.path())
        .output()
        .expect("run coverage CLI");
    // Rejection must preserve the existing report sentinel.
    assert!(!output.status.success());
    assert!(sentinel.is_file());
}

/// Coverage rejects directories containing only deliberate fixture sources.
#[test]
fn rejects_fixture_only_scope_before_cleanup() {
    let directory = tempfile::tempdir().expect("scope fixture");
    // Intentional UI inputs are excluded from executable-source scopes.
    let scope = directory.path().join("ui");
    std::fs::create_dir(&scope).expect("create UI directory");
    std::fs::write(scope.join("main.rs"), "fn main() {}\n").expect("write fixture");
    // Validation must finish before the child reaches generated-output cleanup.
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["coverage", "--path"])
        .arg(scope)
        .env("COVERAGE_TARGET_DIR", directory.path().join("coverage"))
        .output()
        .expect("run coverage CLI");
    assert!(!output.status.success());
    // A rejected scope must not create or replace a report directory.
    assert!(!directory.path().join("coverage").exists());
}

/// A workspace-root target is rejected before any coverage process starts.
#[test]
fn rejects_workspace_target() {
    // The relative target resolves to the workspace rather than a dedicated output.
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["coverage", "--target-dir", "."])
        .env_remove("COVERAGE_TARGET_DIR")
        .output()
        .expect("unsafe coverage target");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("dedicated coverage directory"));
}

/// The default release command reports the shared version without writing Git
/// state.
#[test]
fn release_defaults_to_version_validation() {
    // The absence of publish selects read-only semver validation.
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("release")
        .output()
        .expect("validate release version");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("version output");
    let version = text.trim().strip_prefix('v').expect("Git tag prefix");
    let _version: semver::Version = version.parse().expect("valid shared semver");
}
