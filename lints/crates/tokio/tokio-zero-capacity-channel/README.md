# tokio-zero-capacity-channel

## What it does

Checks `tokio::sync::mpsc::channel` and `tokio::sync::broadcast::channel`
calls whose capacities are statically known to be zero. It follows local
non-trait constants and same-type checked unsigned `+`, `-`, `*`, `/`, and `%`
expressions through 16 nested steps.

## Why is this bad?

Tokio documents that both constructors panic when the capacity is zero. The
mistake is visible in the source but fails only at runtime.

## Known problems

The evaluator reports only values it can prove are zero. Runtime locals and
function calls remain unknown. Casts, statics, trait or external constants,
unsupported operators, and arithmetic that overflows, underflows, divides by
zero, or takes remainder by zero also remain unknown.

## Example

```rust
fn make_queue() {
    let (_tx, _rx) = tokio::sync::mpsc::channel::<u8>(0);
}
```

## Use instead

Use the smallest positive capacity that holds the expected burst.

```rust
fn make_queue() {
    let (_tx, _rx) = tokio::sync::mpsc::channel::<u8>(16);
}
```
