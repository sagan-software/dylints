#![feature(rustc_private)]

//! Public regression tests for dependency table iteration.
//!
//! These tests call `dependency_tables` and `dependencies` through the crate's
//! exported API. They check package and target order, dependency values, and
//! iterator exhaustion. Returned entries borrow the parsed document, which
//! remains readable. A separate manifest reverses table declarations to check
//! the fixed table-kind order.

use tempfile as _;

use cargo_support::{dependencies, dependency_tables, toml_edit::Document};
use dylint_linting as _;
use toml_edit as _;

/// Package and target dependencies used to exercise the public iterators.
const MANIFEST: &str = r#"
[dependencies]
alpha = "1"
beta = { version = "2" }

[target.'cfg(unix)'.dev-dependencies]
gamma = "3"
"#;

/// Dependency values borrow the manifest and retain declaration order through exhaustion.
#[test]
fn dependency_values_borrow_and_preserve_manifest_order() {
    let document = Document::parse(MANIFEST.to_owned()).expect("test manifest must be valid TOML");
    let mut tables = dependency_tables(document.as_table());

    // The package iterator yields both names and then remains exhausted.
    let package_table = tables.next().expect("package dependency table");
    let mut package_dependencies = dependencies(package_table);
    let alpha = package_dependencies
        .next()
        .expect("first package dependency");
    assert_eq!(
        (
            alpha.name,
            alpha.version().map(|(version, _)| version),
            package_dependencies
                .next()
                .map(|dependency| dependency.name),
            package_dependencies.next().is_none(),
        ),
        ("alpha", Some("1"), Some("beta"), true),
    );

    // The source remains readable while entries borrow its document.
    assert!(document.as_table().get("dependencies").is_some());
}

/// Target tables follow package tables, and both iterators exhaust independently.
#[test]
fn target_dependencies_follow_package_tables() {
    let document = Document::parse(MANIFEST.to_owned()).expect("test manifest must be valid TOML");
    let mut tables = dependency_tables(document.as_table());

    // Consume the package table before checking the target table.
    let package_table = tables.next().expect("package dependency table");
    assert_eq!(
        dependencies(package_table).next().map(|entry| entry.name),
        Some("alpha")
    );
    // The target table yields its dependency, then both iterators exhaust.
    let target_table = tables.next().expect("target dependency table");
    let mut target_dependencies = dependencies(target_table);
    assert_eq!(
        (
            target_dependencies.next().map(|dependency| dependency.name),
            target_dependencies.next().is_none(),
            tables.next().is_none(),
        ),
        (Some("gamma"), true, true),
    );
}

/// Table kinds follow their fixed order even when the manifest declares them differently.
#[test]
fn table_kinds_precede_target_tables_in_fixed_order() {
    // Reverse the declaration order to distinguish it from the table-kind policy.
    let source = r#"
[build_dependencies]
legacy_build = "1"
[target.'cfg(unix)'.build-dependencies]
target_build = "1"
[dev_dependencies]
legacy_dev = "1"
[build-dependencies]
build = "1"
[dependencies]
normal = "1"
[dev-dependencies]
dev = "1"
"#;
    let document = Document::parse(source.to_owned()).expect("test manifest must be valid TOML");

    // The public iterator must put all package tables before the target table.
    let names: Vec<_> = dependency_tables(document.as_table())
        .flat_map(dependencies)
        .map(|dependency| dependency.name)
        .collect();
    assert_eq!(
        names,
        [
            "normal",
            "dev",
            "legacy_dev",
            "build",
            "legacy_build",
            "target_build"
        ]
    );
}
