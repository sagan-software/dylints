# path_attribute_outside_root

## What it does

Checks for `#[path = "..."]` attributes, including those produced by
`cfg_attr`, in any file other than the crate root file that rustc compiles,
such as `src/lib.rs`, `src/main.rs`, `build.rs`, or `src/bin/tool.rs`.

## Why is this bad?

A `path` attribute overrides where Rust looks for a module file. Readers who
follow the standard layout look for `src/parser/parser_support.rs` and do not
find the module. Outside a crate root, the override is hard to spot.

## Known problems

The lint compares the attribute's file with the crate root file. It allows every
crate root, including `examples/demo.rs` and `tests/api.rs`, and it warns in a
nested module file even when its name is `lib.rs` or `main.rs`. It skips
files that the compiler cannot map to a real path.

## Example

```rust
// src/parser.rs
#[path = "shared/parser_support.rs"]
mod parser_support;
```

## Use instead

```rust
// src/parser.rs
mod parser_support;
```

Place `parser_support` in `src/parser/parser_support.rs`, or declare the path
override from `main.rs`, `lib.rs`, or `build.rs` when it is necessary.
