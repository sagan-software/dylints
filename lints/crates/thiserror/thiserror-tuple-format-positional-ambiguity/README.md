# thiserror-tuple-format-positional-ambiguity

## What it does

Checks for an `#[error(...)]` attribute on a tuple struct or tuple variant
that derives `thiserror::Error`. It reports a numeric placeholder such as `{0}`
when an extra format argument has no name.

## Why is this bad?

In a tuple type, `{0}` can mean the first tuple field or the first extra
format argument. Thiserror 2 rejects this mix, so code written for
thiserror 1 stops compiling after the upgrade. Naming the extra argument
removes the ambiguity in both versions.

## Known problems

With thiserror 2, an active item with this pattern already fails to compile
with thiserror's own error. The lint is most useful on thiserror 1 code before
a migration. The UI fixture uses thiserror 1 to preserve that pre-migration
behavior. It gives help text but no automatic fix.

## Example

```rust
fn expected() -> &'static str {
    "expected"
}

#[derive(thiserror::Error, Debug)]
#[error("bad {0} {}", expected())]
pub struct Error(String);
```

## Use instead

```rust
fn expected() -> &'static str {
    "expected"
}

#[derive(thiserror::Error, Debug)]
#[error("bad {0} {expected}", expected = expected())]
pub struct Error(String);
```
