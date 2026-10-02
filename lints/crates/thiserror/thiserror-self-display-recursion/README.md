# thiserror-self-display-recursion

## What it does

Checks for a `Display` placeholder that formats `self`, such as `{self}`,
`{self:>10}`, or `{}` with a `self` argument, in the format string of an
`#[error(...)]` attribute on a `thiserror::Error` type.

## Why is this bad?

The `#[error(...)]` message becomes the type's `Display` implementation. A
`{self}` placeholder makes that implementation call itself, so formatting the
error overflows the stack.

## Known problems

Rustc's `unconditional_recursion` lint can report the same attribute.

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
