# dependency_key_order

## What it does

Checks the nearest `Cargo.toml` above the crate root file for dependency
entries that are not sorted alphabetically. It checks `[dependencies]`,
`[dev-dependencies]`, `[build-dependencies]`, `[workspace.dependencies]`, and
their `[target.'...']` variants. The warning points at the out-of-order key.

## Why is this bad?

In an unsorted table, a reader must scan every line to find a dependency, and
duplicate or misplaced entries are easy to miss during updates. Sorted tables
also give new entries one correct position, which reduces merge conflicts.

## Known problems

Only adjacent entries are compared, and the comparison ignores ASCII case.
Blank lines and comment lines start a new block that is sorted on its own.
This lets you keep sorted groups, but a stray blank line also hides an
ordering mistake across it.

Dependency subtables such as `[dependencies.serde]` are not ordered.

## Example

```toml
[dependencies]
serde = "1.0.0"
anyhow = "1.0.0"
```

## Use instead

```toml
[dependencies]
anyhow = "1.0.0"
serde = "1.0.0"
```
