#![expect(
    unused_crate_dependencies,
    reason = "integration tests use only their process-fixture dependencies"
)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the process test controls isolated filesystem and tool fixtures"
)]
//! Exercise coverage and release commands through their CLI and process
//! boundaries.
//!
//! One compiled Rust tool fixture supplies deterministic child outputs in
//! isolated
//! directories. The tests verify source-alias merging, report scope, preserved
//! diagnostics, failure propagation and immutable release operations. The
//! parent
//! process retains its own environment and tool configuration throughout each
//! run.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};
use tempfile::TempDir;

/// Compile one Rust fixture binary, reused across isolated test directories.
fn fixture_binary() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let directory = tempfile::tempdir().expect("fixture directory").keep();
        let output = directory.join("debug/examples/tool-fixture");
        let status = Command::new("cargo")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args([
                "build",
                "--quiet",
                "--example",
                "tool-fixture",
                "--target-dir",
            ])
            .arg(&directory)
            .env("CARGO_BUILD_BUILD_DIR", &directory)
            .status()
            .expect("compile fixture");
        assert!(status.success());
        output
    })
}

/// Measured source mappings contain two aliases and one unselected file.
fn fixture() -> TempDir {
    // Keep tool and source files private to this command invocation.
    let directory = tempfile::tempdir().expect("isolated fixture");
    let root = directory.path();
    install_tools(root);
    source_records(root);
    directory
}

/// Install isolated Cargo, compiler and LLVM process fixtures.
fn install_tools(root: &Path) {
    // LLVM tools live under the host directory reported by the fake compiler.
    let tools = root.join("lib/rustlib/fixture-host/bin");
    fs::create_dir_all(&tools).expect("tool directory");
    link_tools(root, &["cargo", "rustc"]);
    link_tools(&tools, &["llvm-cov", "llvm-profdata"]);
}

/// Link selected tool names to the compiled fixture without modifying PATH
/// globally.
fn link_tools(root: &Path, tools: &[&str]) {
    for tool in tools {
        std::os::unix::fs::symlink(fixture_binary(), root.join(tool)).expect("tool link");
    }
}

/// Construct source aliases and retain both canonical and raw report inputs.
fn source_records(root: &Path) {
    // The lexical alias resolves through an existing parent directory.
    fs::create_dir_all(root.join("src/nested")).expect("source directory");
    fs::write(root.join("src/shared.rs"), "fn measured() {}\n").expect("selected source");
    fs::write(root.join("other.rs"), "fn other() {}\n").expect("unselected source");
    // These three identities deliberately describe two physical source files.
    let first = root.join("src/shared.rs");
    let alias = root.join("src/nested/../shared.rs");
    let other = root.join("other.rs");
    write_lcov(root, &first, &alias, &other);
    write_summary(root, &[first, alias, other]);
}

/// Preserve conflicting alias hits and an unrelated file in measured LCOV
/// input.
fn write_lcov(root: &Path, first: &Path, alias: &Path, other: &Path) {
    // Display each lexical spelling exactly as an LLVM exporter would emit it.
    let first_display = first.display();
    let alias_display = alias.display();
    let other_display = other.display();
    let lcov = format!(
        "SF:{first_display}\nDA:10,0\nDA:20,0\nend_of_record\nSF:{alias_display}\nDA:10,4\nDA:30,0\nend_of_record\nSF:{other_display}\nDA:1,1\nend_of_record\n"
    );
    // Preserve the input independently from every generated output directory.
    fs::write(root.join("lcov-input.info"), lcov).expect("coverage records");
}

/// Retain raw mapping counts without requiring LLVM's optional gap fields.
fn write_summary(root: &Path, paths: &[PathBuf]) {
    // Raw mappings count each lexical alias independently.
    let metric = serde_json::json!({"count":2,"covered":1,"percent":50.0});
    let files: Vec<_> = paths
        .iter()
        .map(|path| serde_json::json!({"filename":path,"summary":{"lines":metric}}))
        .collect();
    let summary = serde_json::json!({"data":[{"files":files,"totals":{"lines":metric}}],"type":"llvm.coverage.json.export","version":"2.0.1"});
    let bytes = serde_json::to_vec(&summary).expect("serialize fixture");
    // The command parses this envelope rather than fixture-generated totals.
    fs::write(root.join("summary-input.json"), bytes).expect("raw summary");
}

