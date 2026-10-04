# insta-binary-snapshot-missing-extension

## What it does

Checks for `insta::assert_binary_snapshot!` calls whose name is a string
literal with no `.`, such as `"response"`.

## Why is this bad?

Insta splits a binary snapshot name at the first `.` to get the file
extension. A name with no `.` makes the macro panic when the test runs, so the
macro never records or compares the snapshot.

## Known problems

The lint follows string literals for at most eight steps through simple immutable
local bindings, same-crate constants, and same-crate inherent associated
constants. Trait-associated constants remain unknown because an implementation
may override a trait default. Mutable, destructured, or uninitialized bindings;
external constants, statics, and associated constants; calls; and other
computed expressions remain unknown.

## Example

```rust
fn snapshot_response(bytes: Vec<u8>) {
    insta::assert_binary_snapshot!("response", bytes);
}
```

## Use instead

Add an extension to the name. A name that is only an extension, such as
`".bin"`, uses the default snapshot name.

```rust
fn snapshot_response(bytes: Vec<u8>) {
    insta::assert_binary_snapshot!("response.bin", bytes);
}
```
