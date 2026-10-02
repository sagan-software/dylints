# ad_hoc_default

## What it does

Checks inherent associated functions named `new`, `empty`, `blank`, or
`default_config` that take no arguments and return the type of the `impl`
block, when that type does not implement `Default`.

## Why is this bad?

Generic code that asks for `T: Default` cannot call a custom zero-argument
constructor, and it does not work with `#[derive(Default)]` on containing
types, `..Default::default()`, or `unwrap_or_default()`.

## Known problems

The lint does not read the function body. It warns on constructors that
allocate resources, have side effects, or set up an invariant that a `Default`
value must not have. For a public `new`, Clippy's `new_without_default` lint
reports the same missing implementation.

## Example

```rust
struct Config {
    retries: u8,
}

impl Config {
    fn new() -> Self {
        Self { retries: 3 }
    }
}
```

## Use instead

```rust
struct Config {
    retries: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self { retries: 3 }
    }
}
```
