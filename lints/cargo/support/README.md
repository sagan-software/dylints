# cargo-support

`cargo-support` provides shared Cargo manifest helpers for the Cargo lint family.
It parses manifests with `toml_edit`, keeps dependency entries tied to their
source document, and finds package and workspace dependency tables.

The dependency iterators borrow the parsed document and preserve Cargo table
order:

```rust
#![feature(rustc_private)]

use cargo_support::{dependencies, dependency_tables, toml_edit::Document};

let source = "[dependencies]\nserde = \"1\"\n";
let document = Document::parse(source.to_owned()).expect("valid manifest");

for table in dependency_tables(document.as_table()) {
    for dependency in dependencies(table) {
        println!("{}", dependency.name);
    }
}
```
