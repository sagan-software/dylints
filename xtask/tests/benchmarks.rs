#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the synchronous benchmark harness owns compiler processes and workload files"
)]
#![expect(
    unused_crate_dependencies,
    reason = "the xtask package has dependencies used by other development commands"
)]

//! These tests check benchmark discovery against Cargo metadata and generated
//! compiler workloads.
//! They verify category exclusion, unique lint identities, build-directory
//! selection, deterministic
//! source generation, and file-system failures. The Criterion harness uses the
//! same support module,
//! so the tests exercise its inventory and workload construction without
//! running timing loops.

#[path = "../benches/support.rs"]
mod support;

use std::{fs, process::Command};

use self::support::{file_size_group, lints, workload};

/// Read the workspace metadata through Cargo's documented JSON interface.
fn metadata() -> serde_json::Value {
    // Reject metadata failures before checking inventory contracts.
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("run Cargo metadata");
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("decode Cargo metadata")
}

/// Every leaf lint has one sorted, unique benchmark identity.
#[test]
#[expect(
    many_assertions_in_test,
    reason = "The inventory has separate count, sorting, uniqueness, and metadata contracts."
)]
fn benchmark_inventory_covers_leaf_lints() {
    // Coverage counts identities once and keeps their registration order stable.
    let metadata = metadata();
    let inventory = lints(&metadata, "test-toolchain");
    // Check the total independently from the shape and ordering of the entries.
    assert_eq!(
        inventory.len(),
        297,
        "each registered leaf lint is measured"
    );
    assert!(inventory.iter().map(|lint| &lint.name).is_sorted());
    let names: std::collections::BTreeSet<_> = inventory.iter().map(|lint| &lint.name).collect();
    assert_eq!(names.len(), inventory.len());
    assert!(
        inventory.iter().all(|lint| {
            metadata["packages"]
                .as_array()
                .expect("packages")
                .iter()
                .find(|package| package["name"] == lint.name)
                .and_then(|package| package.pointer("/metadata/dylint/category"))
                .and_then(serde_json::Value::as_str)
                .is_some()
        }),
        "only packages with a Dylint category are leaf lints"
    );
}

/// Inventory names exclude categories and retain the canonical package identity.
#[test]
#[expect(
    many_assertions_in_test,
    reason = "The test protects package identity, group exclusion, and toolchain-qualified paths."
)]
fn benchmark_inventory_preserves_leaf_library_identity() {
    // Categories are not leaf identities, and filenames retain the toolchain.
    let inventory = lints(&metadata(), "test-toolchain");
    let size_lint = inventory
        .iter()
        .find(|lint| lint.name == "large-rust-file")
        .expect("leaf lint identity");
    // Package specs must retain both the kebab-case crate identity and workspace version.
    assert_eq!(size_lint.package_spec, "large-rust-file@0.3.0");
    let bevy_lint = inventory
        .iter()
        .find(|lint| lint.name == "bevy-duplicate-dependencies")
        .expect("crate-specific lint identity");
    assert_eq!(bevy_lint.package_spec, "bevy-duplicate-dependencies@0.3.0");
    assert!(!inventory.iter().any(|lint| lint.name == "restriction"));
    assert!(
        inventory
            .iter()
            .all(|lint| lint.library.to_string_lossy().contains("@test-toolchain"))
    );
}

/// Fast and full file-size samples never share Criterion comparison baselines.
#[test]
fn file_size_benchmark_groups_match_the_sampling_mode() {
    assert_eq!(file_size_group(true), "file_size_fast");
    assert_eq!(file_size_group(false), "file_size");
}

/// Workloads have deterministic source and an independent Cargo workspace.
#[test]
fn compiler_workload_is_deterministic() {
    // Generate two modules from the same function specification.
    let directory = tempfile::tempdir().unwrap();
    let root = workload(directory.path(), 2, 3).unwrap();
    assert_eq!(
        fs::read_to_string(root).unwrap(),
        "mod part_0;\nmod part_1;\n"
    );
    // Equal modules must retain the same function count and source text.
    let first = fs::read_to_string(directory.path().join("src/part_0.rs")).unwrap();
    let second = fs::read_to_string(directory.path().join("src/part_1.rs")).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.matches("pub fn compute_").count(), 3);
}

