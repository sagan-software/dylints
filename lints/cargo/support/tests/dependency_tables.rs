#![feature(rustc_private)]

//! Public-seam regression tests for dependency table iteration.

use cargo_support::{dependencies, dependency_tables, toml_edit::Document};
use dylint_linting as _;
use toml_edit as _;

#[test]
fn dependency_iterators_borrow_and_preserve_manifest_order() {
    let source = r#"
[dependencies]
alpha = "1"
beta = { version = "2" }

[target.'cfg(unix)'.dev-dependencies]
gamma = "3"
"#;
    let document = Document::parse(source.to_owned()).expect("test manifest must be valid TOML");
    let mut tables = dependency_tables(document.as_table());

    let package_table = tables.next().expect("package dependency table");
    let mut package_dependencies = dependencies(package_table);
    let alpha = package_dependencies
        .next()
        .expect("first package dependency");
    assert_eq!(alpha.name, "alpha");
    assert_eq!(alpha.version().map(|(version, _)| version), Some("1"));
    assert_eq!(
        package_dependencies
            .next()
            .map(|dependency| dependency.name),
        Some("beta")
    );
    assert!(package_dependencies.next().is_none());

    let target_table = tables.next().expect("target dependency table");
    let mut target_dependencies = dependencies(target_table);
    assert_eq!(
        target_dependencies.next().map(|dependency| dependency.name),
        Some("gamma")
    );
    assert!(target_dependencies.next().is_none());
    assert!(tables.next().is_none());

    // The source remains available while the returned values borrow its document.
    assert!(document.as_table().get("dependencies").is_some());
}
