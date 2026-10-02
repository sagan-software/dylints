# unnecessary_map_err

## What it does

Checks for `Result::map_err` calls whose mapper only converts the error with
`From` or `Into`, when the resulting error type equals the enclosing function's
`Result` error type. Accepted mappers are `Into::into`, `From::from`,
`Error::from`, `|e| Error::from(e)`, and `|e| e.into()`.

## Why is this bad?

The `?` operator already converts the error with `From`. The extra `map_err`
adds a call that a reader must check, only to find that it changes nothing that
`?` would not change.

## Known problems

- The lint does not check that a `?` follows the call. In a tail expression such
  as `raw.parse::<u16>().map_err(Error::from)`, the machine-applicable fix only
  deletes `.map_err(..)`, and the result does not compile until you add
  `Ok(..?)` by hand. The same applies inside `Ok(..)` when the function returns
  a nested `Result`.
- A custom `Into` impl without a matching `From` impl can trigger even though
  `?` would not compile.
- Closures are not checked. Mappers with any other body, such as a closure that
  adds context, are ignored.

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
