#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the synchronous integration harness owns fixture files and child processes"
)]
#![allow(
    unused_crate_dependencies,
    reason = "the runner package embeds lint libraries that these tests do not import"
)]

//! Integration tests for the `sagan-lints` runner binary.
//!
//! Each test runs the compiled runner against a small fixture repository with
//! isolated cache directories. Most process-level tests replace Cargo with a
//! shell script through `--cargo-cmd`, so they exercise reporting, fix passes,
//! changed-range filtering, and cache handling without compiling Rust. The
//! remaining tests compile fixtures to cover bundled lint listing, the
//! embedded compiler driver, strict Clippy, and toolchain diagnostics.

use std::{
    ffi::OsStr,
    fs,
    os::unix::{
        ffi::OsStrExt as _,
        fs::{PermissionsExt as _, symlink},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use tempfile::TempDir;

/// Isolated cache and scratch directories for one runner invocation sequence.
struct Sandbox {
    /// Temporary root removed when the test finishes.
    root: TempDir,
}

/// Captured result of one runner process.
struct RunOutput {
    /// Process exit code, or `None` when a signal ended the process.
    code: Option<i32>,
    /// Combined standard output and standard error.
    text: String,
}

impl Sandbox {
    /// Resolve a path below the sandbox root.
    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    /// Build a runner command with sandboxed caches and no inherited runner state.
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sagan-lints"));
        let _command = command
            .args(args)
            .env("SAGAN_LINTS_CACHE_DIR", self.path("cache"))
            .env("XDG_CACHE_HOME", self.path("xdg-cache"))
            .env_remove("RUST_PERSONAL_LINTS_CACHE_DIR")
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES")
            .env_remove("SAGAN_LINTS_DRIVER");
        command
    }

    /// Run the runner with sandboxed cache directories.
    fn run(&self, args: &[&str]) -> RunOutput {
        output(&mut self.command(args))
    }

    /// Write a shell script that stands in for Cargo and return its `--cargo-cmd` value.
    fn fake_cargo(&self, body: &str) -> String {
        let script = self.path("fake-cargo.sh");
        write(&script, &format!("set -eu\n{body}\n"));
        format!("sh {}", text(&script))
    }

    /// Write files into a new Git repository and commit them.
    fn repository(&self, name: &str, files: &[(&str, &str)]) -> PathBuf {
        // Create every requested file before initializing Git so the baseline is complete.
        let repository = self.path(name);
        for (relative, contents) in files {
            write(&repository.join(relative), contents);
        }
        // Commit the fixture before the runner computes changed ranges.
        git(&repository, &["init", "-q"]);
        commit_all(&repository, "baseline");
        repository
    }

    /// Return the single runner-managed target directory below the sandbox cache.
    fn managed_target(&self) -> PathBuf {
        let mut targets = fs::read_dir(self.path("cache/targets"))
            .expect("the managed target root should exist")
            .map(|entry| entry.expect("target entry should be readable").path())
            .collect::<Vec<_>>();
        assert_eq!(targets.len(), 1, "{targets:?}");
        targets.remove(0)
    }
}

impl Default for Sandbox {
    /// Create a sandbox with a temporary root.
    fn default() -> Self {
        Self {
            root: tempfile::tempdir().expect("temporary directory should be available"),
        }
    }
}

/// Run one prepared command and capture its exit code and output.
fn output(command: &mut Command) -> RunOutput {
    let output = command.output().expect("the runner should start");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    RunOutput {
        code: output.status.code(),
        text,
    }
}

/// Return a UTF-8 path for command-line arguments.
fn text(path: &Path) -> &str {
    path.to_str().expect("sandbox path should be UTF-8")
}

/// Assert that the runner exited with `code` and printed every expected fragment.
fn assert_output(output: &RunOutput, code: i32, expected: &[&str]) {
    assert_eq!(output.code, Some(code), "{}", output.text);
    for fragment in expected {
        assert!(
            output.text.contains(fragment),
            "missing {fragment:?}:\n{}",
            output.text
        );
    }
}

/// Repository-relative path of a checked-in fixture.
fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

/// Write one file, creating its parent directories.
fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("fixture files should have a parent"))
        .expect("fixture directory should be writable");
    fs::write(path, contents).expect("fixture file should be writable");
}

/// Read one UTF-8 file.
fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("file should be readable")
}

/// Run one Git command in a repository and require success.
fn git(repository: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .status()
        .expect("git should start");
    assert!(status.success(), "git {args:?} should succeed");
}

/// Stage and commit every change with a synthetic identity.
fn commit_all(repository: &Path, message: &str) {
    git(repository, &["add", "--all"]);
    git(
        repository,
        &[
            "-c",
            "user.name=Synthetic Lint Fixture",
            "-c",
            "user.email=synthetic@example.invalid",
            "commit",
            "-qm",
            message,
        ],
    );
}

/// Build one Cargo `compiler-message` line with a primary span and an optional suggestion.
fn compiler_message(
    level: &str,
    code: Option<&str>,
    location: (&str, u64),
    suggestion: Option<(usize, usize, &str)>,
) -> String {
    let (file, line) = location;
    let code = code.map_or_else(
        || "null".to_owned(),
        |code| format!(r#"{{"code":"{code}"}}"#),
    );
    let suggestion = suggestion.map_or_else(String::new, |(start, end, replacement)| {
        format!(
            r#","byte_start":{start},"byte_end":{end},"suggested_replacement":"{replacement}","suggestion_applicability":"MachineApplicable""#
        )
    });
    format!(
        r#"{{"reason":"compiler-message","message":{{"level":"{level}","message":"{level} finding","code":{code},"spans":[{{"file_name":"{file}","line_start":{line},"line_end":{line},"is_primary":true{suggestion}}}],"children":[]}}}}"#
    )
}

/// Return a shell statement that prints fixed text through a quoted here-document.
fn emit(lines: &[String]) -> String {
    format!("cat <<'JSON'\n{}\nJSON", lines.join("\n"))
}

/// Write a single-package Cargo repository pinned to the runner's toolchain.
///
/// Commit the repository so the runner can compare later source changes.
fn package_repository(
    repository: &Path,
    name: &str,
    manifest_extra: &str,
    source_path: &str,
    source: &str,
) {
    // Keep the manifest, lockfile, toolchain, and source in one committed fixture.
    write(
        &repository.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{manifest_extra}"
        ),
    );
    write(
        &repository.join("Cargo.lock"),
        &format!(
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"{name}\"\nversion = \"0.1.0\"\n"
        ),
    );
    write(
        &repository.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"nightly-2026-07-15\"\n",
    );
    write(&repository.join(source_path), source);
    // Initialize Git only after every input file exists.
    git(repository, &["init", "-q"]);
    commit_all(repository, "baseline");
}

/// Run `--list-private-lints` against an empty workspace and require success.
fn list_private_lints(sandbox: &Sandbox) -> RunOutput {
    // Create an empty Git workspace so listing needs no package build.
    let repository = sandbox.repository(
        "empty",
        &[(
            "Cargo.toml",
            "[workspace]\nresolver = \"2\"\nmembers = []\n",
        )],
    );

    // List every default category and require the runner to succeed.
    let output = sandbox.run(&["--repo", text(&repository), "--list-private-lints"]);
    assert_eq!(output.code, Some(0), "{}", output.text);
    output
}

/// Collect the lint name of every lint crate below `dir` that has a `ui/` fixture directory.
///
/// A lint crate's package name, with `-` replaced by `_`, is its lint name.
fn ui_lint_crate_names(dir: &Path, names: &mut Vec<String>) {
    let manifest = dir.join("Cargo.toml");
    // Record this directory when it is a lint crate with UI fixtures.
    if dir.join("ui").is_dir() && manifest.is_file() {
        let contents = read(&manifest);
        let name = contents
            .lines()
            .find_map(|line| line.strip_prefix("name = \""))
            .and_then(|rest| rest.strip_suffix('"'))
            .expect("lint manifest should declare a package name");
        names.push(name.replace('-', "_"));
    }
    // Descend into child directories other than build output.
    for entry in fs::read_dir(dir).expect("lint directory should be readable") {
        let path = entry
            .expect("lint directory entry should be readable")
            .path();
        if path.is_dir() && path.file_name().is_some_and(|name| name != "target") {
            ui_lint_crate_names(&path, names);
        }
    }
}

