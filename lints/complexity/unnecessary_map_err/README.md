# unnecessary_map_err

## What it does

Checks for `Result::map_err` calls whose mapper only converts the error with
`From` or `Into`. The resulting error type must equal the enclosing function's
`Result` error type, and that type must implement `From` for the original error.
Accepted mappers are `Into::into`, `From::from`, `Error::from`,
`|e| Error::from(e)`, and `|e| e.into()`.

## Why is this bad?

The `?` operator already converts the error with `From`. The extra `map_err`
adds a call that a reader must check, only to find that it changes nothing that
`?` would not change.

## Known problems

- The lint offers a machine-applicable fix only when `?` directly follows the
  call. In a tail expression such as `raw.parse::<u16>().map_err(Error::from)`,
  or inside `Ok(..)` without `?`, the lint gives help without a fix. Rewrite
  these calls by hand as `Ok(raw.parse::<u16>()?)`.
- A call inside a macro body, or whose receiver comes from a macro call, gets
  help without a fix.
- `?` cannot infer the target of `map_err(From::from)`, `map_err(Into::into)`,
  or `map_err(|e| e.into())`, so these mappers trigger only without `?`.
- The lint skips calls inside closures and `async` function bodies.
- The lint ignores mappers with any other body, such as a closure that adds
  context or runs a statement before the conversion.

## Example

```rust
fn parse_port(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(Error::from)?;
    Ok(port)
}
```

## Use instead

```rust
fn parse_port(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>()?;
    Ok(port)
}
```
