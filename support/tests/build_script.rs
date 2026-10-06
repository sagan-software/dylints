#![expect(
    unused_crate_dependencies,
    reason = "bootstrap tests execute the build script directly"
)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "bootstrap tests own isolated files and compiler processes"
)]
//! Verify compiler discovery through the actual Cargo build-script executable.
//!
//! Each child receives an isolated compiler selection and executable search
//! path. Tests exercise valid toolchain layouts, symlink resolution, ordered
//! lookup, missing configuration, invalid layouts and fallible protocol output.
//! Instrumented runs retain the compiled script beside other coverage objects
//! so its real bootstrap branches contribute to the source coverage report.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::OnceLock,
};
use tempfile::TempDir;

/// Compile the unchanged build-script source once and retain its private directory.
fn bootstrap_script() -> &'static Path {
    static SCRIPT: OnceLock<(TempDir, PathBuf)> = OnceLock::new();
    &SCRIPT
        .get_or_init(|| {
            let directory = tempfile::tempdir().expect("bootstrap compiler directory");
            let binary = directory.path().join("bootstrap");
            let mut compiler = Command::new("rustc");
            let _configured = compiler
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"))
                .args(["--edition=2024", "-o"])
                .arg(&binary);
            // Instrument the actual script only when the coverage task owns an archive.
            let archive = env::var_os("DYLINT_COVERAGE_OBJECTS");
            if archive.is_some() {
                let _configured = compiler.args(["-C", "instrument-coverage"]);
            }
            assert!(compiler.status().expect("compile bootstrap").success());
            // Keep this executable's mapping after the private test directory disappears.
            if let Some(archive) = archive {
                let retained = tempfile::NamedTempFile::new_in(archive).expect("bootstrap archive");
                let _bytes = fs::copy(&binary, retained.path()).expect("archive bootstrap bytes");
                let _retained = retained.keep().expect("retain bootstrap mapping");
            }
            (directory, binary)
        })
        .1
}

/// Construct a compiler file in the conventional toolchain layout.
fn toolchain(root: &Path, has_libraries: bool) -> PathBuf {
    // The bootstrap infers layout without executing this compiler file.
    fs::create_dir_all(root.join("bin")).expect("compiler directory");
    if has_libraries {
        fs::create_dir(root.join("lib")).expect("compiler libraries");
    }
    // Preserve a regular compiler file independently of optional libraries.
    let compiler = root.join("bin/rustc");
    fs::write(&compiler, "compiler fixture").expect("compiler file");
    compiler
}

/// Execute one script with configuration confined to the child process.
fn command() -> Command {
    let mut command = Command::new(bootstrap_script());
    let _configured = command.env_remove("RUSTC").env_remove("PATH");
    command
}

/// Successful compiler discovery emits the complete native-link protocol.
#[test_case::test_case(false; "absolute compiler")]
#[test_case::test_case(true; "symlinked compiler")]
fn emits_selected_toolchain(is_symlinked: bool) {
    // Keep the launcher and compiler libraries inside the same private fixture.
    let directory = tempfile::tempdir().expect("toolchain fixture");
    let root = directory.path();
    let compiler = toolchain(root, true);
    // Symlinked launchers resolve to the selected compiler's toolchain.
    let selected = if is_symlinked {
        let link = root.join("selected");
        std::os::unix::fs::symlink(&compiler, &link).expect("compiler symlink");
        link
    } else {
        compiler
    };
    let result = command()
        .env("RUSTC", selected)
        .output()
        .expect("bootstrap process");
    assert!(result.status.success());
    // Compare all directives, including ordering and the environment rerun rule.
    let libraries = root.join("lib");
    let libraries = libraries.display();
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        format!(
            "cargo:rustc-link-search=native={libraries}\ncargo:metadata=compiler-libraries={libraries}\ncargo:rerun-if-env-changed=RUSTC\n"
        )
    );
}

/// PATH lookup skips directories and selects the first regular compiler file.
#[test]
fn respects_path_order() {
    // Keep each PATH candidate separate from the invoking environment.
    let directory = tempfile::tempdir().expect("lookup fixture");
    let root = directory.path();
    // Isolated candidates distinguish skipped directories and accepted files.
    let path = ordered_lookup(root);
    let first = root.join("first");
    let second = root.join("second");
    // The first accepted candidate determines both emitted paths.
    let result = command()
        .env("RUSTC", "rustc")
        .env("PATH", path)
        .output()
        .expect("lookup process");
    assert!(result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stdout).contains(&first.join("lib").display().to_string())
    );
    assert!(!String::from_utf8_lossy(&result.stdout).contains(&second.display().to_string()));
}