/// Run an isolated report while preserving the parent process environment.
fn run(root: &Path, minimum: &str, mode: &str, is_scoped: bool) -> std::process::Output {
    // Environment changes apply only to this child and its selected source scope.
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    let _configured = command
        .args(["coverage", "--min-lines", minimum, "--target-dir"])
        .arg(root.join("coverage"))
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .env("FIXTURE_MODE", mode)
        .env("FIXTURE_VERSION", env!("CARGO_PKG_VERSION"))
        .env_remove("COVERAGE_TARGET_DIR");
    if is_scoped {
        let _configured = command.arg("--path").arg(root.join("src"));
    }
    command.output().expect("coverage process")
}

/// Produce one successful scoped report for focused artifact assertions.
fn scoped_report() -> TempDir {
    // A 33% threshold passes one covered line out of three canonical lines.
    let directory = fixture();
    assert_success(&run(directory.path(), "33", "", true));
    directory
}

/// Child failures retain their diagnostics in the assertion message.
fn assert_success(result: &std::process::Output) {
    let diagnostics = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{diagnostics}");
}

/// Decode one machine-readable report produced by the command.
fn read_report(root: &Path, name: &str) -> serde_json::Value {
    let bytes = fs::read(root.join("coverage/report").join(name)).expect("report file");
    serde_json::from_slice(&bytes).expect("report JSON")
}

/// Canonical totals merge aliases and preserve exact uncovered source lines.
#[test]
fn scoped_canonical_aliases() {
    // Read artifacts only after the process boundary succeeds.
    let directory = scoped_report();
    let canonical = read_report(directory.path(), "canonical_summary.json");
    assert_eq!(canonical["totals"]["lines"]["count"], 3);
    assert_eq!(canonical["totals"]["lines"]["covered"], 1);
    // The gap belongs to the same canonical source identity used by the threshold.
    let gaps = fs::read_to_string(directory.path().join("coverage/report/canonical_gaps.txt"))
        .expect("gaps");
    assert!(gaps.contains("shared.rs:30"));
}

/// Raw scoped reports retain each selected lexical mapping separately.
#[test]
fn scoped_raw_aliases() {
    let directory = scoped_report();
    let raw = read_report(directory.path(), "summary.json");
    assert_eq!(raw["data"][0]["files"].as_array().expect("files").len(), 2);
    assert_eq!(raw["data"][0]["totals"]["lines"]["count"], 4);
}

/// Browsable reports exclude unrelated sources and preserve LLVM diagnostics.
#[test]
fn scoped_browsable_reports() {
    // HTML and diagnostics come from the same measured process run.
    let directory = scoped_report();
    let root = directory.path();
    let report = root.join("coverage/report");
    assert!(report.join("index.html").exists());
    let diagnostics = fs::read_to_string(report.join("llvm-cov.stderr")).expect("warnings");
    assert!(diagnostics.contains("fixture LLVM diagnostic"));
    // LLVM receives an escaped filename, rather than a weakened report scope.
    let calls = fs::read_to_string(root.join("calls.txt")).expect("calls");
    assert!(calls.contains("other\\\\.rs"));
}

/// Threshold failures and test failures retain useful reports and remain
/// failures.
#[test_case::test_case("34", "", "below"; "threshold failure")]
#[test_case::test_case("0", "tests-fail", "cargo test"; "test failure")]
fn failures_preserve_reports(minimum: &str, mode: &str, message: &str) {
    // Reports remain useful even when the test or threshold boundary fails.
    let directory = fixture();
    let root = directory.path();
    let result = run(root, minimum, mode, true);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains(message));
    // Failure status must not discard the completed measurement artifact.
    assert!(root.join("coverage/report/canonical_summary.json").exists());
}

/// Distinct binary mappings retain each line reached by either compiled variant.
#[test]
fn merges_hits_from_each_object_variant() {
    // Complementary object variants must both contribute their measured hits.
    let directory = fixture();
    let root = directory.path();
    assert_success(&run(root, "100", "object-variants", true));
    // Canonical totals retain both executable lines rather than one chosen mapping.
    let summary: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("coverage/report/canonical_summary.json")).expect("canonical summary"),
    )
    .expect("summary JSON");
    assert_eq!(summary["totals"]["lines"]["count"], 2);
    assert_eq!(summary["totals"]["lines"]["covered"], 2);
}

