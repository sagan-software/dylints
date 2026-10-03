# manual_debug_impl

## What it does

Checks hand-written `Debug` implementations for local structs without type or
const parameters. Their `fmt` body must be one `debug_struct` or `debug_tuple`
chain. The chain must use the struct's name, show every field in declaration
order as `&self.field` under the field's own name, and end with `.finish()`.
That chain prints the same text as `#[derive(Debug)]`.

When a struct has fields, its builder must match its field syntax. Named structs
must use `debug_struct` with field labels. Tuple structs must use `debug_tuple`
with positional fields. A different builder prints different text, so the lint
skips it.

The machine-applicable fix adds `#[derive(Debug)]` to the struct and removes
the implementation. The lint offers it only when neither item comes from a macro
and the implementation has no attributes or doc comments.

## Why is this bad?

Adding, removing, or renaming a field requires an update to a hand-written
`Debug` implementation. `#[derive(Debug)]` stays in sync with the type
definition and produces the same output.

## Known problems

The lint misses implementations that write the output with `write!` or
`f.write_str`, enums, and generic structs, where the derive adds a `Debug`
bound to every type parameter. Implementations that leave out, reorder, rename,
or redact a field, or that end with `.finish_non_exhaustive()`, print different
text, so the lint skips them.

## Example

```rust
use std::fmt;

struct User {
    id: u64,
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User").field("id", &self.id).finish()
    }
}
```

## Use instead

```rust
#[derive(Debug)]
struct User {
    id: u64,
}
```