/// The runner lists lints from repository, crate-specific, and style libraries.
#[test]
fn lists_bundled_private_lints() {
    // List the bundled lints from an empty workspace.
    let sandbox = Sandbox::default();
    let output = list_private_lints(&sandbox);

    assert_bundled_lints_are_listed(&output);
}

/// Verify representative lints from the bundled category libraries.
fn assert_bundled_lints_are_listed(output: &RunOutput) {
    // Sample one lint from a repository, a crate-specific, and a style library.
    for lint in [
        "dependency_full_semver_versions",
        "bevy_main_return_without_app_exit",
        "ad_hoc_from_str",
    ] {
        // Print the full listing so a missing name is easy to diagnose.
        assert!(
            output.text.contains(lint),
            "missing {lint}:\n{}",
            output.text
        );
    }
}

/// Every lint crate with UI fixtures is registered in a bundled category library.
#[test]
fn lists_every_lint_crate_with_ui_fixtures() {
    // Discover lint crates from the source tree instead of a hand-kept list.
    let mut names = Vec::new();
    ui_lint_crate_names(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("lints"),
        &mut names,
    );
    assert!(!names.is_empty(), "no lint crates with ui/ were found");

    // Ask the runner which lints its bundled category libraries register.
    let sandbox = Sandbox::default();
    let output = list_private_lints(&sandbox);

    // Match whole list rows so a lint name that prefixes another cannot hide a gap.
    let listed: Vec<&str> = output
        .text
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    // Report every unregistered crate at once.
    let missing: Vec<&String> = names
        .iter()
        .filter(|name| !listed.contains(&name.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "lint crates missing from --list-private-lints: {missing:?}"
    );
}

/// A listing dry run plans one direct compiler phase per default category.
#[test]
fn list_dry_run_plans_one_phase_per_default_category() {
    // Dry-run output is the public phase-order contract for the default category set.
    let sandbox = Sandbox::default();
    let output = sandbox.run(&[
        "--repo",
        text(&fixture("cargo-install-target")),
        "--list-private-lints",
        "--dry-run",
    ]);

    assert_eq!(output.code, Some(0), "{}", output.text);
    assert_eq!(
        output.text.matches(" rustc -W help\n").count(),
        9,
        "{}",
        output.text
    );
    assert!(
        output.text.contains("dylint-list-suspicious: "),
        "{}",
        output.text
    );
}

/// A dry run plans Clippy and the embedded driver without external Dylint helpers.
#[test]
fn dry_run_uses_embedded_driver() {
    assert_dry_run_uses_embedded_driver();
}

/// Verify that dry-run planning uses the embedded compiler driver.
fn assert_dry_run_uses_embedded_driver() {
    // Build a dry-run command with one private category and an explicit target.
    let sandbox = Sandbox::default();
    let target = sandbox.path("target");
    let output = sandbox.run(&[
        "--repo",
        text(&fixture("cargo-install-target")),
        "--no-workspace",
        "--fast",
        "--target-dir",
        text(&target),
        "--dylint-category",
        "suspicious",
        "--heartbeat-seconds",
        "0",
        "--dry-run",
    ]);

    // Confirm both Cargo phases use the embedded executable and target path.
    assert_eq!(output.code, Some(0), "{}", output.text);
    assert!(output.text.contains(" clippy "), "{}", output.text);
    assert!(output.text.contains(" check"), "{}", output.text);
    assert!(!output.text.contains("cargo-dylint"), "{}", output.text);
    assert!(!output.text.contains("dylint-link"), "{}", output.text);
    assert!(
        output
            .text
            .contains(&format!("CARGO_TARGET_DIR={}\n", text(&target))),
        "{}",
        output.text
    );
}

/// A bundled private lint rejects the fixture's string error result.
#[test]
fn bundled_private_lint_rejects_fixture() {
    // Select one bundled lint phase against the checked-in diagnostic fixture.
    let sandbox = Sandbox::default();
    let target = sandbox.path("target");
    let output = sandbox.run(&[
        "--repo",
        text(&fixture("cargo-install-target")),
        "--no-workspace",
        "--fast",
        "--target-dir",
        text(&target),
        "--skip-clippy",
        "--dylint-category",
        "suspicious",
        "--heartbeat-seconds",
        "0",
    ]);

    assert_eq!(output.code, Some(1), "{}", output.text);
    assert!(
        output.text.contains("string_error_result"),
        "{}",
        output.text
    );
}

/// The bundled restriction category executes the public Serde schema check at
/// a Cargo boundary.
///
/// Manifest for the temporary Serde fixture repository.
const SERDE_SCHEMA_FIXTURE_MANIFEST: &str = "[dependencies]\nserde = { version = \"1.0.229\", features = [\"derive\"] }\nschemars = \"1.2.1\"\n";

/// Source cases for the temporary Serde fixture repository.
const SERDE_SCHEMA_FIXTURE_SOURCE: &str = r"use schemars::JsonSchema;
use serde::Serialize;

/// A DTO that must have a schema.
#[derive(Serialize)]
pub struct MissingSchema {
    /// Stable fixture identifier.
    pub id: String,
}

/// An enum that must have a schema.
#[derive(Serialize)]
pub enum MissingSchemaEnum {
    /// One valid fixture state.
    Ready,
}

/// A DTO with both representations.
#[derive(JsonSchema, Serialize)]
pub struct CompleteSchema {
    /// Stable fixture identifier.
    pub id: String,
}

#[derive(Serialize)]
struct PrivateSchema {
    id: String,
}

#[derive(Clone)]
struct NotSerde;

fn helper() {}
";

/// Run the bundled public Serde schema lint through a temporary Cargo repository.
#[test]
fn bundled_public_serde_schema_derive_rejects_public_dto() {
    // Build a standalone repository so the test exercises the Cargo boundary.
    let sandbox = Sandbox::default();
    let repository = sandbox.path("serde-schema");
    package_repository(
        &repository,
        "serde_schema_fixture",
        SERDE_SCHEMA_FIXTURE_MANIFEST,
        "src/lib.rs",
        SERDE_SCHEMA_FIXTURE_SOURCE,
    );
    // Select the restriction category that embeds the public Serde lint.
    let output = sandbox.run(&[
        "--repo",
        text(&repository),
        "--no-workspace",
        "--fast",
        "--target-dir",
        text(&sandbox.path("target")),
        "--skip-clippy",
        "--dylint-category",
        "restriction",
        "--heartbeat-seconds",
        "0",
    ]);

    // Require both public violations while preserving the valid and private cases.
    assert_eq!(output.code, Some(1), "{}", output.text);
    assert!(
        output.text.contains("public_serde_schema_derive"),
        "{}",
        output.text
    );
    assert!(
        output.text.contains("MissingSchema")
            && output.text.contains("MissingSchemaEnum")
            && !output.text.contains("CompleteSchema"),
        "{}",
        output.text
    );
}

/// `--no-deps` still runs the embedded private lints on the selected package.
#[test]
fn no_deps_runs_private_lints() {
    let sandbox = Sandbox::default();
    let output = sandbox.run(&[
        "--repo",
        text(&fixture("cargo-install-target")),
        "--no-workspace",
        "--fast",
        "--no-deps",
        "--target-dir",
        text(&sandbox.path("target")),
        "--skip-clippy",
        "--dylint-category",
        "suspicious",
        "--heartbeat-seconds",
        "0",
    ]);

    assert_eq!(output.code, Some(1), "{}", output.text);
    // Private lints run through Cargo check, because Clippy would replace the compiler wrapper.
    assert!(output.text.contains("/cargo check\n"), "{}", output.text);
    assert!(
        output.text.contains("string_error_result"),
        "{}",
        output.text
    );
}

/// Assert that one crate-family category exposes its representative lint.
fn assert_category_registered(
    sandbox: &Sandbox,
    args: &[&str],
    category: &str,
    representative_lint: &str,
) {
    // Run the compiler driver in list mode for one explicit category.
    let mut command = driver(sandbox, args);
    let _command = command
        .env("SAGAN_LINTS_DRIVER_CATEGORIES", category)
        .env("SAGAN_LINTS_DRIVER_LIST", "1");
    let result = output(&mut command);
    let result_text = result.text;
    let has_representative_lint = result_text
        .lines()
        .any(|line| line.contains(representative_lint));

    // Preserve the category name and complete driver output in failures.
    assert_eq!(result.code, Some(0), "{category}: {result_text}");
    assert!(
        has_representative_lint,
        "{category} did not register {representative_lint}:\n{result_text}"
    );
}

/// Check the transport-oriented crate-family categories.
fn assert_transport_categories(sandbox: &Sandbox, args: &[&str]) {
    // Exercise each facade through the same compiler-driver list boundary.
    assert_category_registered(sandbox, args, "axum", "axum_nest_at_root");
    assert_category_registered(sandbox, args, "bevy", "bevy_borrowed_reborrowable");
    assert_category_registered(
        sandbox,
        args,
        "clap",
        "clap_allow_hyphen_values_without_num_args",
    );
    assert_category_registered(sandbox, args, "insta", "insta_allow_empty_glob");
    assert_category_registered(sandbox, args, "reqwest", "reqwest_blocking_in_async");
}

/// Check the schema, serialization, database, and enum crate-family categories.
fn assert_data_categories(sandbox: &Sandbox, args: &[&str]) {
    // Keep schema and representation families covered by separate assertions.
    assert_category_registered(sandbox, args, "schemars", "schemars_json_schema_ref_return");
    assert_category_registered(sandbox, args, "serde", "serde_all_fields_default");
    assert_category_registered(sandbox, args, "sqlx", "sqlx_assert_sql_safe");
    assert_category_registered(sandbox, args, "strum", "strum_enum_representation");
}

/// Check the test, error, runtime, and tracing crate-family categories.
fn assert_runtime_categories(sandbox: &Sandbox, args: &[&str]) {
    // Check the remaining facades without collapsing their category names.
    assert_category_registered(
        sandbox,
        args,
        "test-case",
        "test_case_async_without_test_harness",
    );
    assert_category_registered(
        sandbox,
        args,
        "thiserror",
        "thiserror_named_field_positional_format",
    );
    assert_category_registered(sandbox, args, "tokio", "tokio_blocking_call_in_async");
    assert_category_registered(
        sandbox,
        args,
        "tracing",
        "tracing_async_block_in_sync_scope",
    );
}

/// A check that passed under one category reruns when another category is selected.
#[test]
fn switching_categories_reruns_cached_checks() {
    let sandbox = Sandbox::default();
    let repository = sandbox.path("switch");
    package_repository(
        &repository,
        "synthetic-category-switch",
        "",
        "src/lib.rs",
        CHANGED_RANGE_BASELINE,
    );
    let target = sandbox.path("target");
    let run = |category: &str| {
        sandbox.run(&[
            "--repo",
            text(&repository),
            "--fast",
            "--target-dir",
            text(&target),
            "--skip-clippy",
            "--dylint-category",
            category,
            "--heartbeat-seconds",
            "0",
        ])
    };

    // The first category passes and leaves fresh check artifacts behind.
    let clean = run("suspicious");
    assert_eq!(clean.code, Some(0), "{}", clean.text);

    // The second category must lint again instead of trusting those artifacts.
    let flagged = run("complexity");
    assert_eq!(flagged.code, Some(1), "{}", flagged.text);
    assert!(flagged.text.contains("collect_return"), "{}", flagged.text);
}

/// Strict Clippy rejects the fixture's `unwrap` call.
#[test]
fn strict_clippy_rejects_fixture() {
    // Run strict Clippy without the embedded private-lint phase.
    let sandbox = Sandbox::default();
    let target = sandbox.path("target");
    let output = sandbox.run(&[
        "--repo",
        text(&fixture("cargo-install-target")),
        "--no-workspace",
        "--fast",
        "--target-dir",
        text(&target),
        "--skip-dylint",
        "--heartbeat-seconds",
        "0",
    ]);

    assert_eq!(output.code, Some(1), "{}", output.text);
    assert!(
        output.text.contains("clippy::unwrap-used"),
        "{}",
        output.text
    );
}

/// Fix mode rewrites machine-applicable findings.
///
/// A second run no longer reports the rewritten findings.
#[test]
fn fix_mode_applies_machine_applicable_suggestions() {
    assert_fix_mode_applies_machine_applicable_suggestions();
}

/// Verify the rewritten source and the clean verification pass.
fn assert_fix_mode_applies_machine_applicable_suggestions() {
    // Copy the fixture into a writable repository before running fix mode.
    let sandbox = Sandbox::default();
    let working_copy = sandbox.path("input");
    let input = fixture("lint-fixer/input");
    write(
        &working_copy.join("Cargo.toml"),
        &read(&input.join("Cargo.toml")),
    );
    write(
        &working_copy.join("src/main.rs"),
        &read(&input.join("src/main.rs")),
    );
    let repository = text(&working_copy);
    let fix_args = ["--repo", repository, "--fast", "--skip-clippy", "--fix"];

    // Apply machine suggestions, then rerun the same selected phase.
    let fixed = sandbox.run(&fix_args);
    assert_eq!(fixed.code, Some(0), "{}", fixed.text);
    let expected = read(&fixture("lint-fixer/expected/src/main.rs"));
    let actual = read(&working_copy.join("src/main.rs"));
    assert_eq!(actual, expected);

    // The fixed source must no longer trigger either rewritten lint.
    let checked = sandbox.run(&fix_args[..4]);
    assert!(
        !checked
            .text
            .contains("internal import should start with `self::`"),
        "{}",
        checked.text
    );
    assert!(
        !checked
            .text
            .contains("outer `doc` attribute can be written as `///`"),
        "{}",
        checked.text
    );
}

/// A clean external repository passes without build output or Git changes inside it.
#[test]
fn clean_repository_passes_without_writing_into_it() {
    assert_clean_repository_passes_without_writing_into_it();
}

/// Verify a clean external repository stays free of runner artifacts and changes.
fn assert_clean_repository_passes_without_writing_into_it() {
    // Prepare a committed package that satisfies the pinned compiler version.
    let sandbox = Sandbox::default();
    let repository = sandbox.path("clean");
    package_repository(
        &repository,
        "synthetic-clean-boundary",
        "rust-version = \"1.98.1\"\n",
        "src/main.rs",
        "#![allow(missing_docs, reason = \"synthetic fixture has no public API\")]\n\nfn main() {}\n",
    );

    // Run both phases and inspect the repository after completion.
    let output = sandbox.run(&[
        "--repo",
        text(&repository),
        "--fast",
        "--heartbeat-seconds",
        "0",
    ]);

    assert_eq!(output.code, Some(0), "{}", output.text);
    assert!(
        output.text.contains("clippy: finished in"),
        "{}",
        output.text
    );
    assert!(
        output.text.contains("dylint: finished in"),
        "{}",
        output.text
    );
    assert!(!repository.join("target").exists());
    let status = Command::new("git")
        .arg("-C")
        .arg(&repository)
        .args(["status", "--short"])
        .output()
        .expect("git should start");
    assert!(
        status.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&status.stdout)
    );
}

