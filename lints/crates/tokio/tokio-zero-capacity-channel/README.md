# tokio-zero-capacity-channel

## What it does

Checks for the integer literal `0` passed as the capacity to
`tokio::sync::mpsc::channel` or `tokio::sync::broadcast::channel`.

## Why is this bad?

Tokio documents that both constructors panic when the capacity is zero. The
mistake is visible in the source but fails only at runtime.

## Known problems

The lint checks only the literal `0`. A constant, variable, or arithmetic
expression whose value is zero does not trigger the lint.

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