/// Tool failures and missing build artifacts propagate without guessed reports.
#[test_case::test_case("wrapper-fail", "cargo"; "wrapper build failure")]
#[test_case::test_case("llvm-fail", "llvm-cov"; "LLVM failure")]
#[test_case::test_case("no-objects", "no reportable objects"; "missing objects")]
#[test_case::test_case("no-host", "no host triple"; "missing host")]
#[test_case::test_case("text-fail", "llvm-cov"; "text export failure")]
#[test_case::test_case("html-fail", "llvm-cov"; "HTML export failure")]
#[test_case::test_case("canonical-write-fail", "Is a directory"; "canonical summary write failure")]
fn tool_failures_are_visible(mode: &str, message: &str) {
    // Each tool failure has its own named test and diagnostic assertion.
    let directory = fixture();
    let result = run(directory.path(), "0", mode, false);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains(message));
}

/// The default unscoped command includes every selected test target.
#[test_case::test_case("--no-fail-fast"; "retains failing tests")]
#[test_case::test_case("--workspace"; "selects workspace")]
#[test_case::test_case("--lib"; "selects libraries")]
#[test_case::test_case("--bins"; "selects binaries")]
#[test_case::test_case("--tests"; "selects integration tests")]
fn unscoped_default_test_arguments(expected: &str) {
    // Inspect the actual Cargo invocation rather than reconstructed CLI defaults.
    let directory = fixture();
    let root = directory.path();
    assert_success(&run(root, "0", "", false));
    let arguments = fs::read_to_string(root.join("test-arguments.txt")).expect("test arguments");
    assert!(arguments.contains(expected));
}

/// Release publication preserves tags, retries matching tags and propagates API
/// failures.
#[test_case::test_case("new-release"; "creates new release")]
#[test_case::test_case("existing-tag"; "retries matching tag")]
#[test_case::test_case("existing-release"; "retains published release")]
#[test_case::test_case("conflicting-tag"; "rejects different tag commit")]
#[test_case::test_case("api-fail"; "propagates API failure")]
#[test_case::test_case("branch-race"; "preserves concurrently created branch")]
#[test_case::test_case("head-fail"; "propagates commit read failure")]
#[test_case::test_case("verify-spawn-fail"; "propagates tag process spawn failure")]
#[test_case::test_case("tag-fail"; "propagates tag creation failure")]
#[test_case::test_case("tag-push-fail"; "propagates tag push failure")]
#[test_case::test_case("branch-read-fail"; "propagates branch read failure")]
fn release_publication_is_idempotent_and_failure_aware(mode: &str) {
    let directory = fixture();
    let root = directory.path();
    link_tools(root, &["git", "gh"]);
    // The child alone receives fake Git and GitHub executables.
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["release", "--publish"])
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .env("FIXTURE_MODE", mode)
        .env("FIXTURE_VERSION", env!("CARGO_PKG_VERSION"))
        .output()
        .expect("release process");
    // Observe remote mutations after the release process has completed.
    let calls = fs::read_to_string(root.join("calls.txt")).expect("release calls");
    assert_release_result(mode, &result, &calls);
    // A branch created after discovery retains its owner's commit.
    if mode == "branch-race" {
        assert_eq!(
            fs::read_to_string(root.join("branch-owner.txt")).unwrap(),
            "owner"
        );
    }
}

/// Assert each release outcome separately from process setup.
fn assert_release_result(mode: &str, result: &std::process::Output, calls: &str) {
    // Rejections must precede every protected publication side effect.
    match mode {
        "conflicting-tag" => {
            assert!(!result.status.success());
            assert!(!calls.contains("push"));
            assert!(
                String::from_utf8_lossy(&result.stderr)
                    .contains("already identifies another commit")
            );
        }
        "api-fail" | "branch-race" | "head-fail" | "verify-spawn-fail" | "tag-fail"
        | "tag-push-fail" | "branch-read-fail" => {
            assert!(!result.status.success());
            assert!(!calls.contains("release\", \"create"));
        }
        // Successful retries preserve artifacts that already exist.
        _ => assert_successful_release(mode, result, calls),
    }
}

