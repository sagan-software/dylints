# insta-binary-snapshot-missing-extension

## What it does

Checks for `insta::assert_binary_snapshot!` calls whose name is a string
literal with no `.`, such as `"response"`.

## Why is this bad?

Insta splits a binary snapshot name at the first `.` to get the file
extension. A name with no `.` makes the macro panic when the test runs, so the
macro never records or compares the snapshot.

## Known problems

The lint checks a string literal passed directly as the name, or
a `const` defined in the same crate and initialized with a string literal. It does not check names held in variables or built at runtime.

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
