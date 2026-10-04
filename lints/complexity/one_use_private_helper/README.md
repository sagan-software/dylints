# one_use_private_helper

## What it does

Checks for a free function visible only in its own module. Its body must be one
expression without statements, and its only crate reference must be one direct
call from another function, closure, or constant. The lint tokenizes each source
file once and reuses its Rust identifier counts for other candidate helpers.
Identifier counts use the same Unicode NFC normalization as rustc.

## Why is this bad?

The reader must jump to another item to see one expression that runs in one
place. Writing the expression at the call site keeps the behavior where callers
use it. A helper still earns its place when its name states a rule, as covered
by the exclusions below.

## Known problems

- Code removed by `cfg`, such as a `#[cfg(test)]` module in a library build, does
  not enter the compiled crate. To preserve helpers called by such code, the
  lint requires the helper name to appear exactly twice as a Rust identifier
  in its source file. Comments and literal contents are excluded. A matching
  identifier in an unrelated item can still suppress the warning.
- The lint ignores a helper whose only call comes from a macro expansion.
- Attributes other than doc comments, `#[inline]`, `#[must_use]`, and lint
  level attributes such as `#[allow]` prevent the lint. So do type or const
  parameters and an `async` or `unsafe` prefix. A `pub(crate)` function in the
  crate root has the same visibility as a private one and can trigger.
- A body whose expression is an `if`, `match`, loop, closure, or block is
  ignored.
- The lint ignores names containing a word such as `validate`, `check`, `ensure`,
  `assert`, `test`, `fixture`, `build`, `make`, `hook`, `rule`, `policy`, or
  `invariant`, or starting with `can`, `should`, `is`, `has`, or `must`.

## Example

```rust
fn adjusted_total(amount: u64) -> u64 {
    amount.saturating_add(1)
}

fn checkout(amount: u64) -> u64 {
    adjusted_total(amount)
}
```

## Use instead

```rust
fn checkout(amount: u64) -> u64 {
    amount.saturating_add(1)
}
```