/// A failed base-manifest read rejects publication before any Git mutation.
#[test]
fn rejects_failed_base_read() {
    let directory = fixture();
    let root = directory.path();
    link_tools(root, &["git", "gh"]);
    // The base read must complete before publication can begin.
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "release",
            "--publish",
            "--base",
            "1111111111111111111111111111111111111111",
        ])
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .env("FIXTURE_MODE", "show-fail")
        .output()
        .expect("base failure process");
    let calls = fs::read_to_string(root.join("calls.txt")).expect("release calls");
    // A failed base read cannot reach tag, branch or release writes.
    assert!(!result.status.success());
    assert!(!calls.contains("push"));
    assert!(!calls.contains("release\", \"create"));
}

/// Existing tags are retried, while existing releases remain untouched.
fn assert_successful_release(mode: &str, result: &std::process::Output, calls: &str) {
    assert!(result.status.success());
    assert!(calls.contains("push"));
    // Existing releases and tags are retained rather than recreated.
    assert_eq!(calls.contains("create"), mode != "existing-release");
    // A matching local tag still needs its remote push.
    if mode == "existing-tag" {
        assert!(!calls.contains("\"tag\", \"-a\""));
    }
}

/// Version comparison runs before publication; invalid commits fail at ingress.
#[test]
fn release_checks_base_before_writes() {
    let directory = fixture();
    let root = directory.path();
    link_tools(root, &["git", "gh"]);
    // Compare the validated base before any release command can mutate Git.
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "release",
            "--base",
            "1111111111111111111111111111111111111111",
        ])
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .output()
        .expect("version comparison");
    assert!(result.status.success());
    // A successful comparison must leave publication operations unreachable.
    assert!(
        !fs::read_to_string(root.join("calls.txt"))
            .expect("comparison calls")
            .contains("push")
    );
}

/// Invalid commit identities cannot reach release child processes.
#[test_case::test_case("main"; "branch name")]
#[test_case::test_case("0000000000000000000000000000000000000000"; "zero SHA")]
#[test_case::test_case("../unsafe"; "relative path")]
#[test_case::test_case("-option"; "option spelling")]
fn release_rejects_invalid_base(source: &str) {
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["release", "--base", source])
        .output()
        .expect("invalid base");
    assert!(!result.status.success());
}

/// Equal and decreasing semvers fail before tag, branch or release writes.
#[test_case::test_case(env!("CARGO_PKG_VERSION"); "equal version")]
#[test_case::test_case("999.0.0"; "decreasing version")]
fn release_requires_a_version_increase(version: &str) {
    let directory = fixture();
    let root = directory.path();
    std::os::unix::fs::symlink(fixture_binary(), root.join("git")).expect("version tool");
    // Version rejection must happen before the publish branch executes.
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "release",
            "--publish",
            "--base",
            "1111111111111111111111111111111111111111",
        ])
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .env("FIXTURE_BASE_VERSION", version)
        .output()
        .expect("version comparison");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("must exceed base version"));
    // Both tag and push operations remain unreachable for invalid versions.
    let calls = fs::read_to_string(root.join("calls.txt")).expect("comparison calls");
    assert_eq!(
        (calls.contains("push"), calls.contains("tag")),
        (false, false)
    );
}

/// Each native command preserves its Cargo arguments and child failure status.
#[test_case::test_case("list", false; "listing")]
#[test_case::test_case("lint", false; "linting")]
#[test_case::test_case("bench", false; "quick benchmarks")]
#[test_case::test_case("bench", true; "full benchmarks")]
fn native_command_dispatch(task: &str, is_full: bool) {
    let directory = fixture();
    let root = directory.path();
    // Verify both successful dispatch and preserved child exit failures.
    for mode in ["", "command-fail"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
        let _configured = command
            .arg(task)
            .env("PATH", root)
            .env("FIXTURE_ROOT", root)
            .env("FIXTURE_MODE", mode);
        // Full measurement must bypass the quick-sweep argument.
        if is_full {
            let _configured = command.arg("--full");
        }
        let result = command.output().expect("native command");
        assert_eq!(result.status.success(), mode.is_empty());
    }
    // Check the actual operation selected by the CLI.
    let calls = fs::read_to_string(root.join("calls.txt")).expect("native calls");
    assert!(calls.contains(if task == "bench" {
        "lint_passes"
    } else {
        "dylint"
    }));
    // Quick and full benchmark modes retain distinct child arguments.
    if task == "bench" {
        assert_eq!(calls.contains("--quick"), !is_full);
    }
}

