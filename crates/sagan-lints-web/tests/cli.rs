//! Process-level checks for the generated catalog command.
//!
//! These tests build small temporary repositories and invoke the compiled
//! catalog binary. The fixture includes package metadata, required README
//! sections, and a UI tree, so command behavior stays representative of a
//! downstream checkout. Assertions cover successful generation, path-aware
//! failures, and argument validation without checked-in report artifacts.
#![expect(
    unused_crate_dependencies,
    reason = "the integration test launches the packaged catalog binary"
)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "synchronous fixture setup and subprocess calls are appropriate for this process-level CLI test"
)]

use std::{fs, process::Command};

/// Write the smallest valid catalog input that exercises the packaged binary.
fn fixture(root: &std::path::Path) {
    // Create the directories before writing files so the fixture is self-contained.
    let lint = root.join("crates/fixture-lint");
    fs::create_dir_all(lint.join("src")).expect("lint source directory");
    fs::create_dir_all(lint.join("ui")).expect("lint UI directory");

    // Keep the README and manifest minimal while satisfying catalog validation.
    fs::write(
        lint.join("README.md"),
        "# fixture-lint\n\n## What it does\n\nChecks a fixture.\n\n## Why is this bad?\n\nExamples need a reason.\n\n## Known problems\n\nNo known problems.\n\n## Example\n\n```rust\nfn example() {}\n```\n\n## Use instead\n\nKeep the example simple.\n",
    )
    .expect("lint README");
    fs::write(
        lint.join("Cargo.toml"),
        "[package.metadata.dylint]\ncategory = \"style\"\n",
    )
    .expect("lint manifest");
    fs::write(lint.join("src/lib.rs"), "fn fixture_lint() {}\n").expect("lint source");
    fs::write(lint.join("ui/main.stderr"), "").expect("lint stderr fixture");
}

/// The packaged command renders a valid fixture and reports its generated page.
#[test]
#[expect(
    abc_size,
    reason = "This end-to-end test checks CLI setup, process status, HTML, and reported output."
)]
fn command_generates_catalog_from_arguments() {
    // Build a representative downstream crate tree in an isolated temporary directory.
    let directory = tempfile::tempdir().expect("CLI fixture directory");
    let root = directory.path();
    fixture(root);
    let lint_list = root.join("lints.txt");
    fs::write(&lint_list, "    fixture_lint    warn    Fixture\n").expect("lint registry");
    let output_directory = root.join("public");

    // Exercise the same argument boundary a consumer uses to run the packaged binary.
    let output = Command::new(env!("CARGO_BIN_EXE_sagan-lints-web"))
        .args(["--root", root.to_str().expect("UTF-8 fixture path")])
        .arg("--lint-list")
        .arg(&lint_list)
        .arg("--out-dir")
        .arg(&output_directory)
        .output()
        .expect("run catalog command");

    // Keep exit status, rendered content, and the summary line as separate assertions.
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let page = fs::read_to_string(output_directory.join("index.html")).expect("catalog page");
    assert!(page.contains("fixture_lint"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Generated "));
}

/// A missing registry reaches the command's path-aware failure status.
#[test]
fn command_reports_missing_registry() {
    let directory = tempfile::tempdir().expect("CLI failure fixture");
    let missing_list = directory.path().join("absent.txt");

    // A missing registry should retain its path in the CLI's failure message.
    let output = Command::new(env!("CARGO_BIN_EXE_sagan-lints-web"))
        .arg("--lint-list")
        .arg(&missing_list)
        .arg("--out-dir")
        .arg(directory.path().join("public"))
        .output()
        .expect("run catalog command");

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("absent.txt"));
}

/// Clap rejects a command that omits the required lint registry argument.
#[test]
fn command_rejects_missing_required_argument() {
    let output = Command::new(env!("CARGO_BIN_EXE_sagan-lints-web"))
        .output()
        .expect("run catalog command");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--lint-list"));
}
