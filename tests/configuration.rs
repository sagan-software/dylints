#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the synchronous integration test owns fixture files and runner processes"
)]
#![allow(
    unused_crate_dependencies,
    reason = "the runner package embeds lint libraries that this integration test does not import"
)]

//! These tests run the bundled compiler against one small Cargo workspace with
//! changing lint limits.
//! They reuse the workspace's target directory to check that configuration
//! changes invalidate cached
//! compiler checks. Inherited configurations must take precedence over the
//! workspace file, including
//! an empty override. Malformed workspace configuration must retain Dylint's
//! configuration error.

use std::{fs, path::Path, process::Command};

/// Run the restriction category with one optional inherited configuration.
fn check(repo: &Path, inherited: Option<&str>) -> (bool, String) {
    // Give each invocation the same target cache and an explicit override boundary.
    let mut command = Command::new(env!("CARGO_BIN_EXE_sagan-lints"));
    let _configured = command
        .args([
            "--skip-clippy",
            "--dylint-category",
            "restriction",
            "--no-all-targets",
            "--heartbeat-seconds",
            "0",
        ])
        .arg("--repo")
        .arg(repo)
        .arg("--target-dir")
        .arg(repo.join("target"))
        .env("SAGAN_LINTS_CACHE_DIR", repo.join("cache"))
        .env_remove("DYLINT_TOML");
    if let Some(inherited) = inherited {
        let _configured = command.env("DYLINT_TOML", inherited);
    }
    // Capture both streams because the runner preserves child diagnostic routing.
    let output = command.output().expect("configuration test setup");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    (output.status.success(), format!("{stdout}{stderr}"))
}

/// Configuration changes rerun checks while inherited overrides retain their
/// precedence.
#[test]
fn preserves_overrides_and_invalidates_cached_configuration() {
    // Two source lines distinguish the relaxed and strict crate-size limits.
    let directory = tempfile::tempdir().expect("configuration test setup");
    let repo = directory.path();
    fs::create_dir(repo.join("src")).expect("configuration test setup");
    fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"configured\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n",
    )
    .expect("configuration test setup");
    fs::write(repo.join("src/main.rs"), "//! The runner checks this binary with the restriction category. Its crate-size limit changes between invocations. An inherited override must keep its configured limit. A changed workspace limit must rerun the compiler and reject this crate when the source exceeds that limit.\nfn main() {}\n").expect("configuration test setup");
    let relaxed = "[large_rust_crate]\nnon_test_line_limit = 1000\ntotal_line_limit = 1000\n";
    let strict = "[large_rust_crate]\nnon_test_line_limit = 1\ntotal_line_limit = 1000\n";
    // A changed workspace limit must invalidate the successful cached check.
    assert_configuration(repo, relaxed, None, None);
    assert_configuration(repo, strict, None, Some("large_rust_crate"));
    // Both configured and empty inherited values take precedence over the strict file.
    assert_configuration(repo, strict, Some(relaxed), None);
    assert_configuration(repo, strict, Some(""), None);
    // Malformed files retain Dylint's existing configuration error.
    assert_configuration(
        repo,
        "[malformed\n",
        None,
        Some("could not read configuration file"),
    );
}

/// Check one transition while retaining all preceding Cargo cache state.
fn assert_configuration(
    repo: &Path,
    source: &str,
    inherited: Option<&str>,
    diagnostic: Option<&str>,
) {
    // Change configuration without touching source files or clearing compiled artifacts.
    fs::write(repo.join("dylint.toml"), source).expect("configuration test setup");
    let (is_success, output) = check(repo, inherited);
    assert_eq!(is_success, diagnostic.is_none(), "{output}");
    // A failure must report the expected lint or configuration error.
    if let Some(diagnostic) = diagnostic {
        assert!(output.contains(diagnostic), "{output}");
    }
}

/// External dependencies remain outside the workspace compiler wrapper.
#[test]
fn preserves_dependency_configuration() {
    // Separate workspaces distinguish dependency discovery from the runner's root.
    let directory = tempfile::tempdir().expect("configuration test setup");
    let dependency = tempfile::tempdir().expect("configuration test setup");
    write_package(dependency.path(), "external_dependency", "");
    let path = serde_json::to_string(dependency.path()).expect("configuration test setup");
    let dependencies = format!("[dependencies]\nexternal_dependency = {{ path = {path} }}\n");
    write_package(directory.path(), "configured", &dependencies);
    fs::write(directory.path().join("dylint.toml"), "").expect("configuration test setup");
    fs::write(dependency.path().join("dylint.toml"), "[malformed\n")
        .expect("configuration test setup");
    // External dependencies use plain rustc and do not parse Dylint configuration.
    let (is_success, output) = check(directory.path(), None);
    assert!(is_success, "{output}");
    // An inherited empty override bypasses the workspace file; dependencies retain plain rustc.
    let (is_success, output) = check(directory.path(), Some(""));
    assert!(is_success, "{output}");
}

/// Create one standalone package with optional Cargo dependency declarations.
fn write_package(root: &Path, name: &str, dependencies: &str) {
    fs::create_dir(root.join("src")).expect("configuration test setup");
    let manifest = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n{dependencies}"
    );
    fs::write(root.join("Cargo.toml"), manifest).expect("configuration test setup");
    // The source satisfies documentation policy without triggering restriction lints.
    fs::write(root.join("src/lib.rs"), "//! This standalone package checks configuration discovery at a Cargo dependency boundary. The workspace compiler wrapper must exclude this dependency. Its malformed configuration must remain unread by Dylint. The fixture has no public functions or additional dependencies and uses one source file.\n").expect("configuration test setup");
}
