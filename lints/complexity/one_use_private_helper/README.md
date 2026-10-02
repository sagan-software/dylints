# one_use_private_helper

## What it does

Checks for a free function visible only in its own module, whose body is one
expression without statements, and whose only reference in the crate is one
direct call from another function, closure, or constant.

## Why is this bad?

The reader must jump to another item to see one expression that runs in one
place. Writing the expression at the call site keeps the behavior where it is
used. A helper still earns its place when its name states a rule, as covered by
the exclusions below.

## Known problems

- Code removed by `cfg`, such as a `#[cfg(test)]` module in a library build, is
  not in the compiled crate. To keep helpers that such code calls, the lint also
  requires the name to appear exactly twice in the helper's source file, so a
  mention in a comment, a string, or an unrelated item suppresses it.
- A helper whose only call comes from a macro expansion is ignored.
- Attributes other than doc comments, `#[inline]`, `#[must_use]`, and lint
  level attributes such as `#[allow]` prevent the lint. So do type or const
  parameters and an `async` or `unsafe` prefix. A `pub(crate)` function in the
  crate root has the same visibility as a private one and can trigger.
- A body whose expression is an `if`, `match`, loop, closure, or block is
  ignored.
- Names containing a word such as `validate`, `check`, `ensure`, `assert`,
  `test`, `fixture`, `build`, `make`, `hook`, `rule`, `policy`, or `invariant`,
  or starting with `can`, `should`, `is`, `has`, or `must`, are ignored.
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
