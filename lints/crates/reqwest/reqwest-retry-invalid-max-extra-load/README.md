# reqwest-retry-invalid-max-extra-load

## What it does

Checks for a literal argument to `reqwest::retry::Builder::max_extra_load` that
is below `0.0` or above `1000.0`.

## Why is this bad?

`max_extra_load` panics when its argument is outside `0.0..=1000.0`. The
program crashes when it builds the retry policy, even though the bad value is
written in the source.

## Known problems

The lint only checks numeric literals, with or without a leading minus sign. It
misses a named constant or a computed value.

## Example

```rust
fn retry_policy() -> reqwest::retry::Builder {
    reqwest::retry::for_host("example.com").max_extra_load(-0.1)
}
```

## Use instead

```rust
fn retry_policy() -> reqwest::retry::Builder {
    reqwest::retry::for_host("example.com").max_extra_load(0.2)
}
```