/// Catalog and book assembly reaches real process boundaries with isolated
/// output.
#[test]
fn site_command_assembles_catalog_and_book() {
    let directory = fixture();
    let root = directory.path();
    std::os::unix::fs::symlink(fixture_binary(), root.join("mdbook")).expect("book tool");
    // Every generated artifact belongs to this isolated destination.
    let destination = root.join("site");
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["site", "--out-dir"])
        .arg(&destination)
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .output()
        .expect("site command");
    assert_success(&result);
    // Catalog and book must both be assembled before the command succeeds.
    assert_eq!(
        fs::read_to_string(destination.join("index.html")).expect("catalog"),
        "catalog fixture"
    );
    assert_eq!(
        fs::read_to_string(destination.join("book/index.html")).expect("book"),
        "book fixture"
    );
}

/// A failed catalog or book process cannot publish a complete site.
#[test_case::test_case("command-fail", "cargo"; "catalog discovery failure")]
#[test_case::test_case("book-fail", "mdbook"; "book generation failure")]
fn site_propagates_tool_failure(mode: &str, tool: &str) {
    let directory = fixture();
    let root = directory.path();
    link_tools(root, &["mdbook"]);
    // Failed generation stays confined to this private destination.
    let destination = root.join("site");
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["site", "--out-dir"])
        .arg(&destination)
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .env("FIXTURE_MODE", mode)
        .output()
        .expect("failed site process");
    // The named tool failure prevents report assembly and command success.
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains(tool));
    assert!(!destination.join("coverage").exists());
}

/// A failed terminal write retains the completed coverage artifacts.
#[test]
fn coverage_propagates_output_failure() {
    let directory = fixture();
    let root = directory.path();
    // Keep a valid report setup while disconnecting only its terminal output.
    let (reader, writer) = std::os::unix::net::UnixStream::pair().expect("output pipe");
    drop(reader);
    let writer: std::os::fd::OwnedFd = writer.into();
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["coverage", "--target-dir"])
        .arg(root.join("coverage"))
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .env_remove("COVERAGE_TARGET_DIR")
        .stdout(std::process::Stdio::from(writer))
        .output()
        .expect("failed output process");
    // Output failure follows successful report generation without removing its proof.
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Broken pipe"));
    assert!(root.join("coverage/report/canonical_summary.json").exists());
}

/// Multiple report objects and explicit Cargo arguments remain observable.
#[test]
fn coverage_replaces_owned_outputs_and_keeps_explicit_arguments() {
    let directory = fixture();
    let root = directory.path();
    // Existing generated directories exercise replacement before measurement.
    fixture_old_coverage_outputs(root);
    let result = explicit_coverage(root);
    assert_success(&result);
    // Explicit test selection and additional objects reach the real child argv.
    let calls = fs::read_to_string(root.join("calls.txt")).expect("object calls");
    assert!(calls.contains("--object"));
    assert!(!root.join("coverage/report/sentinel").exists());
    assert!(
        fs::read_to_string(root.join("test-arguments.txt"))
            .expect("test arguments")
            .contains("xtask")
    );
}

/// Seed an owned generated report so its removal has a direct behavioral
/// assertion.
fn fixture_old_coverage_outputs(root: &Path) {
    // Only coverage-owned directories contain the replaced sentinel.
    for name in ["debug", "report", "profiles", "objects"] {
        let directory = root.join("coverage").join(name);
        fs::create_dir_all(&directory).expect("old generated output");
        let sentinel = directory.join("sentinel");
        fs::write(sentinel, "old output").expect("old generated file");
    }
}

/// Invoke one coverage run with explicit Cargo selection and multiple objects.
fn explicit_coverage(root: &Path) -> std::process::Output {
    // Both build directories and process tools remain isolated from the parent.
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .args(["coverage", "--target-dir"])
        .arg(root.join("coverage"))
        .args(["--", "-p", "xtask", "--tests"])
        .env("PATH", root)
        .env("FIXTURE_ROOT", root)
        .env("FIXTURE_MODE", "extra-objects")
        .env_remove("COVERAGE_TARGET_DIR")
        .output()
        .expect("explicit coverage command")
}
