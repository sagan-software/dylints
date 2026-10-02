# string_error_result

## What it does

Checks function and method return types for `Result<T, String>`, including
aliases of `Result` or `String` and a `Result` nested in the type arguments
of the return type.

## Why is this bad?

A `String` error only carries text. Callers cannot match on the failure kind
or reach the original error through `source()`, and they must parse the
message to react to a specific failure.

## Known problems

The lint also warns at program edges where a text error is enough, such as a
small command-line tool.

It misses `async fn` return types, required trait methods without a body,
closures, and other text error types such as `&str` or `Box<dyn Error>`.

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
