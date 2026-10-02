# thiserror-tuple-format-positional-ambiguity

## What it does

Checks for an `#[error(...)]` attribute on a tuple struct or tuple variant
that derives `thiserror::Error` when the format string has a numeric
placeholder such as `{0}` and the first extra format argument is positional.

## Why is this bad?

In a tuple type, `{0}` can mean the first tuple field or the first extra
format argument. Thiserror 2 rejects this mix, so code written for
thiserror 1 stops compiling after the upgrade. Naming the extra argument
removes the ambiguity in both versions.

## Known problems

With thiserror 2, an active item with this pattern already fails to compile,
so the lint is most useful during a migration from thiserror 1.

The lint reads the attributes written directly above the item. A comment or
doc comment between the derive and the `#[error(...)]` attribute hides the
derive, and the lint does not warn. It checks only the first extra argument,
so a named first argument followed by a positional one does not trigger it.
It gives help text but no automatic fix.

The lint recognizes the derive only as `thiserror::Error` or through a
`use thiserror::Error` or `use thiserror::Error as Name` import. A glob import
such as `use thiserror::*` hides the derive from the lint.

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
