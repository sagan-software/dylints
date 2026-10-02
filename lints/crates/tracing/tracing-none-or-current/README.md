# tracing-none-or-current

## What it does

Checks for `or_current` called directly on `tracing::Span::none()`.

## Why is this bad?

`Span::or_current` returns the current span when its receiver is disabled.
`Span::none()` is always disabled, so the expression always returns
`Span::current()`. The extra call hides that fixed result.

## Known problems

The lint only checks a direct `Span::none().or_current()` chain. It does not
follow the span through a local binding, a field, or a helper function.

## Example

```rust
fn current_span() -> tracing::Span {
    tracing::Span::none().or_current()
}
```

## Use instead

Call `Span::current()`:

```rust
fn current_span() -> tracing::Span {
    tracing::Span::current()
}
```