/// Generated projects are independent Cargo workspaces with a pinned toolchain
/// and empty Clippy configuration.
#[test]
fn compiler_workload_is_an_independent_cargo_project() {
    // Isolated project files prevent discovery from inheriting this workspace.
    let directory = tempfile::tempdir().unwrap();
    let _source = workload(directory.path(), 1, 1).unwrap();
    assert!(
        fs::read_to_string(directory.path().join("Cargo.toml"))
            .unwrap()
            .contains("[workspace]")
    );
    assert!(directory.path().join("rust-toolchain.toml").is_file());
    assert_eq!(
        fs::read_to_string(directory.path().join("clippy.toml")).unwrap(),
        ""
    );
}

/// Empty workloads and file-system failures preserve their explicit outcomes.
#[test]
fn compiler_workload_handles_empty_and_unwritable_destinations() {
    // Empty projects remain valid, even without generated module items.
    let directory = tempfile::tempdir().unwrap();
    let root = workload(directory.path(), 0, 0).unwrap();
    assert!(fs::read_to_string(root).unwrap().is_empty());
    // A file cannot become a project directory; retain the I/O failure.
    let blocker = directory.path().join("file");
    fs::write(&blocker, "occupied").unwrap();
    let _error = workload(&blocker, 1, 1).expect_err("a file cannot become a project directory");
}

/// Each workload write propagates its file-system failure.
#[test_case::test_case("Cargo.toml"; "manifest")]
#[test_case::test_case("rust-toolchain.toml"; "toolchain")]
#[test_case::test_case("clippy.toml"; "clippy configuration")]
#[test_case::test_case("src/part_0.rs"; "module")]
#[test_case::test_case("src/lib.rs"; "crate root")]
fn compiler_workload_propagates_write_failures(blocked: &str) {
    // A directory at each file destination must retain the original I/O failure.
    let directory = tempfile::tempdir().expect("workload fixture");
    let blocked = directory.path().join(blocked);
    fs::create_dir_all(&blocked).expect("occupied workload destination");
    let _error = workload(directory.path(), 1, 1).expect_err("occupied file destination");
    assert!(blocked.is_dir());
}

/// Check the number of discovered libraries after one fixture-state transition.
fn assert_inventory_size(metadata: &serde_json::Value, count: usize) {
    assert_eq!(lints(metadata, "toolchain").len(), count);
}

/// Exclude a configured category even when its files match a leaf layout.
fn assert_category_excluded(dynamic: &serde_json::Value, directory: &std::path::Path) {
    // Preserve the leaf fixture while adding a separate category declaration.
    let mut category = dynamic.clone();
    // Category paths come from existing workspace metadata, not another package list.
    let object = category.as_object_mut().expect("metadata fixture object");
    let _previous = object.insert("workspace_root".into(), serde_json::json!(directory));
    let _previous = object.insert(
        "metadata".into(),
        serde_json::json!({"dylint": {"libraries": [{"path": "."}]}}),
    );
    assert_inventory_size(&category, 0);
}

/// Model one Cargo library package without conflating file presence and target kind.
fn fixture_metadata(directory: &std::path::Path, crate_type: &str) -> serde_json::Value {
    serde_json::json!({
        "target_directory": directory,
        "packages": [{"name": "candidate-lint", "version": "0.1.0", "metadata": {"dylint": {"category": "test"}}, "manifest_path": directory.join("Cargo.toml"),
            "targets": [{"crate_types": [crate_type]}]}]
    })
}

/// Discovery excludes groups, support crates, and static libraries
/// independently.
#[test]
fn benchmark_inventory_excludes_non_leaf_packages() {
    // Add required files independently so missing-file guards stay observable.
    let directory = tempfile::tempdir().unwrap();
    let metadata = fixture_metadata(directory.path(), "rlib");
    assert_inventory_size(&metadata, 0);
    fs::write(directory.path().join("README.md"), "# candidate").unwrap();
    assert_inventory_size(&metadata, 0);
    fs::create_dir(directory.path().join("ui")).unwrap();
    assert_inventory_size(&metadata, 0);
    // A dynamic target becomes loadable only after documentation and UI exist.
    let dynamic = fixture_metadata(directory.path(), "cdylib");
    assert_inventory_size(&dynamic, 1);
}

