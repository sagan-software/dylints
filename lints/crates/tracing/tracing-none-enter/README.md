# tracing-none-enter

## What it does

Checks for `enter` called directly on `tracing::Span::none()`.

## Why is this bad?

`Span::none()` returns a disabled span. Entering it records nothing and does
not change the current span, so the guard does nothing. The guard also suggests
that the work runs inside a span when it does not.

## Known problems

The lint only checks a direct `Span::none().enter()` chain. It does not follow
the span through a local binding, a field, or another function.

## Example

```rust
# fn work() {}
fn handle() {
    let _ = tracing::Span::none().enter();
    work();
}
```

## Use instead

Remove the guard. If the work needs a span, enter an enabled span instead:

```rust
# fn work() {}
fn handle() {
    work();
}
```
