# bevy-time-elapsed-secs-cast-f64

## What it does

Checks for `Time::elapsed_secs()` cast to `f64` with `as f64`.

## Why is this bad?

`elapsed_secs` returns an `f32`, which loses precision as elapsed time grows. After about five
hours, consecutive `f32` values are about 2 ms apart. The cast to `f64` cannot recover the lost
precision. `elapsed_secs_f64` computes the value as an `f64` from the start.

## Known problems

The lint only checks `as f64` casts. It does not report `f64::from(time.elapsed_secs())` or
`.into()`.

## Example

```rust
fn wave(time: Res<Time>) -> f64 {
    (time.elapsed_secs() as f64).sin()
}
```

## Use instead

```rust
fn wave(time: Res<Time>) -> f64 {
    time.elapsed_secs_f64().sin()
}
```