/// Library source whose `collect` return triggers `collect_return`.
const CHANGED_RANGE_BASELINE: &str = "pub fn baseline(values: &[u64]) -> Vec<u64> {\n    values.iter().map(|value| value * 2).collect()\n}\n";

/// Run the complexity lints on a repository, optionally limited to the last commit.
fn run_complexity(sandbox: &Sandbox, repository: &Path, is_last_commit_only: bool) -> RunOutput {
    let mut args = vec![
        "--repo",
        text(repository),
        "--fast",
        "--skip-clippy",
        "--dylint-category",
        "complexity",
        "--heartbeat-seconds",
        "0",
    ];
    if is_last_commit_only {
        args.extend(["--changed-range", "HEAD^..HEAD"]);
    }
    sandbox.run(&args)
}

/// A changed range reports findings on changed lines only.
#[test]
fn changed_range_reports_only_changed_lines() {
    assert_changed_range_reports_only_changed_lines();
}

/// Verify that changed-range filtering ignores unrelated changes and selects new findings.
fn assert_changed_range_reports_only_changed_lines() {
    // Start from a committed baseline with one known complexity finding.
    let sandbox = Sandbox::default();
    let repository = sandbox.path("diff");
    let library = repository.join("src/lib.rs");
    let baseline = CHANGED_RANGE_BASELINE;
    package_repository(
        &repository,
        "synthetic-diff-boundary",
        "",
        "src/lib.rs",
        baseline,
    );

    // The full run reports the baseline finding.
    let full = run_complexity(&sandbox, &repository, false);
    assert_eq!(full.code, Some(1), "{}", full.text);
    assert!(full.text.contains("collect_return"), "{}", full.text);

    // An unrelated change leaves the baseline finding outside the range.
    let unrelated_source = format!("{baseline}\n// unrelated source change\n");
    write(&library, &unrelated_source);
    commit_all(&repository, "unrelated source change");
    let unrelated = run_complexity(&sandbox, &repository, true);
    assert_eq!(unrelated.code, Some(0), "{}", unrelated.text);
    assert!(
        unrelated.text.contains("no diagnostics on changed lines"),
        "{}",
        unrelated.text
    );
    assert!(
        !unrelated.text.contains("collect_return"),
        "{}",
        unrelated.text
    );

    // A new finding inside the range is reported exactly once.
    write(
        &library,
        &format!(
            "{unrelated_source}\npub fn changed(values: &[u64]) -> Vec<u64> {{\n    values.iter().map(|value| value + 1).collect()\n}}\n"
        ),
    );
    commit_all(&repository, "changed lint boundary");
    // The final range must contain only the newly introduced finding.
    let changed = run_complexity(&sandbox, &repository, true);
    assert_eq!(changed.code, Some(1), "{}", changed.text);
    assert_eq!(
        changed.text.matches("collect_return").count(),
        1,
        "{}",
        changed.text
    );
}

