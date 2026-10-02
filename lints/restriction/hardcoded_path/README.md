# hardcoded_path

## What it does

Checks for string literals that hold a machine-specific filesystem path: a Unix
absolute path such as `/home/alice/cache`, a home path such as `~/cache` or
`~alice/cache`, a Windows drive path such as `C:\Users`, or a UNC path such as
`\\server\share`.

## Why is this bad?

The path exists only on the machine where it was written. Builds, tests, and
tools that use it fail on other machines, in CI, and in containers.

## Known problems

The lint reads only the literal's text, not how the value is used. It warns on
any string that starts with `/`, including URL routes such as `"/api/users"`
and Unix sockets that are meant to be fixed. It skips the `//`, `//!`, and `///`
comment markers and strings made only of slashes.

It does not flag relative paths such as `assets/icon.png` or `../src/lib.rs`,
URLs such as `https://example.com/assets`, byte strings, or paths built at
runtime.

## Example

```rust
fn cache_dir() -> &'static str {
    "/home/alice/.cache/example"
}
```

## Use instead

```rust
fn cache_dir() -> &'static str {
    "target/example-cache"
}
```
