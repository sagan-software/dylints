# tracing-current-span-enter

## What it does

Checks for `enter` called directly on `tracing::Span::current()`.

## Why is this bad?

`Span::current()` returns the current span. Entering it again does not change
which span is current, so the guard does nothing. The guard also suggests a
scope boundary that does not exist.

## Known problems

The lint only checks a direct `Span::current().enter()` chain. It does not
follow the span through a local binding, a field, or another function.

## Example

```rust
# fn work() {}
fn handle() {
    let _ = tracing::Span::current().enter();
    work();
}
```

## Use instead

Remove the guard:

```rust
# fn work() {}
fn handle() {
    work();
}
```