/// A package that requires a newer compiler fails with the required version.
#[test]
fn incompatible_rust_version_is_reported() {
    // Build a fixture whose declared version exceeds the embedded compiler.
    let sandbox = Sandbox::default();
    let repository = sandbox.path("toolchain");
    package_repository(
        &repository,
        "synthetic-toolchain-boundary",
        "rust-version = \"1.999.0\"\n",
        "src/main.rs",
        "fn main() {}\n",
    );

    // The runner must reject the package before starting lint phases.
    let output = sandbox.run(&[
        "--repo",
        text(&repository),
        "--fast",
        "--skip-dylint",
        "--heartbeat-seconds",
        "0",
    ]);

    assert_eq!(output.code, Some(1), "{}", output.text);
    assert!(
        output.text.contains("requires rustc 1.999.0"),
        "{}",
        output.text
    );
}

/// Source used by fake-Cargo fix tests; `foo` occupies bytes `0..3`.
const FIX_SOURCE: &str = "foo\n";

/// Run the fake Cargo phase in fix mode with extra arguments.
fn run_fix(sandbox: &Sandbox, body: &str, extra: &[&str]) -> (PathBuf, RunOutput) {
    // Use a fresh source fixture for each fake Cargo process.
    let repository = sandbox.repository("fix", &[("src/lib.rs", FIX_SOURCE)]);
    let cargo = sandbox.fake_cargo(body);
    // Append caller-selected mode flags after the stable fix arguments.
    let mut args = vec![
        "--repo",
        text(&repository),
        "--cargo-cmd",
        &cargo,
        "--skip-dylint",
        "--fix",
        "--heartbeat-seconds",
        "0",
    ];
    args.extend_from_slice(extra);
    let output = sandbox.run(&args);
    (repository, output)
}

/// Fix mode applies one suggestion and defers its overlapping rival.
///
/// The next pass verifies a clean source.
#[test]
fn fix_mode_converges_and_reports_only_the_final_pass() {
    assert_fix_mode_converges_and_reports_only_the_final_pass();
}

/// Verify overlapping suggestions, pass logs, reports, and final timing metadata.
fn assert_fix_mode_converges_and_reports_only_the_final_pass() {
    let sandbox = Sandbox::default();
    let logs = sandbox.path("logs");
    let report = sandbox.path("report.json");
    let timings = sandbox.path("timings.json");
    // Request all persisted artifacts so each output boundary is checked.
    let (repository, output) = run_converging_fix(&sandbox, &logs, &report, &timings);

    // Only the final verification pass may populate the report.
    assert_output(
        &output,
        0,
        &[
            "clippy: src/lib.rs:1: error [demo]: error finding",
            "clippy: applied 1 machine-applicable suggestions across 1 files.",
            "clippy: deferred 1 overlapping suggestions to the next pass.",
            "fix: applied 1 suggestions; starting verification pass 2.",
        ],
    );
    assert_eq!(read(&repository.join("src/lib.rs")), "bar\n");
    assert_fix_convergence_artifacts(&logs, &report, &timings);
}

/// Run the two-pass fixture that applies one fix and defers its rival.
fn run_converging_fix(
    sandbox: &Sandbox,
    logs: &Path,
    report: &Path,
    timings: &Path,
) -> (PathBuf, RunOutput) {
    // Feed one accepted suggestion and one overlapping rival to the fixer.
    let first = compiler_message(
        "error",
        Some("demo"),
        ("src/lib.rs", 1),
        Some((0, 3, "bar")),
    );
    let rival = compiler_message(
        "error",
        Some("demo"),
        ("src/lib.rs", 1),
        Some((0, 3, "baz")),
    );
    // Keep the shell body stable so the second pass sees the applied source edit.
    let finding = emit(&[first, rival]);
    let body = format!("if grep -q foo src/lib.rs; then\n{finding}\nexit 101\nfi");
    run_fix(
        sandbox,
        &body,
        &[
            "--log-dir",
            text(logs),
            "--gitlab-code-quality",
            text(report),
            "--timings-json",
            text(timings),
        ],
    )
}

/// Verify the final source, report, log files, and phase timing metadata.
fn assert_fix_convergence_artifacts(logs: &Path, report: &Path, timings: &Path) {
    // The report describes the verified final source, not the first pass.
    assert_eq!(read(report), "[]\n");
    assert!(logs.join("fix-pass-1/clippy.stdout").is_file());
    assert!(logs.join("fix-pass-2/clippy.command").is_file());
    // The retained phase list includes both the initial and verification passes.
    let timings: serde_json::Value =
        serde_json::from_str(&read(timings)).expect("timings should be JSON");
    assert_eq!(
        timings
            .get("phases")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(2)
    );
}

/// Fix mode stops after its pass limit when every pass produces another edit.
#[test]
fn fix_mode_stops_at_the_pass_limit() {
    // A repeated insertion exercises the runner's fixed pass bound.
    let sandbox = Sandbox::default();
    let insertion = compiler_message("error", Some("demo"), ("src/lib.rs", 1), Some((0, 0, "x")));

    // The final source records every bounded insertion before the failure.
    let (repository, output) = run_fix(&sandbox, &format!("{}\nexit 101", emit(&[insertion])), &[]);

    assert_eq!(output.code, Some(1), "{}", output.text);
    assert!(
        output
            .text
            .contains("fix: reached the maximum of 10 passes; inspect the remaining diagnostics."),
        "{}",
        output.text
    );
    assert_eq!(read(&repository.join("src/lib.rs")), "xxxxxxxxxxfoo\n");
}

/// A failing phase without fixes fails, and an invalid fix aborts with its path.
#[test]
fn fix_mode_reports_unfixable_failures() {
    assert_fix_mode_reports_unfixable_failures();
}

