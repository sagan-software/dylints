# manual_default_impl

## What it does

Checks hand-written `Default` implementations for local structs without type or
const parameters. The `default` method must return one struct expression, such as
`Self { ... }`, `Self(...)`, or `Self`, with every field set to its type's
default value. A default value is a call that resolves to `Default::default`,
such as `Default::default()`, `u8::default()`, or `<Vec<T>>::default()`, or
`false`, the integer `0`, or `None`.

The machine-applicable fix adds `#[derive(Default)]` to the struct and removes
the implementation. The lint offers it only when neither item comes from a macro
and the implementation has no attributes or doc comments.

## Why is this bad?

Adding, removing, or renaming a field requires an update to a hand-written
`Default` implementation. `#[derive(Default)]` stays in sync with the type
definition and produces the same value.

## Known problems

The lint misses fields set with constructors such as `Vec::new()` or
`String::new()`, bodies with statements, and enums, where a derive needs a
`#[default]` variant. The lint skips generic structs because the derive adds a
`Default` bound to every type parameter. Clippy's `derivable_impls` lint
reports many of the same implementations.

## Example

```rust
struct Config {
    retries: u8,
    labels: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            retries: Default::default(),
            labels: <Vec<String>>::default(),
        }
    }
}
```

## Use instead

```rust
#[derive(Default)]
struct Config {
    retries: u8,
    labels: Vec<String>,
}
```
