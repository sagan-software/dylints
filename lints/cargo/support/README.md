# cargo-support

`cargo-support` provides shared Cargo manifest helpers for the Cargo lint family.
It parses manifests with `toml_edit`, keeps dependency entries tied to their
source document, and finds package and workspace dependency tables.

The dependency iterators borrow the parsed document. Package tables precede
target tables. Within each package or target, table kinds follow
`dependencies`, `dev-dependencies`, `dev_dependencies`, `build-dependencies`,
then `build_dependencies`. Targets and entries within each table keep
manifest order:

```rust
#![feature(rustc_private)]

use cargo_support::{dependencies, dependency_tables, toml_edit::Document};

let source = "[dependencies]\nserde = \"1\"\n";
let document = Document::parse(source.to_owned()).expect("valid manifest");

for table in dependency_tables(document.as_table()) {
    for dependency in dependencies(table) {
        let name = dependency.name;
        println!("{name}");
    }
}
```
