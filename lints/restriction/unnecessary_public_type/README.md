# unnecessary_public_type

## What it does

Checks for `pub` structs, enums, and type aliases whose name appears nowhere in
the crate's source except their own declaration. It runs only in crates that
are not published: the nearest `Cargo.toml` sets `publish = false` or
`publish = []`, directly or through `publish.workspace = true` and a workspace
with `publish = false`.

## Why is this bad?

A `pub` type looks like part of an interface that other code depends on.
Readers keep it, document it, and update it on refactors, while nothing in the
crate uses it.

## Known problems

The lint counts names in the text of Rust files: the crate's own files under the
crate root's directory, and every `.rs` file under the nearest workspace root,
or under the package directory when there is no workspace. It skips `target`
directories and hidden entries. A mention in a comment, a string, an unrelated
item with the same name, or another workspace crate counts as a use, so the lint
stays silent. A crate outside the workspace that uses the type is not seen.

It skips types that have a doc comment or an `allow`, `expect`, `cfg`,
`cfg_attr`, `test`, `repr`, `no_mangle`, `export_name`, `used`, `link_name`, or
`wasm_bindgen` attribute. It does not check `pub(crate)` types, and it does not
recognize the inline form `publish = { workspace = true }`.

## Example

```rust
// In a crate with `publish = false`
pub struct InternalPayload {
    value: String,
}

pub fn run() {}
```

## Use instead

```rust
// In a crate with `publish = false`
pub fn run() {}
```