/// Verify both a failed phase without fixes and an invalid replacement range.
fn assert_fix_mode_reports_unfixable_failures() {
    // Preserve the command failure when no machine-applicable fix exists.
    let sandbox = Sandbox::default();
    let (_repository, unfixable) = run_fix(&sandbox, "echo broken >&2\nexit 101", &[]);
    assert_eq!(unfixable.code, Some(1), "{}", unfixable.text);
    assert!(
        unfixable
            .text
            .contains("clippy: command failed without machine-applicable fixes; inspect logs."),
        "{}",
        unfixable.text
    );

    // Use a separate run to verify invalid ranges leave source unchanged.
    let sandbox = Sandbox::default();
    let invalid = compiler_message("error", None, ("src/lib.rs", 1), Some((0, 99, "x")));
    let (repository, output) = run_fix(&sandbox, &emit(&[invalid]), &[]);
    assert_eq!(output.code, Some(1), "{}", output.text);
    assert!(
        output.text.contains(
            "sagan-lints: diagnostic processing failed: could not apply fixes to `src/lib.rs`"
        ),
        "{}",
        output.text
    );
    assert_eq!(read(&repository.join("src/lib.rs")), FIX_SOURCE);
}

/// Raw mode forwards child streams and fails only when a phase fails.
#[test]
fn raw_mode_forwards_output_and_exit_status() {
    assert_raw_mode_forwards_output_and_exit_status();
}

/// Verify raw stream forwarding and the phase exit-status contract.
fn assert_raw_mode_forwards_output_and_exit_status() {
    // Run one successful phase and inspect both child streams.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("raw", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo(
        "echo out-marker\necho err-marker >&2\nexit \"$(printenv FAKE_STATUS || echo 0)\"",
    );
    let args = [
        "--repo",
        text(&repository),
        "--cargo-cmd",
        &cargo,
        "--skip-dylint",
    ];

    let passed = sandbox.run(&args);
    assert_eq!(passed.code, Some(0), "{}", passed.text);
    assert_forwarded_fragments(&passed);
    // A nonzero child status must remain visible in the runner summary.
    let failed = output(sandbox.command(&args).env("FAKE_STATUS", "3"));
    assert_eq!(failed.code, Some(1), "{}", failed.text);
    assert!(failed.text.contains("(exit 3)"), "{}", failed.text);
}

/// Check every stable marker emitted by a successful raw phase.
fn assert_forwarded_fragments(passed: &RunOutput) {
    // Keep stream markers and runner timing output in one ordered contract.
    for expected in [
        "out-marker",
        "err-marker",
        "clippy: finished in",
        "(exit 0)",
        "Total: ",
    ] {
        assert!(
            passed.text.contains(expected),
            "{expected}\n{}",
            passed.text
        );
    }
}

/// Log and timing files record each phase's command, streams, and status.
#[test]
fn log_dir_and_timings_record_each_phase() {
    assert_log_dir_and_timings_record_each_phase();
}

/// Verify captured streams, command logs, and timing metadata for one phase.
fn assert_log_dir_and_timings_record_each_phase() {
    // Request logs and timings together so both persisted output contracts run.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("logs", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo("echo out-marker\necho err-marker >&2");
    let logs = sandbox.path("logs");
    let timings = sandbox.path("reports/timings.json");

    // Run one phase before checking its forwarded output and persisted artifacts.
    let output = sandbox.run(&[
        "--repo",
        text(&repository),
        "--cargo-cmd",
        &cargo,
        "--skip-dylint",
        "--log-dir",
        text(&logs),
        "--timings-json",
        text(&timings),
    ]);

    assert_log_output(&output, &logs);
    assert_log_files(&logs, &timings);
}

/// Verify the process status and streamed output before reading persisted files.
fn assert_log_output(output: &RunOutput, logs: &Path) {
    assert_eq!(output.code, Some(0), "{}", output.text);
    // Captured streams are still forwarded after the phase finishes.
    assert!(output.text.contains("out-marker"), "{}", output.text);
    let log_path = text(logs);
    assert!(
        output.text.contains(&format!("Full logs: {log_path}\n")),
        "{}",
        output.text
    );
}

/// Verify phase stream files and timing metadata for the completed process.
fn assert_log_files(logs: &Path, timings: &Path) {
    // Each stream and command file records the same phase that produced the timing entry.
    assert_eq!(read(&logs.join("clippy.stdout")), "out-marker\n");
    assert_eq!(read(&logs.join("clippy.stderr")), "err-marker\n");
    assert!(read(&logs.join("clippy.command")).starts_with("sh "));
    // Parse the timing document only after all phase files have been checked.
    let timings: serde_json::Value =
        serde_json::from_str(&read(timings)).expect("timings should be JSON");
    let phase = timings
        .get("phases")
        .and_then(serde_json::Value::as_array)
        .and_then(|phases| phases.first())
        .expect("timings should contain the clippy phase");
    assert_eq!(
        phase.get("name"),
        Some(&serde_json::Value::String("clippy".to_owned()))
    );
    assert_eq!(phase.get("return_code"), Some(&serde_json::Value::from(0)));
}

/// Report and log paths that cannot be written fail with the affected path.
#[test]
fn unwritable_report_paths_are_reported() {
    assert_unwritable_report_paths_are_reported();
}

/// Verify every report destination reports its affected path when blocked.
fn assert_unwritable_report_paths_are_reported() {
    // Build one file blocker and one directory blocker for the three outputs.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("unwritable", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo("true");
    let blocker = sandbox.path("blocker");
    write(&blocker, "a file where a directory is expected");
    let directory = sandbox.path("existing-directory");
    fs::create_dir_all(&directory).expect("directory should be creatable");
    let base = [
        "--repo",
        text(&repository),
        "--cargo-cmd",
        &cargo,
        "--skip-dylint",
    ];

    // Each report option must report its own failed destination.
    assert_unwritable_paths(&sandbox, &base, &blocker, &directory);
}

/// Run the three output-path failure cases without hiding their individual paths.
fn assert_unwritable_paths(sandbox: &Sandbox, base: &[&str], blocker: &Path, directory: &Path) {
    // Each option exercises a different filesystem failure boundary.
    // Preserve the option name in each assertion so failures identify the boundary.
    for (flag, path) in [
        ("--log-dir", blocker.join("logs")),
        ("--timings-json", directory.to_path_buf()),
        ("--gitlab-code-quality", blocker.join("report.json")),
    ] {
        let mut args = base.to_vec();
        args.extend([flag, text(&path)]);
        let output = sandbox.run(&args);
        assert_eq!(output.code, Some(1), "{flag}: {}", output.text);
        assert!(
            output
                .text
                .contains(&format!("filesystem operation failed for {}", text(&path))),
            "{flag}: {}",
            output.text
        );
    }
}

/// GitLab reports contain findings and fail the run.
///
/// Failures without JSON keep raw output.
#[test]
fn gitlab_code_quality_reports_findings() {
    assert_gitlab_code_quality_reports_findings();
}

/// Verify finding, raw-failure, and clean report outcomes.
fn assert_gitlab_code_quality_reports_findings() {
    // Reuse one fixture while selecting each fake Cargo outcome by environment.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("quality", &[("src/lib.rs", "")]);
    let finding = compiler_message("warning", Some("demo"), ("src/lib.rs", 1), None);
    let finding_output = emit(&[finding]);
    let body = format!(
        "case \"$(printenv FAKE_CASE || echo clean)\" in\nfinding)\n{finding_output}\n;;\nbroken)\necho raw-failure >&2\nexit 101\n;;\nesac"
    );
    let cargo = sandbox.fake_cargo(&body);
    let report = sandbox.path("report.json");
    let args = [
        "--repo",
        text(&repository),
        "--cargo-cmd",
        &cargo,
        "--skip-dylint",
        "--gitlab-code-quality",
        text(&report),
    ];

    let found = output(sandbox.command(&args).env("FAKE_CASE", "finding"));
    assert_gitlab_findings(&found, &report);

    // A broken phase keeps raw output and replaces the report with no findings.
    let broken = output(sandbox.command(&args).env("FAKE_CASE", "broken"));
    assert_gitlab_broken(&broken, &report);

    // A clean phase succeeds and leaves an empty report.
    let clean = sandbox.run(&args);
    assert_gitlab_clean(&clean, &report);
}

