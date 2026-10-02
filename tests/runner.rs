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
    /// Create an empty sandbox.
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("temporary directory should be available"),
        }
    }

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
        let repository = self.path(name);
        for (relative, contents) in files {
            write(&repository.join(relative), contents);
        }
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

/// Write a single-package Cargo repository pinned to the runner's toolchain and commit it.
fn package_repository(
    repository: &Path,
    name: &str,
    manifest_extra: &str,
    source_path: &str,
    source: &str,
) {
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
    let sandbox = Sandbox::new();
    let output = list_private_lints(&sandbox);

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
    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
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

/// `--no-deps` still runs the embedded private lints on the selected package.
#[test]
fn no_deps_runs_private_lints() {
    let sandbox = Sandbox::new();
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

/// A check that passed under one category reruns when another category is selected.
#[test]
fn switching_categories_reruns_cached_checks() {
    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
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

/// Fix mode rewrites machine-applicable findings and a second run no longer reports them.
#[test]
fn fix_mode_applies_machine_applicable_suggestions() {
    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
    let repository = sandbox.path("clean");
    package_repository(
        &repository,
        "synthetic-clean-boundary",
        "rust-version = \"1.98.1\"\n",
        "src/main.rs",
        "#![allow(missing_docs, reason = \"synthetic fixture has no public API\")]\n\nfn main() {}\n",
    );

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
fn run_complexity(sandbox: &Sandbox, repository: &Path, last_commit_only: bool) -> RunOutput {
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
    if last_commit_only {
        args.extend(["--changed-range", "HEAD^..HEAD"]);
    }
    sandbox.run(&args)
}

/// A changed range reports findings on changed lines only.
#[test]
fn changed_range_reports_only_changed_lines() {
    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
    let repository = sandbox.path("toolchain");
    package_repository(
        &repository,
        "synthetic-toolchain-boundary",
        "rust-version = \"1.999.0\"\n",
        "src/main.rs",
        "fn main() {}\n",
    );

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
    let repository = sandbox.repository("fix", &[("src/lib.rs", FIX_SOURCE)]);
    let cargo = sandbox.fake_cargo(body);
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

/// Fix mode applies one suggestion, defers its overlapping rival, and verifies a clean pass.
#[test]
fn fix_mode_converges_and_reports_only_the_final_pass() {
    let sandbox = Sandbox::new();
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
    let body = format!(
        "if grep -q foo src/lib.rs; then\n{}\nexit 101\nfi",
        emit(&[first, rival])
    );
    let logs = sandbox.path("logs");
    let report = sandbox.path("report.json");
    let timings = sandbox.path("timings.json");

    let (repository, output) = run_fix(
        &sandbox,
        &body,
        &[
            "--log-dir",
            text(&logs),
            "--gitlab-code-quality",
            text(&report),
            "--timings-json",
            text(&timings),
        ],
    );

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
    // The report describes the verified final source, not the first pass.
    assert_eq!(read(&report), "[]\n");
    assert!(logs.join("fix-pass-1/clippy.stdout").is_file());
    assert!(logs.join("fix-pass-2/clippy.command").is_file());
    let timings: serde_json::Value =
        serde_json::from_str(&read(&timings)).expect("timings should be JSON");
    assert_eq!(timings["phases"].as_array().map(Vec::len), Some(2));
}

/// Fix mode stops after its pass limit when every pass produces another edit.
#[test]
fn fix_mode_stops_at_the_pass_limit() {
    let sandbox = Sandbox::new();
    let insertion = compiler_message("error", Some("demo"), ("src/lib.rs", 1), Some((0, 0, "x")));

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
    let sandbox = Sandbox::new();
    let (_repository, unfixable) = run_fix(&sandbox, "echo broken >&2\nexit 101", &[]);
    assert_eq!(unfixable.code, Some(1), "{}", unfixable.text);
    assert!(
        unfixable
            .text
            .contains("clippy: command failed without machine-applicable fixes; inspect logs."),
        "{}",
        unfixable.text
    );

    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
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

    let failed = output(sandbox.command(&args).env("FAKE_STATUS", "3"));
    assert_eq!(failed.code, Some(1), "{}", failed.text);
    assert!(failed.text.contains("(exit 3)"), "{}", failed.text);
}

/// Log and timing files record each phase's command, streams, and status.
#[test]
fn log_dir_and_timings_record_each_phase() {
    let sandbox = Sandbox::new();
    let repository = sandbox.repository("logs", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo("echo out-marker\necho err-marker >&2");
    let logs = sandbox.path("logs");
    let timings = sandbox.path("reports/timings.json");

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

    assert_eq!(output.code, Some(0), "{}", output.text);
    // Captured streams are still forwarded after the phase finishes.
    assert!(output.text.contains("out-marker"), "{}", output.text);
    assert!(
        output
            .text
            .contains(&format!("Full logs: {}\n", text(&logs))),
        "{}",
        output.text
    );
    assert_eq!(read(&logs.join("clippy.stdout")), "out-marker\n");
    assert_eq!(read(&logs.join("clippy.stderr")), "err-marker\n");
    assert!(read(&logs.join("clippy.command")).starts_with("sh "));
    let timings: serde_json::Value =
        serde_json::from_str(&read(&timings)).expect("timings should be JSON");
    assert_eq!(timings["phases"][0]["name"], "clippy");
    assert_eq!(timings["phases"][0]["return_code"], 0);
}

/// Report and log paths that cannot be written fail with the affected path.
#[test]
fn unwritable_report_paths_are_reported() {
    let sandbox = Sandbox::new();
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

    for (flag, path) in [
        ("--log-dir", blocker.join("logs")),
        ("--timings-json", directory),
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

/// GitLab reports contain findings and fail the run; failures without JSON keep raw output.
#[test]
fn gitlab_code_quality_reports_findings() {
    let sandbox = Sandbox::new();
    let repository = sandbox.repository("quality", &[("src/lib.rs", "")]);
    let finding = compiler_message("warning", Some("demo"), ("src/lib.rs", 1), None);
    let cargo = sandbox.fake_cargo(&format!(
        "case \"$(printenv FAKE_CASE || echo clean)\" in\nfinding)\n{}\n;;\nbroken)\necho raw-failure >&2\nexit 101\n;;\nesac",
        emit(&[finding])
    ));
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
    assert_eq!(found.code, Some(1), "{}", found.text);
    assert!(
        found
            .text
            .contains("clippy: src/lib.rs:1: warning [demo]: warning finding"),
        "{}",
        found.text
    );
    let entries: serde_json::Value =
        serde_json::from_str(&read(&report)).expect("report should be JSON");
    assert_eq!(entries[0]["check_name"], "demo");

    let broken = output(sandbox.command(&args).env("FAKE_CASE", "broken"));
    assert_eq!(broken.code, Some(1), "{}", broken.text);
    assert!(broken.text.contains("raw-failure"), "{}", broken.text);
    assert_eq!(read(&report), "[]\n");

    let clean = sandbox.run(&args);
    assert_eq!(clean.code, Some(0), "{}", clean.text);
    assert_eq!(read(&report), "[]\n");
}

/// Skipping every phase succeeds and still writes a valid empty report.
#[test]
fn empty_selection_writes_an_empty_report() {
    let sandbox = Sandbox::new();
    let report = sandbox.path("report.json");

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
    let repository = sandbox.repository(
        "changed",
        &[
            ("src/lib.rs", "one\ntwo\nthree\n"),
            ("crates/a/Cargo.toml", "[package]\nname = \"a\"\n"),
        ],
    );
    write(&repository.join("src/lib.rs"), "one\ntwo\nTHREE\n");
    write(
        &repository.join("crates/a/Cargo.toml"),
        "[package]\nname = \"b\"\n",
    );
    commit_all(&repository, "change");
    let unchanged = compiler_message("error", Some("demo"), ("src/lib.rs", 1), None);
    let changed = compiler_message("error", Some("demo"), ("src/lib.rs", 3), None);
    let manifest = compiler_message("warning", Some("demo"), ("crates/a/Cargo.toml", 2), None);
    let blocking = compiler_message("error", Some("E0425"), ("src/lib.rs", 1), None);
    let cargo = sandbox.fake_cargo(&format!(
        "case \"$(printenv FAKE_CASE || echo clean)\" in\nfindings)\n{}\nexit 101\n;;\nblocking)\n{}\nexit 101\n;;\nbroken)\nexit 101\n;;\nesac",
        emit(&[unchanged, changed, manifest]),
        emit(&[blocking])
    ));
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
    let sandbox = Sandbox::new();
    let (repository, cargo) = changed_range_repository(&sandbox);

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
    let sandbox = Sandbox::new();
    let (repository, cargo) = changed_range_repository(&sandbox);

    let clean = run_changed_range(&sandbox, &repository, &cargo, "clean");
    assert_output(&clean, 0, &["clippy: no diagnostics on changed lines."]);

    // A compiler error outside the range or a failure without JSON means the run is incomplete.
    for case in ["blocking", "broken"] {
        assert_output(
            &run_changed_range(&sandbox, &repository, &cargo, case),
            1,
            &["clippy: command failed before changed-range linting completed; inspect logs."],
        );
    }
}

/// An unknown revision range or a missing Git executable fails before any phase runs.
#[test]
fn changed_range_reports_git_failures() {
    let sandbox = Sandbox::new();
    let repository = sandbox.repository("git", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo("true");
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
    assert_eq!(unknown.code, Some(1), "{}", unknown.text);
    assert!(
        unknown
            .text
            .contains("Git could not resolve changed range `missing-revision..HEAD`"),
        "{}",
        unknown.text
    );

    let no_git = output(run("HEAD").env("PATH", sandbox.path("empty-path")));
    assert_eq!(no_git.code, Some(1), "{}", no_git.text);
    assert!(
        no_git
            .text
            .contains("could not run Git for changed-range filtering"),
        "{}",
        no_git.text
    );
}

/// A long phase prints heartbeats at the requested interval.
#[test]
fn heartbeat_reports_long_running_phases() {
    let sandbox = Sandbox::new();
    let repository = sandbox.repository("heartbeat", &[("src/lib.rs", "")]);
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
    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
    let file = sandbox.path("file");
    write(&file, "not a repository");

    let missing = sandbox.run(&["--repo", text(&sandbox.path("missing"))]);
    assert_eq!(missing.code, Some(1), "{}", missing.text);
    assert!(
        missing.text.contains("could not resolve"),
        "{}",
        missing.text
    );

    let not_directory = sandbox.run(&["--repo", text(&file)]);
    assert_eq!(not_directory.code, Some(1), "{}", not_directory.text);
    assert!(
        not_directory.text.contains("not a directory:"),
        "{}",
        not_directory.text
    );
    assert!(!sandbox.path("cache").exists());
}

/// Cache candidates inside the repository, relative, or unwritable are all skipped.
#[test]
fn unsafe_cache_candidates_are_rejected() {
    let sandbox = Sandbox::new();
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

    assert_eq!(output.code, Some(1), "{}", output.text);
    assert!(
        output
            .text
            .contains("no writable Sagan-lints cache directory"),
        "{}",
        output.text
    );
    assert!(output.text.contains("relative-cache"), "{}", output.text);
    // Rejection happens before anything is created through the symbolic link.
    assert!(!repository.join("sagan-lints").exists());
    assert!(!repository.join("cache").exists());
}

/// The target-cache limit must be an unsigned decimal byte count.
#[test]
fn invalid_target_cache_limits_are_rejected() {
    let sandbox = Sandbox::new();
    let repository = sandbox.repository("limit", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo("true");
    let args = [
        "--repo",
        text(&repository),
        "--cargo-cmd",
        &cargo,
        "--skip-dylint",
    ];

    for value in [OsStr::new("-1"), OsStr::from_bytes(b"\xff")] {
        let output = output(
            sandbox
                .command(&args)
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
    let sandbox = Sandbox::new();
    let repository = fixture("cargo-install-target");
    let empty_sysroot = sandbox.path("sysroot");
    fs::create_dir_all(&empty_sysroot).expect("sysroot should be creatable");

    for (sysroot, missing) in [
        (
            sandbox.path("missing-sysroot"),
            sandbox.path("missing-sysroot"),
        ),
        (empty_sysroot.clone(), empty_sysroot.join("bin/cargo")),
    ] {
        let output = output(
            sandbox
                .command(&["--repo", text(&repository), "--dry-run"])
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
    let sandbox = Sandbox::new();
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
    for limit in [None, Some("0"), Some("000")] {
        let kept = run(limit);
        assert_eq!(kept.code, Some(0), "{}", kept.text);
        assert!(sandbox.managed_target().join("build/artifact").is_file());
    }

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

/// An early failure still prunes an oversized managed target directory.
#[test]
fn managed_target_cache_is_pruned_after_early_failure() {
    let sandbox = Sandbox::new();
    let repository = sandbox.repository("early-failure", &[("src/lib.rs", "")]);
    let cargo = sandbox.fake_cargo(ARTIFACT_CARGO);
    let args = managed_args(&repository, &cargo);
    let built = output(
        sandbox
            .command(&args)
            .env("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES", "0"),
    );
    assert_eq!(built.code, Some(0), "{}", built.text);
    let target = sandbox.managed_target();

    let mut failing = args.to_vec();
    failing.extend(["--changed-range", "missing-revision..HEAD"]);
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
    for (mode, message) in [
        ("000", "could not measure managed target directory"),
        ("555", "could not prune managed target directory"),
    ] {
        let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();
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
    let sandbox = Sandbox::new();

    let category = output(driver(&sandbox, &[]).env("SAGAN_LINTS_DRIVER_CATEGORIES", "bogus"));
    assert_eq!(category.code, Some(1), "{}", category.text);
    assert!(
        category
            .text
            .contains("sagan-lints compiler driver: unknown embedded lint category `bogus`"),
        "{}",
        category.text
    );

    let sysroot = output(driver(&sandbox, &[]).env("SAGAN_LINTS_SYSROOT", sandbox.path("missing")));
    assert_eq!(sysroot.code, Some(1), "{}", sysroot.text);
    assert!(
        sysroot
            .text
            .contains("sagan-lints compiler driver: packaged asset does not exist"),
        "{}",
        sysroot.text
    );

    // Without arguments, rustc itself reports the missing input.
    let empty = output(&mut driver(&sandbox, &[]));
    assert_eq!(empty.code, Some(1), "{}", empty.text);
}

/// In no-deps mode, only Cargo's primary packages receive the selected private lints.
#[test]
fn compiler_driver_lints_only_primary_packages_in_no_deps_mode() {
    let sandbox = Sandbox::new();
    let source = sandbox.path("lib.rs");
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

    let dependency = run(false);
    assert_eq!(dependency.code, Some(0), "{}", dependency.text);

    let primary = run(true);
    assert_eq!(primary.code, Some(1), "{}", primary.text);
    assert!(
        primary.text.contains("string_error_result"),
        "{}",
        primary.text
    );
}