/// Categories remain excluded while example-only leaf packages remain eligible.
#[test]
fn benchmark_inventory_checks_categories_and_examples() {
    let directory = documented_leaf();
    let dynamic = fixture_metadata(directory.path(), "cdylib");
    // Category declarations override otherwise valid leaf files.
    assert_category_excluded(&dynamic, directory.path());
    // Example-based lints remain eligible after the UI directory disappears.
    fs::remove_dir(directory.path().join("ui")).unwrap();
    fs::create_dir(directory.path().join("examples")).unwrap();
    assert_inventory_size(&dynamic, 1);
}

/// Aggregate packages remain excluded even when they resemble documented lint crates.
#[test]
fn benchmark_inventory_requires_lint_category_metadata() {
    // A documented package is not a leaf lint until Cargo metadata marks its category.
    let directory = documented_leaf();
    let mut metadata = fixture_metadata(directory.path(), "cdylib");
    let _removed = metadata["packages"][0]
        .as_object_mut()
        .expect("package object")
        .remove("metadata");
    assert_inventory_size(&metadata, 0);
    // Adding the authoritative Dylint category makes the same fixture eligible.
    metadata["packages"][0]["metadata"] = serde_json::json!({"dylint": {"category": "test"}});
    assert_inventory_size(&metadata, 1);
}

/// Create the files that distinguish leaf lints from support packages.
fn documented_leaf() -> tempfile::TempDir {
    // Documentation and a UI directory make this fixture eligible as a leaf lint.
    let directory = tempfile::tempdir().expect("temporary lint package");
    fs::write(directory.path().join("README.md"), "# candidate").expect("write lint documentation");
    fs::create_dir(directory.path().join("ui")).expect("create UI fixture");
    directory
}

/// Cargo's separate build directory contains the copies created by
/// `dylint-link`.
#[test]
fn benchmark_inventory_respects_separate_build_directory() {
    // Model separate build and target directories before choosing library paths.
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("README.md"), "# candidate").unwrap();
    fs::create_dir(directory.path().join("ui")).unwrap();
    let mut metadata = serde_json::json!({
        "target_directory": "/target",
        "build_directory": "/build",
        "packages": [{
            "name": "candidate-lint",
            "version": "0.1.0",
            "metadata": {"dylint": {"category": "test"}},
            "manifest_path": directory.path().join("Cargo.toml"),
            "targets": [{"crate_types": ["cdylib"]}]
        }]
    });
    let inventory = lints(&metadata, "toolchain");
    assert!(
        inventory
            .first()
            .unwrap()
            .library
            .starts_with("/build/debug")
    );
    // Missing build-directory metadata retains the documented target fallback.
    metadata["build_directory"] = serde_json::Value::Null;
    let inventory = lints(&metadata, "toolchain");
    assert!(
        inventory
            .first()
            .unwrap()
            .library
            .starts_with("/target/debug")
    );
}

/// Missing, non-text, and parentless manifests cannot identify a leaf package.
#[test_case::test_case(serde_json::Value::Null; "missing manifest")]
#[test_case::test_case(serde_json::json!(4); "nontext manifest")]
#[test_case::test_case(serde_json::json!("/"); "parentless manifest")]
fn benchmark_inventory_rejects_invalid_manifests(manifest: serde_json::Value) {
    // Retain valid files so each case isolates manifest decoding.
    let directory = documented_leaf();
    let mut metadata = fixture_metadata(directory.path(), "cdylib");
    let package = metadata
        .get_mut("packages")
        .expect("packages")
        .as_array_mut()
        .expect("package array")
        .first_mut()
        .expect("fixture package")
        .as_object_mut()
        .expect("package object");
    let _previous = package.insert("manifest_path".into(), manifest);
    assert_inventory_size(&metadata, 0);
}
