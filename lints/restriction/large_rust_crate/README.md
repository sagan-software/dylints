# large_rust_crate

## What it does

Checks the combined line count of the Rust source files compiled into a crate.
It warns when the crate has more than 20,000 non-test lines or more than 40,000
total lines. Blank and comment lines count, and test lines count only toward the
total limit.

Set the limits in the workspace's `dylint.toml`:

```toml
[large_rust_crate]
non_test_line_limit = 20000
total_line_limit = 40000
```

## Why is this bad?

A crate is the unit of compilation, so a change to any file recompiles the whole
crate. A large crate also tends to mix responsibilities that could have separate
owners, dependencies, and public APIs.

## Known problems

The lint counts readable `.rs` files compiled into the crate that sit under the
package directory, the nearest directory above the crate root with a
`Cargo.toml`. It skips files from inactive `cfg` modules and files outside the
package directory.

A line is a test line only when it belongs to an item marked with a test
attribute, such as `#[test]`, or a `#[cfg(...)]` that requires `test`. If `syn`
cannot parse a file, every line counts as non-test. When code exceeds both limits, the lint reports only the total-line violation.

## Example

The abbreviated snippets require their separate module files and omitted implementation.

```rust,ignore
// src/lib.rs of a crate whose modules total 25,000 non-test lines
pub mod billing;
pub mod reporting;
pub mod storage;
```

## Use instead

Move independent modules into their own workspace crates and re-export them:

```rust,ignore
// src/lib.rs
pub use billing;
pub use reporting;
pub use storage;
```
