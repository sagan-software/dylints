# manual_debug_impl

## What it does

Checks hand-written `Debug` implementations for local structs without type or
const parameters whose `fmt` body is one `debug_struct` or `debug_tuple` chain
that uses the struct's name, shows every field in declaration order as
`&self.field` under the field's own name, and ends with `.finish()`. That chain
prints the same text as `#[derive(Debug)]`.

The machine-applicable fix adds `#[derive(Debug)]` to the struct and removes
the implementation. It is offered only when neither item comes from a macro
and the implementation has no attributes or doc comments.

## Why is this bad?

A hand-written `Debug` implementation must be updated whenever a field is added,
removed, or renamed. `#[derive(Debug)]` stays in sync with the type definition
and produces the same output.

## Known problems

The lint misses implementations that write the output with `write!` or
`f.write_str`, enums, and generic structs, where the derive adds a `Debug`
bound to every type parameter. Implementations that leave out, reorder, rename,
or redact a field, or that end with `.finish_non_exhaustive()`, print different
text and are not reported.

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
