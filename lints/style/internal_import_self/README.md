# internal_import_self

## What it does

Checks `use` items whose path starts with a module declared in the same module
as the `use`, without a `self::` prefix, such as `use helpers::Thing;` next to
`mod helpers`.

## Why is this bad?

A bare path such as `helpers::Thing` looks the same as an import from an
external crate named `helpers`. The `self::` prefix shows at the import site
that the path is local.

## Known problems

`use` items inside function bodies are not checked. For grouped imports such
as `use helpers::{One, nested::Two};`, the lint reports each name but cannot
suggest a fix. Paths that start with a module brought into scope by another `use` item
are not checked.

## Example

```rust
mod helpers {
    pub struct Thing;
}

use helpers::Thing;
```

## Use instead

```rust
mod helpers {
    pub struct Thing;
}

use self::helpers::Thing;
```
