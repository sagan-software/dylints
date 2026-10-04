# bevy-time-elapsed-secs-cast-f64

## What it does

Checks for `Time::elapsed_secs()` cast to `f64` with `as f64`.

## Why is this bad?

`elapsed_secs` returns an `f32`, which loses precision as elapsed time grows. After about five
hours, consecutive `f32` values are about 2 ms apart. The cast to `f64` cannot recover the lost
precision. `elapsed_secs_f64` computes the value as an `f64` from the start.

## Known problems

The lint recognizes `as f64`, `f64::from`, and `.into()` around a direct `Time::elapsed_secs()`
call. It does not resolve a function-pointer call to `f64::from`. Macro-expanded conversions get a
diagnostic, but no source rewrite.

## Example

```rust
# use bevy_time::Time;
fn wave(time: &Time) -> f64 {
    (time.elapsed_secs() as f64).sin()
}
```

## Use instead

```rust
# use bevy_time::Time;
fn wave(time: &Time) -> f64 {
    time.elapsed_secs_f64().sin()
}
```
