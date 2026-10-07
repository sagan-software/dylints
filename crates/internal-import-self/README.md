# internal-import-self

## What it does

Checks `use` items whose path starts with a module declared in the same module
as the `use`, without a `self::` prefix, such as `use helpers::Thing;` next to
`mod helpers`.

## Why is this bad?

A bare path such as `helpers::Thing` looks the same as an import from an
external crate named `helpers`. The `self::` prefix shows at the import site
that the path is local.

## Known problems

The lint does not check `use` items inside function bodies. For grouped imports such
as `use helpers::{One, nested::Two};`, the lint reports each name but cannot
suggest a fix. The lint does not check paths that start with a module brought into scope by another `use` item.

## Example

```rust
mod helpers {
    pub struct Thing;
}

use helpers::Thing;
# fn main() {}
```

## Use instead

```rust
mod helpers {
    pub struct Thing;
}

use self::helpers::Thing;
# fn main() {}
```