/// Verify a finding fails the run and becomes a GitLab code-quality entry.
fn assert_gitlab_findings(found: &RunOutput, report: &Path) {
    assert_eq!(found.code, Some(1), "{}", found.text);
    assert!(
        found
            .text
            .contains("clippy: src/lib.rs:1: warning [demo]: warning finding"),
        "{}",
        found.text
    );
    let entries: serde_json::Value =
        serde_json::from_str(&read(report)).expect("report should be JSON");
    assert_eq!(
        entries
            .get(0)
            .and_then(serde_json::Value::as_object)
            .and_then(|entry| entry.get("check_name")),
        Some(&serde_json::Value::String("demo".to_owned()))
    );
}

/// Verify a failed phase preserves raw output and clears the report.
fn assert_gitlab_broken(broken: &RunOutput, report: &Path) {
    assert_eq!(broken.code, Some(1), "{}", broken.text);
    assert!(broken.text.contains("raw-failure"), "{}", broken.text);
    assert_eq!(read(report), "[]\n");
}

/// Verify a clean phase succeeds and leaves an empty report.
fn assert_gitlab_clean(clean: &RunOutput, report: &Path) {
    assert_eq!(clean.code, Some(0), "{}", clean.text);
    assert_eq!(read(report), "[]\n");
}

/// Skipping every phase succeeds and still writes a valid empty report.
#[test]
fn empty_selection_writes_an_empty_report() {
    // An empty phase selection still owns the requested report path.
    let sandbox = Sandbox::default();
    let report = sandbox.path("report.json");

    // The runner should report the empty selection without preparing Cargo.
    let output = sandbox.run(&[
        "--repo",
        text(&fixture("cargo-install-target")),
        "--skip-clippy",
        "--skip-dylint",
        "--gitlab-code-quality",
        text(&report),
    ]);

    assert_eq!(output.code, Some(0), "{}", output.text);
    assert!(
        output.text.contains("No commands selected."),
        "{}",
        output.text
    );
    assert_eq!(read(&report), "[]\n");
}

/// Commit a Rust and a nested-manifest change and return the repository and fake Cargo.
///
/// The fake Cargo replays diagnostics selected by `FAKE_CASE`: `findings` reports
/// lines inside and outside the range, `blocking` reports a compiler error outside
/// it, `broken` fails without JSON, and any other value succeeds silently.
fn changed_range_repository(sandbox: &Sandbox) -> (PathBuf, String) {
    // Commit the baseline before adding the changed Rust and manifest lines.
    let repository = sandbox.repository(
        "changed",
        &[
            ("src/lib.rs", "one\ntwo\nthree\n"),
            ("crates/a/Cargo.toml", "[package]\nname = \"a\"\n"),
        ],
    );
    // Change both a Rust line and a nested manifest line for filtering coverage.
    write(&repository.join("src/lib.rs"), "one\ntwo\nTHREE\n");
    write(
        &repository.join("crates/a/Cargo.toml"),
        "[package]\nname = \"b\"\n",
    );
    commit_all(&repository, "change");
    // Build inside, outside, blocking, and malformed phase outcomes.
    let unchanged = compiler_message("error", Some("demo"), ("src/lib.rs", 1), None);
    let changed = compiler_message("error", Some("demo"), ("src/lib.rs", 3), None);
    let manifest = compiler_message("warning", Some("demo"), ("crates/a/Cargo.toml", 2), None);
    let blocking = compiler_message("error", Some("E0425"), ("src/lib.rs", 1), None);
    let findings_output = emit(&[unchanged, changed, manifest]);
    let blocking_output = emit(&[blocking]);
    let body = format!(
        "case \"$(printenv FAKE_CASE || echo clean)\" in\nfindings)\n{findings_output}\nexit 101\n;;\nblocking)\n{blocking_output}\nexit 101\n;;\nbroken)\nexit 101\n;;\nesac"
    );
    let cargo = sandbox.fake_cargo(&body);
    (repository, cargo)
}

/// Run the changed-range fake Cargo scenario named by `case`.
fn run_changed_range(sandbox: &Sandbox, repository: &Path, cargo: &str, case: &str) -> RunOutput {
    output(
        sandbox
            .command(&[
                "--repo",
                text(repository),
                "--cargo-cmd",
                cargo,
                "--skip-dylint",
                "--changed-range",
                "HEAD^..HEAD",
            ])
            .env("FAKE_CASE", case),
    )
}

/// Changed-range mode selects diagnostics on changed Rust and nested manifest lines.
#[test]
fn changed_range_selects_changed_rust_and_manifest_lines() {
    // The changed-range report must include both selected source categories.
    let sandbox = Sandbox::default();
    let (repository, cargo) = changed_range_repository(&sandbox);

    // Run the fixture after the repository contains both changed hunks.
    let findings = run_changed_range(&sandbox, &repository, &cargo, "findings");

    assert_output(
        &findings,
        1,
        &[
            "Filtering diagnostics to 2 changed hunks from HEAD^..HEAD.",
            "clippy: src/lib.rs:3: error [demo]",
            "clippy: crates/a/Cargo.toml:2: warning [demo]",
        ],
    );
    assert!(
        !findings.text.contains("src/lib.rs:1:"),
        "{}",
        findings.text
    );
    assert!(
        !findings.text.contains("command failed"),
        "{}",
        findings.text
    );
}

/// Changed-range mode passes a clean run and fails an incomplete one.
#[test]
fn changed_range_fails_incomplete_runs() {
    // A clean run succeeds before incomplete compiler outcomes are checked.
    let sandbox = Sandbox::default();
    let (repository, cargo) = changed_range_repository(&sandbox);

    let clean = run_changed_range(&sandbox, &repository, &cargo, "clean");
    assert_output(&clean, 0, &["clippy: no diagnostics on changed lines."]);

    // A compiler error outside the range or a failure without JSON means the run is incomplete.
    assert_incomplete_changed_range_cases(&sandbox, &repository, &cargo);
}

/// Verify both incomplete changed-range outcomes use the same failure diagnostic.
fn assert_incomplete_changed_range_cases(sandbox: &Sandbox, repository: &Path, cargo: &str) {
    // Keep compiler errors outside the range and malformed output as separate cases.
    for case in ["blocking", "broken"] {
        assert_output(
            &run_changed_range(sandbox, repository, cargo, case),
            1,
            &["clippy: command failed before changed-range linting completed; inspect logs."],
        );
    }
}

/// An unknown revision range or a missing Git executable fails before any phase runs.
#[test]
fn changed_range_reports_git_failures() {
    // Exercise revision resolution and the missing Git executable separately.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("git", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo("true");
    // Build one command for each Git failure boundary while preserving the repository path.
    let args = |range| {
        vec![
            "--repo".to_owned(),
            text(&repository).to_owned(),
            "--cargo-cmd".to_owned(),
            cargo.clone(),
            "--changed-range".to_owned(),
            range,
        ]
    };
    let run = |range: &str| {
        let args = args(range.to_owned());
        sandbox.command(&args.iter().map(String::as_str).collect::<Vec<_>>())
    };

    let unknown = output(&mut run("missing-revision..HEAD"));
    let unknown_is_rejected = unknown.code == Some(1)
        && unknown
            .text
            .contains("Git could not resolve changed range `missing-revision..HEAD`");

    // Removing Git from PATH must fail before Cargo starts.
    let no_git = output(run("HEAD").env("PATH", sandbox.path("empty-path")));
    let no_git_is_rejected = no_git.code == Some(1)
        && no_git
            .text
            .contains("could not run Git for changed-range filtering");
    // Both failures report their own boundary and never reach the phase runner.
    assert_eq!(
        (unknown_is_rejected, no_git_is_rejected),
        (true, true),
        "unknown:\n{}\nno Git:\n{}",
        unknown.text,
        no_git.text
    );
}

