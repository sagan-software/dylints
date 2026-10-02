# path_attribute_outside_root

## What it does

Checks for `#[path = "..."]` attributes, including those produced by
`cfg_attr`, in any file not named `main.rs`, `lib.rs`, or `build.rs`.

## Why is this bad?

A `path` attribute overrides where Rust looks for a module file. Readers who
follow the standard layout look for `src/parser/parser_support.rs` and do not
find the module. Outside a crate root, the override is hard to spot.

## Known problems

The lint checks only the file name. It allows a nested file named `main.rs` or
`lib.rs` that is not a crate root. It warns in crate roots with other names,
such as `src/bin/tool.rs`, `examples/demo.rs`, and `tests/api.rs`. It skips
files that the compiler cannot map to a local path.

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
