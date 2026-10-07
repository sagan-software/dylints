# tokio-zero-duration-interval

## What it does

Checks for a zero standard-library `Duration` passed to
`tokio::time::interval` or `tokio::time::interval_at`. It recognizes
`Duration::ZERO`, `Default::default()`, and local non-trait constants. It
evaluates the standard constructors `Duration::new`, `Duration::from_secs`,
`Duration::from_millis`, `Duration::from_micros`, `Duration::from_nanos`,
`Duration::from_secs_f32`, and `Duration::from_secs_f64`. It also evaluates
bounded `Duration` addition and subtraction, multiplication or division by a
`u32` scalar, and checked `u32` or `u64` arithmetic used by integer
constructors. Float constructors use `Duration`'s own conversion, including
its exact nanosecond rounding.

## Why is this bad?

Tokio documents that both constructors panic when the period is zero. The
mistake is visible in the source but fails only at runtime.

## Known problems

The lint leaves runtime variables and parameters, function calls, statics,
external constants, trait constants, and unsupported expressions unknown. It
also leaves arithmetic unknown when checked evaluation fails, operand types do
not match, or evaluation reaches the 16-step limit. Float constructors whose
standard conversion returns an error remain unknown. The lint recognizes the
resolved standard `Duration` and Tokio interval functions, not user-defined
items with the same names.

## Example

```rust
fn make_ticker() {
    let _ticker = tokio::time::interval(std::time::Duration::from_secs_f64(1e-20));
}
```

## Use instead

```rust
fn make_ticker() {
    let _ticker = tokio::time::interval(std::time::Duration::from_nanos(1));
}
```
