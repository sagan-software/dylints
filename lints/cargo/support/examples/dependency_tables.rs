#![feature(rustc_private)]

//! Print dependencies from the package and target-specific Cargo tables.

use cargo_support::{dependencies, dependency_tables, toml_edit::Document};
use dylint_linting as _;
use toml_edit as _;

/// Demonstrate values borrowed from a parsed manifest.
fn main() {
    // Parse both package and target dependency tables before reading their entries.
    let source = r#"
[dependencies]
serde = "1"

[target.'cfg(unix)'.dependencies]
libc = "0.2"
"#;
    let document = Document::parse(source.to_owned()).expect("example manifest must be valid TOML");

    // Keep the document alive while its dependency names are borrowed.
    for table in dependency_tables(document.as_table()) {
        for dependency in dependencies(table) {
            let name = dependency.name;
            println!("{name}");
        }
    }
}
