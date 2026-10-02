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
- The observation must borrow the binding as `&name`, have no `else`, and hold
  one action. The lint skips it when it contains `?`, `.await`, `break`,
  `continue`, or `return` outside a closure.
- The variant payload pattern must accept every payload. Partial patterns such
  as `Some(0)` are ignored because `inspect` would observe all `Some` values.
- With `inspect`, temporaries in the initial value can drop at a different time.
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
