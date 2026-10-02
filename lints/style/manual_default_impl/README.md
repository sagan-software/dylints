# manual_default_impl

## What it does

Checks hand-written `Default` implementations whose `default` method returns
`Self { ... }` with every field set to `Default::default()` or
`<Type>::default()`.

## Why is this bad?

A hand-written `Default` implementation must be updated whenever a field is
added, removed, or renamed. `#[derive(Default)]` stays in sync with the type
definition and produces the same value.

## Known problems

The lint reads source text, not types. It misses fields written as
`Type::default()` without angle brackets, `Vec::new()`, `0`, `false`, or
`None`, and it misses tuple structs and enums. It skips any implementation
whose source contains a string literal, `if `, `match `, `todo`, or `panic`,
or a comment that mentions `custom` or `invariant`.

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
