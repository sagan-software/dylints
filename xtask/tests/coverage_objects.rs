#![expect(
    unused_crate_dependencies,
    reason = "the wrapper test uses process and temporary-file dependencies"
)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the test compiles isolated library variants"
)]
//! Preserve distinct linked library variants before a later build replaces their path.
//!
//! These process tests use the actual compiler and Dylint linker in temporary
//! directories. They verify exact retained bytes, metadata-only compilation,
//! compiler and linker failures, and missing archive configuration. Every case
//! keeps its source, linked output and archive separate from the workspace.

use std::{env, fs, path::Path, process::Command};
use tempfile::TempDir;

/// Construct one private compiler source and archive directory.
fn fixture() -> TempDir {
    // Keep generated libraries and all retained variants inside this fixture.
    let directory = tempfile::tempdir().expect("isolated compiler output");
    fs::create_dir(directory.path().join("objects")).expect("archive directory");
    fs::write(
        directory.path().join("probe.rs"),
        "#[unsafe(no_mangle)] pub fn probe() -> u8 { 1 }\n",
    )
    .expect("source");
    directory
}

/// Configure the real compiler with the coverage linker and an isolated output.
fn compiler(root: &Path) -> Command {
    // Preserve compiler command extraction by configuring only its linker.
    let mut command = Command::new("rustc");
    let _configured = command
        .arg(root.join("probe.rs"))
        .args([
            "--crate-name",
            "probe",
            "--crate-type",
            "cdylib",
            "--out-dir",
        ])
        .arg(root)
        .args(["--edition=2024", "-C", "instrument-coverage"])
        .arg("-C")
        .arg(linker_argument());
    command
}

/// Compile one variant and read the bytes that the next build can overwrite.
fn linked_bytes(root: &Path) -> Vec<u8> {
    // A successful real link is a prerequisite for comparing retained bytes.
    assert!(
        compiler(root)
            .env("DYLINT_COVERAGE_OBJECTS", root.join("objects"))
            .status()
            .expect("instrumented link")
            .success()
    );
    fs::read(root.join(library_filename())).expect("linked bytes")
}

/// A later constituent build must not erase the first instrumented library.
#[test]
fn retains_each_linked_library_variant() {
    // Build the first source before replacing the same output path.
    let directory = fixture();
    let root = directory.path();
    let first = linked_bytes(root);
    // Change executable behavior so the two binaries discriminate overwritten variants.
    fs::write(
        root.join("probe.rs"),
        "#[unsafe(no_mangle)] pub fn probe() -> u8 { 2 }\n",
    )
    .expect("second source");
    let second = linked_bytes(root);
    assert_ne!(first, second);
    // Compare the complete archive, including its exact cardinality and bytes.
    let mut retained: Vec<_> = fs::read_dir(root.join("objects"))
        .expect("archive")
        .map(|entry| fs::read(entry.expect("entry").path()).expect("archived bytes"))
        .collect();
    let mut expected = vec![first, second];
    retained.sort();
    expected.sort();
    assert_eq!(retained, expected);
}

/// Metadata-only compilation succeeds without producing an archived library.
#[test]
fn metadata_does_not_create_archived_output() {
    // Metadata skips linking and therefore requires no archive setting.
    let directory = fixture();
    let root = directory.path();
    let result = compiler(root)
        .arg("--emit=metadata")
        .env_remove("DYLINT_COVERAGE_OBJECTS")
        .output()
        .expect("metadata probe");
    assert!(result.status.success());
    // An empty archive distinguishes metadata from a successful library link.
    assert_eq!(
        fs::read_dir(root.join("objects")).expect("archive").count(),
        0
    );
}

/// Compiler failure retains its exit code and cannot create an archived library.
#[test]
fn compiler_failure_creates_no_archive() {
    // Reject invalid source before any linker invocation can preserve output.
    let directory = fixture();
    let root = directory.path();
    fs::write(root.join("probe.rs"), "invalid Rust\n").expect("invalid source");
    let result = compiler(root)
        .env("DYLINT_COVERAGE_OBJECTS", root.join("objects"))
        .output()
        .expect("failed compilation");
    // Both failure status and an empty archive are observable at the process boundary.
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(
        fs::read_dir(root.join("objects")).expect("archive").count(),
        0
    );
}

/// Linker failure retains a failing status and cannot archive its missing output.
#[test]
fn linker_failure_creates_no_archive() {
    // A missing object makes the real native linker fail before the output exists.
    let directory = fixture();
    let root = directory.path();
    let result = Command::new(env!("CARGO_BIN_EXE_coverage-link"))
        .arg("-o")
        .arg(root.join(library_filename()))
        .arg(root.join("missing.o"))
        .env("DYLINT_COVERAGE_OBJECTS", root.join("objects"))
        .output()
        .expect("failed link");
    // No partial or invented binary may enter the archive after a failed link.
    assert!(!result.status.success());
    assert_eq!(
        fs::read_dir(root.join("objects")).expect("archive").count(),
        0
    );
}

/// A successful link without archive configuration fails with its setting named.
#[test]
fn reports_missing_archive() {
    // Remove configuration only from this child; the parent environment remains intact.
    let directory = fixture();
    let result = compiler(directory.path())
        .env_remove("DYLINT_COVERAGE_OBJECTS")
        .output()
        .expect("missing archive");
    assert!(!result.status.success());
    // The diagnostic identifies the missing input rather than discarding the error.
    assert!(String::from_utf8_lossy(&result.stderr).contains("DYLINT_COVERAGE_OBJECTS is missing"));
}

/// An unavailable linker remains a wrapper error with an observable diagnostic.
#[test]
fn reports_missing_linker() {
    // Restrict lookup to an empty private directory without changing global PATH.
    let directory = tempfile::tempdir().expect("isolated linker lookup");
    let result = Command::new(env!("CARGO_BIN_EXE_coverage-link"))
        .args(["-o", "unused"])
        .env("PATH", directory.path())
        .output()
        .expect("missing linker");
    assert!(!result.status.success());
    // A wrapper diagnostic retains the operation responsible for the lookup failure.
    assert!(String::from_utf8_lossy(&result.stderr).contains("coverage-link:"));
}

/// Select the real linker wrapper without changing compiler argument extraction.
fn linker_argument() -> String {
    let linker = env!("CARGO_BIN_EXE_coverage-link");
    format!("linker={linker}")
}

/// Use the supported host's actual shared-library spelling.
fn library_filename() -> String {
    let prefix = env::consts::DLL_PREFIX;
    let suffix = env::consts::DLL_SUFFIX;
    format!("{prefix}probe{suffix}")
}
