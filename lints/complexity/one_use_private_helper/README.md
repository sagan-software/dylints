# one_use_private_helper

## What it does

Checks for a private free function written as `fn name(..) { expr }`, with no
attributes, generics, or statements, that the same source file calls exactly
once as `name(..)`.

## Why is this bad?

The reader must jump to another item to see one expression that runs in one
place. Writing the expression at the call site keeps the behavior where it is
used. A helper still earns its place when its name states a rule, as covered by
the exclusions below.

## Known problems

- The lint counts calls by searching the file's source text. A `name(` in a
  comment or string counts as a call, and calls from other files, through
  qualified paths such as `self::name(..)`, or from macros are not counted.
- A helper that is called once and also passed as a value, such as
  `.map(name)`, still triggers.
- Any attribute, including a `///` doc comment, prevents the lint. So does a
  `const`, `async`, `unsafe`, or `pub(crate)` prefix.
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
