# string-error-result

## What it does

Checks function, method, and `async fn` return types for `Result<T, String>`,
including required trait method signatures, aliases of `Result` or `String`,
and a `Result` nested in the type arguments of the return type.

## Why is this bad?

A `String` error only carries text. Callers cannot match on the failure kind
or reach the original error through `source()`, and they must parse the
message to react to a specific failure.

## Known problems

The lint also warns at program edges where a text error is enough, such as a
small command-line tool.

It does not inspect closures or return types written as an associated type
such as `<Self as Trait>::Out`. It also ignores error types other than
`String`, including `&str` and `Box<dyn Error>`.

## Example

```rust
fn parse_port(raw: &str) -> Result<u16, String> {
    raw.parse::<u16>().map_err(|error| error.to_string())
}
```

## Use instead

```rust
fn parse_port(raw: &str) -> Result<u16, std::num::ParseIntError> {
    raw.parse::<u16>()
}
```
