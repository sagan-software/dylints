# thiserror-self-display-recursion

## What it does

Checks for a `{self}` or `{self:}` placeholder in the format string of an
`#[error(...)]` attribute on a `thiserror::Error` type.

## Why is this bad?

The `#[error(...)]` message becomes the type's `Display` implementation. A
`{self}` placeholder makes that implementation call itself, so formatting the
error overflows the stack.

## Known problems

The lint matches the format string text. It warns on the escaped text
`{{self}}`, which prints a literal `{self}` and does not recurse. It does not
warn on `{self}` with a format specification such as `{self:>10}`, or on
`self` passed as a positional argument.

Rustc's `unconditional_recursion` lint can report the same attribute.

The lint recognizes the derive only as `thiserror::Error` or through a
`use thiserror::Error` or `use thiserror::Error as Name` import. A glob import
such as `use thiserror::*` hides the derive from the lint.

## Example

```rust
#[derive(thiserror::Error, Debug)]
#[error("invalid error: {self}")]
pub struct Error;
```

## Use instead

```rust
#[derive(thiserror::Error, Debug)]
#[error("invalid error")]
pub struct Error;
```
