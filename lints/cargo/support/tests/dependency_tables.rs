#![feature(rustc_private)]

//! Public-seam regression tests for dependency table iteration.

use cargo_support::{dependencies, dependency_tables, toml_edit::Document};
use dylint_linting as _;
use toml_edit as _;

/// Dependency values borrow the manifest and retain declaration order through exhaustion.
#[test]
fn dependency_iterators_borrow_and_preserve_manifest_order() {
    // Preserve package and target tables in the order a manifest declares them.
    let source = r#"
[dependencies]
alpha = "1"
beta = { version = "2" }

[target.'cfg(unix)'.dev-dependencies]
gamma = "3"
"#;
    let document = Document::parse(source.to_owned()).expect("test manifest must be valid TOML");
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

    // The target iterator follows the package table and exhausts independently.
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

    // The source remains available while the returned values borrow its document.
    assert!(document.as_table().get("dependencies").is_some());
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