/// A long phase prints heartbeats at the requested interval.
#[test]
fn heartbeat_reports_long_running_phases() {
    // Use a delayed fake phase so the one-second heartbeat becomes observable.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("heartbeat", &[("src/lib.rs", "")]);
    // Keep the child alive long enough for one heartbeat to be emitted.
    let cargo = sandbox.fake_cargo("sleep 1.5");

    let output = sandbox.run(&[
        "--repo",
        text(&repository),
        "--cargo-cmd",
        &cargo,
        "--skip-dylint",
        "--heartbeat-seconds",
        "1",
    ]);

    assert_eq!(output.code, Some(0), "{}", output.text);
    assert!(
        output.text.contains("clippy: still running (1s elapsed)"),
        "{}",
        output.text
    );
}

/// Unusable Cargo commands fail with the phase or option that rejected them.
#[test]
fn unusable_cargo_commands_are_reported() {
    // Check both an unstartable executable and an empty wrapper command.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("cargo-command", &[("src/lib.rs", "")]);
    let missing = sandbox.path("missing-cargo");
    let repo = text(&repository);

    let unstartable = sandbox.run(&[
        "--repo",
        repo,
        "--cargo-cmd",
        text(&missing),
        "--skip-dylint",
    ]);
    assert_eq!(unstartable.code, Some(1), "{}", unstartable.text);
    assert!(
        unstartable.text.contains("could not run phase `clippy`"),
        "{}",
        unstartable.text
    );

    // Both phase builders reject a wrapper without an executable.
    assert_empty_cargo_wrappers_rejected(&sandbox, repo);
}

/// Verify both phase-selection branches reject a wrapper without an executable.
fn assert_empty_cargo_wrappers_rejected(sandbox: &Sandbox, repo: &str) {
    // Keep the Clippy and private-lint skip flags as independent boundaries.
    for skipped in ["--skip-clippy", "--skip-dylint"] {
        let empty = sandbox.run(&["--repo", repo, "--cargo-cmd", " ", skipped]);
        assert_eq!(empty.code, Some(1), "{}", empty.text);
        assert!(
            empty
                .text
                .contains("sagan-lints: --cargo-cmd must contain an executable"),
            "{}",
            empty.text
        );
    }
}

/// Missing and non-directory repository paths fail before any cache is created.
#[test]
fn invalid_repository_paths_are_rejected() {
    // Check missing and non-directory paths before any cache directory exists.
    let sandbox = Sandbox::default();
    let file = sandbox.path("file");
    write(&file, "not a repository");

    // A missing path fails during canonicalization.
    let missing = sandbox.run(&["--repo", text(&sandbox.path("missing"))]);
    let missing_is_rejected = missing.code == Some(1) && missing.text.contains("could not resolve");

    // A regular file resolves but fails the directory boundary.
    let not_directory = sandbox.run(&["--repo", text(&file)]);
    let file_is_rejected =
        not_directory.code == Some(1) && not_directory.text.contains("not a directory:");
    let cache_is_untouched = !sandbox.path("cache").exists();
    assert_eq!(
        (missing_is_rejected, file_is_rejected, cache_is_untouched),
        (true, true, true),
        "missing:\n{}\nfile:\n{}",
        missing.text,
        not_directory.text
    );
}

/// Cache candidates inside the repository, relative, or unwritable are all skipped.
#[test]
fn unsafe_cache_candidates_are_rejected() {
    // Combine repository, relative, symlink, and HOME-file candidates.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("cache-safety", &[("src/lib.rs", "")]);
    let link = sandbox.path("link-into-repository");
    symlink(&repository, &link).expect("symlink should be creatable");
    let home = sandbox.path("home-file");
    write(&home, "HOME is a file");

    let output = output(
        sandbox
            .command(&["--repo", text(&repository), "--dry-run"])
            .env("SAGAN_LINTS_CACHE_DIR", repository.join("cache"))
            .env("RUST_PERSONAL_LINTS_CACHE_DIR", "relative-cache")
            .env("XDG_CACHE_HOME", &link)
            .env("HOME", &home),
    );

    // Rejection must happen before cache creation through any candidate.
    let cache_is_rejected = output.code == Some(1)
        && output
            .text
            .contains("no writable Sagan-lints cache directory");
    let has_relative_cache_diagnostic = output.text.contains("relative-cache");
    // Rejection happens before anything is created through the symbolic link.
    let linked_cache_is_untouched = !repository.join("sagan-lints").exists();
    let repository_cache_is_untouched = !repository.join("cache").exists();
    assert_eq!(
        (
            cache_is_rejected,
            has_relative_cache_diagnostic,
            linked_cache_is_untouched,
            repository_cache_is_untouched,
        ),
        (true, true, true, true),
        "{}",
        output.text
    );
}

/// The target-cache limit must be an unsigned decimal byte count.
#[test]
fn invalid_target_cache_limits_are_rejected() {
    // Check signed and non-UTF-8 values against the same parser boundary.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("limit", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo("true");
    let args = [
        "--repo",
        text(&repository),
        "--cargo-cmd",
        &cargo,
        "--skip-dylint",
    ];

    assert_invalid_target_cache_limits(&sandbox, &args);
}

/// Verify each malformed target-cache limit produces the same diagnostic.
fn assert_invalid_target_cache_limits(sandbox: &Sandbox, args: &[&str]) {
    // Keep signed and invalid-UTF-8 inputs as separate parser cases.
    for value in [OsStr::new("-1"), OsStr::from_bytes(b"\xff")] {
        let output = output(
            sandbox
                .command(args)
                .env("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES", value),
        );
        assert_eq!(output.code, Some(1), "{}", output.text);
        assert!(
            output
                .text
                .contains("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES must be an unsigned byte count"),
            "{}",
            output.text
        );
    }
}

/// A missing compiler sysroot or tool fails with the missing path.
#[test]
fn missing_compiler_runtime_is_reported() {
    // Check a missing sysroot and a missing Cargo asset independently.
    let sandbox = Sandbox::default();
    let repository = fixture("cargo-install-target");
    let empty_sysroot = sandbox.path("sysroot");
    fs::create_dir_all(&empty_sysroot).expect("sysroot should be creatable");

    assert_missing_compiler_runtime(&sandbox, &repository, &empty_sysroot);
}

/// Verify missing runtime directories report the exact packaged asset path.
fn assert_missing_compiler_runtime(sandbox: &Sandbox, repository: &Path, empty_sysroot: &Path) {
    // Preserve the distinction between a missing sysroot and a missing tool.
    for (sysroot, missing) in [
        (
            sandbox.path("missing-sysroot"),
            sandbox.path("missing-sysroot"),
        ),
        (empty_sysroot.to_path_buf(), empty_sysroot.join("bin/cargo")),
    ] {
        let output = output(
            sandbox
                .command(&["--repo", text(repository), "--dry-run"])
                .env("SAGAN_LINTS_SYSROOT", &sysroot),
        );
        assert_eq!(output.code, Some(1), "{}", output.text);
        assert!(
            output.text.contains(&format!(
                "packaged asset does not exist: {}",
                text(&missing)
            )),
            "{}",
            output.text
        );
    }
}

/// Fake Cargo body that writes one artifact into the phase's target directory.
const ARTIFACT_CARGO: &str = "mkdir -p \"$CARGO_TARGET_DIR/build\"\nprintf artifact > \"$CARGO_TARGET_DIR/build/artifact\"\nchmod \"$(printenv FAKE_MODE || echo 755)\" \"$CARGO_TARGET_DIR/build\"\nexit \"$(printenv FAKE_STATUS || echo 0)\"";

/// Prepare arguments for a run that builds into the runner-managed target directory.
fn managed_args<'a>(repository: &'a Path, cargo: &'a str) -> [&'a str; 5] {
    [
        "--repo",
        text(repository),
        "--cargo-cmd",
        cargo,
        "--skip-dylint",
    ]
}

