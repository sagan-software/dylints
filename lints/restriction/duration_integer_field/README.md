# duration_integer_field

## What it does

Checks for named fields whose type is a primitive integer and whose name contains
a time-unit word. The words cover nanoseconds through weeks, such as `ns`, `ms`,
`millis`, `sec`, `seconds`, `mins`, `hr`, `hours`, `days`, and `weeks`, matched
between underscores and with case. The singular words `second`, `minute`,
`hour`, `day`, and `week`, which usually name a calendar component, and `min`,
which usually means minimum, do not count.

## Why is this bad?

The unit lives only in the field name, so code that passes milliseconds where
seconds are expected still compiles. `std::time::Duration` stores one value and
converts units explicitly.

## Known problems

The lint checks only primitive integer types, after resolving type aliases and
`use` renames. It does not flag `Option<u64>` or integer newtypes such as
`struct BusinessDays(u16)`.

It warns on a raw integer count that uses a unit word but is not an elapsed
time, such as `business_days: u16`. It does not flag duration names without a
unit word, such as `timeout` or `ttl`.

## Example

```rust
struct RetryConfig {
    retry_delay_seconds: u64,
    timeout_ms: usize,
}
```

## Use instead

```rust
use std::time::Duration;

struct RetryConfig {
    retry_delay: Duration,
    timeout: Duration,
}
```
