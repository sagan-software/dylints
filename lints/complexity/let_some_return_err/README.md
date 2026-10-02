# let_some_return_err

## What it does

Checks for `let Some(..) = option else { return Err(error) };` inside a function
or closure that returns `Result`, when the `else` block holds only that
`return` and `error` already has the function's error type.

## Why is this bad?

The `let`-`else` spends three lines on a standard conversion. `ok_or_else` turns
the `Option` into a `Result`, and `?` returns the error, in one line.

## Known problems

- The machine-applicable fix always uses `ok_or_else` and wraps the initializer
  in parentheses, for example `(find_user(id)).ok_or_else(|| error)?`.
- The lint does not check that the pattern inside `Some(..)` is irrefutable.
  `let Some(0) = count else { return Err(..) };` triggers, and the suggested
  `let 0 = ...?;` does not compile.
- An `else` block with any other statement, a `return Err(..)` whose error type
  differs from the function's error type, and an `Err` value built through
  `From` are ignored.

## Example

```rust
fn user_name(id: u64) -> Result<&'static str, Error> {
    let Some(user) = find_user(id) else {
        return Err(Error::Missing);
    };
    Ok(user)
}
```

## Use instead

```rust
fn user_name(id: u64) -> Result<&'static str, Error> {
    let user = find_user(id).ok_or(Error::Missing)?;
    Ok(user)
}
```
