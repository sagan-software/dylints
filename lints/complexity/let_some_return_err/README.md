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
- The lint gives help without a fix when the rewrite could fail to compile or
  change behavior. This covers a refutable pattern inside `Some(..)`, such as
  `Some(0)`, and an error value that uses a local variable or contains a
  closure, `return`, `?`, or `.await`. It also covers a type annotation on the
  `let`, a statement inside a macro, and an initializer that is a reference to
  an `Option`. A place initializer such as `holder.name` gets a fix only when
  its type is `Copy` and the pattern has no `ref` binding.
- The fix also requires an error value whose type does not come from the
  expected type, because `?` converts the closure's error through `From`. An
  error such as `"x".into()`, `Default::default()`, an unsuffixed number,
  `None`, or `Box::new(..)` coerced to `Box<dyn Error>` gets help without a fix.
  Literals with a fixed type, constants, struct literals, and calls whose
  declared return type has no type parameter keep the fix.
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
