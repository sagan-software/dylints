# manual_passthrough_inspect

## What it does

Checks for a block that binds an `Option` or `Result`, observes it with
`if let Some(..)`, `if let Ok(..)`, or `if let Err(..) = &binding`, and then
returns the binding unchanged.

## Why is this bad?

The binding exists only to look at the value before returning it. The three
statements hide that the value passes through unchanged. `inspect` and
`inspect_err` state the side effect and the unchanged return in one chain.

## Known problems

- The block must contain exactly the `let`, the `if let`, and the returned
  binding. A block with any other statement is ignored.
- The observation must borrow the binding as `&name` and hold at most one `;`.
  The lint skips it when its source text contains `?`, `.await`, `break`,
  `continue`, or `return`. The check is a text match, so a name such as
  `returned` also prevents the lint.
- With `inspect`, temporaries in the initializer can drop at a different time.
  The lint emits help without an automatic fix.

## Example

```rust
fn load(ok: bool) -> Result<i32, String> {
    let result = operation(ok);
    if let Err(error) = &result {
        println!("{error}");
    }
    result
}
```

## Use instead

```rust
fn load(ok: bool) -> Result<i32, String> {
    operation(ok).inspect_err(|error| println!("{error}"))
}
```