/// The managed target directory is pruned only when it exceeds the configured limit.
#[test]
fn managed_target_cache_is_pruned_above_its_limit() {
    // Run unlimited and zero-sized limits before the pruning boundary.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("managed", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo(ARTIFACT_CARGO);
    let args = managed_args(&repository, &cargo);
    let run = |limit: Option<&str>| {
        let mut command = sandbox.command(&args);
        if let Some(limit) = limit {
            let _command = command.env("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES", limit);
        }
        output(&mut command)
    };

    // The default and disabled limits retain a small build cache.
    assert_retained_managed_cache_limits(&sandbox, &run);

    // The positive limit removes the oversized target after the phase.
    let target = sandbox.managed_target();
    let pruned = run(Some("1"));
    assert_eq!(pruned.code, Some(0), "{}", pruned.text);
    assert!(
        pruned.text.contains(
            "target-cache: removed the managed target directory after it reached 8 bytes"
        ),
        "{}",
        pruned.text
    );
    assert!(!target.exists());
}

/// Verify limits that retain the managed target directory.
fn assert_retained_managed_cache_limits(
    sandbox: &Sandbox,
    run: &impl Fn(Option<&str>) -> RunOutput,
) {
    // Disabled limits retain the artifact for reuse.
    for limit in [None, Some("0"), Some("000")] {
        let kept = run(limit);
        assert_eq!(kept.code, Some(0), "{}", kept.text);
        assert!(sandbox.managed_target().join("build/artifact").is_file());
    }
}

/// An early failure still prunes an oversized managed target directory.
#[test]
fn managed_target_cache_is_pruned_after_early_failure() {
    // Build once, then trigger an early changed-range failure during cleanup.
    let sandbox = Sandbox::default();
    let repository = sandbox.repository("early-failure", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo(ARTIFACT_CARGO);
    let args = managed_args(&repository, &cargo);
    // The successful build creates the managed target before the failing run.
    let built = output(
        sandbox
            .command(&args)
            .env("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES", "0"),
    );
    assert_eq!(built.code, Some(0), "{}", built.text);
    let target = sandbox.managed_target();

    let mut failing = args.to_vec();
    failing.extend(["--changed-range", "missing-revision..HEAD"]);
    // Drop cleanup must prune the target even though planning fails early.
    let failed = output(
        sandbox
            .command(&failing)
            .env("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES", "1"),
    );

    assert_eq!(failed.code, Some(1), "{}", failed.text);
    assert!(!target.exists());
}

/// Measurement and removal failures name the managed target directory.
#[test]
fn managed_target_cache_failures_are_reported() {
    // Measure and removal failures use distinct fake Cargo permission modes.
    assert_managed_target_cache_failures();
}

/// Verify managed target measurement and removal errors retain their context.
fn assert_managed_target_cache_failures() {
    // Restore permissions after each mode so temporary directories remain removable.
    for (mode, message) in [
        ("000", "could not measure managed target directory"),
        ("555", "could not prune managed target directory"),
    ] {
        let sandbox = Sandbox::default();
        let repository = sandbox.repository("prune-failure", &[("src/lib.rs", "")]);
        let cargo = sandbox.fake_cargo(ARTIFACT_CARGO);
        let failed = output(
            sandbox
                .command(&managed_args(&repository, &cargo))
                .env("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES", "1")
                .env("FAKE_MODE", mode),
        );
        // Restore access so the sandbox can be removed.
        let build = sandbox.managed_target().join("build");
        fs::set_permissions(&build, fs::Permissions::from_mode(0o755))
            .expect("permissions should be restorable");

        assert_eq!(failed.code, Some(1), "{mode}: {}", failed.text);
        assert!(failed.text.contains(message), "{mode}: {}", failed.text);
    }
}

/// A closed standard output fails the run with one diagnostic instead of a panic.
#[test]
fn closed_stdout_is_reported() {
    // Close the runner's output pipe before its first progress write.
    let sandbox = Sandbox::default();
    let mut child = sandbox
        .command(&[
            "--repo",
            text(&fixture("cargo-install-target")),
            "--skip-clippy",
            "--skip-dylint",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the runner should start");
    // Close the read end before the runner writes its first line.
    drop(child.stdout.take());

    let output = child.wait_with_output().expect("the runner should finish");

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("sagan-lints: could not write runner output"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Run the executable in its internal compiler-driver role.
fn driver(sandbox: &Sandbox, args: &[&str]) -> Command {
    let mut command = sandbox.command(args);
    let _command = command
        .env("SAGAN_LINTS_DRIVER", "1")
        .env_remove("SAGAN_LINTS_DRIVER_CATEGORIES")
        .env_remove("SAGAN_LINTS_DRIVER_RUSTFLAGS")
        .env_remove("SAGAN_LINTS_DRIVER_LIST")
        .env_remove("SAGAN_LINTS_DRIVER_NO_DEPS")
        .env_remove("CARGO_PRIMARY_PACKAGE");
    command
}

/// The compiler driver rejects invalid process state before starting rustc.
#[test]
fn compiler_driver_rejects_invalid_process_state() {
    // Exercise category, runtime, and missing-input failures before rustc starts.
    let sandbox = Sandbox::default();

    let category = output(driver(&sandbox, &[]).env("SAGAN_LINTS_DRIVER_CATEGORIES", "bogus"));
    let category_is_rejected = category.code == Some(1)
        && category
            .text
            .contains("sagan-lints compiler driver: unknown embedded lint category `bogus`");

    let sysroot = output(driver(&sandbox, &[]).env("SAGAN_LINTS_SYSROOT", sandbox.path("missing")));
    let sysroot_is_rejected = sysroot.code == Some(1)
        && sysroot
            .text
            .contains("sagan-lints compiler driver: packaged asset does not exist");

    // Without arguments, rustc itself reports the missing input.
    let empty = output(&mut driver(&sandbox, &[]));
    let empty_is_rejected = empty.code == Some(1);
    assert_eq!(
        (category_is_rejected, sysroot_is_rejected, empty_is_rejected),
        (true, true, true)
    );
}

/// Every crate-specific category registers its bundled lint facade with the driver.
#[test]
fn compiler_driver_registers_every_crate_family_category() {
    // Prepare one metadata-only source for every independent category invocation.
    let sandbox = Sandbox::default();
    let source = sandbox.path("category_registration.rs");
    write(&source, "//! Registration fixture.\n");
    let output_path = sandbox.path("category_registration.rmeta");
    let args = [
        "rustc",
        "--crate-name",
        "category_registration",
        "--crate-type=lib",
        "--edition=2024",
        "--emit=metadata",
        "-o",
        text(&output_path),
        text(&source),
    ];

    // Exercise all thirteen explicit crate-family categories.
    assert_transport_categories(&sandbox, &args);
    assert_data_categories(&sandbox, &args);
    assert_runtime_categories(&sandbox, &args);
}

/// In no-deps mode, only Cargo's primary packages receive the selected private lints.
#[test]
fn compiler_driver_lints_only_primary_packages_in_no_deps_mode() {
    // Compile the same source as a dependency and a primary package.
    let sandbox = Sandbox::default();
    let source = sandbox.path("lib.rs");
    // The fixture triggers a selected private lint only in the primary branch.
    write(
        &source,
        "//! Fixture.\n\n/// Parse.\npub fn parse() -> Result<u8, String> {\n    Ok(1)\n}\n",
    );
    let output_path = sandbox.path("lib.rmeta");
    let args = [
        "rustc",
        "--crate-type=lib",
        "--edition=2024",
        "--emit=metadata",
        "-o",
        text(&output_path),
        text(&source),
    ];
    let run = |is_primary: bool| {
        let mut command = driver(&sandbox, &args);
        let _command = command
            .env("SAGAN_LINTS_DRIVER_CATEGORIES", "suspicious")
            .env("SAGAN_LINTS_DRIVER_RUSTFLAGS", "-D\u{1f}warnings")
            .env("SAGAN_LINTS_DRIVER_NO_DEPS", "1");
        if is_primary {
            let _command = command.env("CARGO_PRIMARY_PACKAGE", "1");
        }
        output(&mut command)
    };

    // Dependencies skip private lints in no-deps mode.
    let dependency = run(false);
    assert_eq!(dependency.code, Some(0), "{}", dependency.text);

    // Primary packages retain the selected private lint diagnostics.
    let primary = run(true);
    assert_eq!(primary.code, Some(1), "{}", primary.text);
    assert!(
        primary.text.contains("string_error_result"),
        "{}",
        primary.text
    );
}
