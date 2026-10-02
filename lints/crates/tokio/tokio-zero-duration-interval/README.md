# tokio-zero-duration-interval

## What it does

Checks for a zero `Duration` passed as the period to `tokio::time::interval`
or `tokio::time::interval_at`. A zero period is `Duration::ZERO`,
`Duration::new(0, 0)`, or `Duration::from_secs`, `from_millis`, `from_micros`,
or `from_nanos` called with the literal `0`.

## Why is this bad?

Tokio documents that both constructors panic when the period is zero. The
mistake is visible in the source but fails only at runtime.

## Known problems

The lint checks only the spellings listed above. A constant, variable,
`Duration::default()`, `Duration::from_secs_f64(0.0)`, or arithmetic
expression whose value is zero does not trigger the lint.

## Example

```rust
fn make_ticker() {
    let _ticker = tokio::time::interval(std::time::Duration::ZERO);
}
```

## Use instead

```rust
fn make_ticker() {
    let _ticker = tokio::time::interval(std::time::Duration::from_secs(1));
}
```