/// Install ordered compiler candidates and return their private executable path.
fn ordered_lookup(root: &Path) -> std::ffi::OsString {
    // A directory named rustc must be skipped before inspecting regular files.
    fs::create_dir_all(root.join("skip/rustc")).expect("non-file compiler");
    let first = root.join("first");
    let second = root.join("second");
    let _first = toolchain(&first, true);
    let _second = toolchain(&second, true);
    // Later valid candidates must not replace the first accepted candidate.
    let path = env::join_paths([root.join("skip"), first.join("bin"), second.join("bin")])
        .expect("ordered path");
    // Return only the configuration consumed by the child process.
    path
}

/// Missing configuration and absent compiler files fail before Cargo output.
#[test_case::test_case(None, None, "Cargo did not provide the RUSTC executable"; "missing compiler")]
#[test_case::test_case(Some("rustc"), None, "PATH is not set"; "missing search path")]
#[test_case::test_case(Some("rustc"), Some(""), "compiler executable was not found on PATH"; "unresolved compiler")]
#[test_case::test_case(Some("./missing-rustc"), None, "No such file or directory"; "absent compiler file")]
fn rejects_configuration(rustc: Option<&str>, path: Option<&str>, error: &str) {
    // Use an empty working directory so absent compiler files cannot resolve.
    let directory = tempfile::tempdir().expect("configuration fixture");
    let mut child = command();
    let _configured = child.current_dir(directory.path());
    if let Some(rustc) = rustc {
        let _configured = child.env("RUSTC", rustc);
    }
    if let Some(path) = path {
        let _configured = child.env("PATH", path);
    }
    // Configuration rejection must precede any Cargo directive.
    let result = child.output().expect("configuration process");
    assert_failure(&result, error);
}

/// Malformed toolchain layouts fail before any dependent link search is emitted.
#[test_case::test_case(false; "missing libraries")]
#[test_case::test_case(true; "wrong compiler parent")]
fn rejects_layout(has_wrong_parent: bool) {
    // Each fixture violates exactly one layout requirement.
    let directory = tempfile::tempdir().expect("layout fixture");
    let compiler = if has_wrong_parent {
        let compiler = directory.path().join("rustc");
        fs::write(&compiler, "compiler fixture").expect("compiler file");
        compiler
    } else {
        toolchain(directory.path(), false)
    };
    // Resolve the selected file before diagnosing its toolchain layout.
    let result = command()
        .env("RUSTC", compiler)
        .output()
        .expect("layout process");
    // Preserve the sentinel for the rejected layout boundary.
    let error = if has_wrong_parent {
        "rustc is not inside a toolchain bin directory"
    } else {
        "rustc compiler library directory does not exist"
    };
    assert_failure(&result, error);
}

/// A canonical root has no parent and cannot identify a toolchain.
#[test]
fn rejects_compiler_without_parent() {
    let directory = tempfile::tempdir().expect("root fixture");
    let mut selected = directory.path().to_path_buf();
    // Parent components resolve an existing private path to the filesystem root.
    for _component in directory.path().components().skip(1) {
        selected.push("..");
    }
    // Root canonicalization must fail before any native-link directive.
    let result = command()
        .env("RUSTC", selected)
        .output()
        .expect("root compiler process");
    assert_failure(&result, "rustc has no parent directory");
}

/// Output failures remain errors after successful compiler validation.
#[test]
fn propagates_output_failure() {
    // A valid toolchain isolates protocol output from compiler discovery errors.
    let directory = tempfile::tempdir().expect("output fixture");
    let compiler = toolchain(directory.path(), true);
    let (reader, writer) = std::os::unix::net::UnixStream::pair().expect("output pipe");
    drop(reader);
    let writer: std::os::fd::OwnedFd = writer.into();
    // A closed reader rejects the script's protocol write with BrokenPipe.
    let result = command()
        .env("RUSTC", compiler)
        .stdout(Stdio::from(writer))
        .output()
        .expect("output process");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("BrokenPipe"));
}

/// Rejected bootstrap inputs preserve their sentinel and emit no Cargo directives.
fn assert_failure(result: &Output, error: &str) {
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains(error));
}
