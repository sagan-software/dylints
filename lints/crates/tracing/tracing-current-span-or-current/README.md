# tracing-current-span-or-current

## What it does

Checks for `or_current` called directly on `tracing::Span::current()`.

## Why is this bad?

`Span::or_current` returns the current span when its receiver is disabled. The
receiver is already the current span, so the call always returns its receiver
and only adds noise.

## Known problems

The lint only checks a direct `Span::current().or_current()` chain. It does not
follow the span through a local binding, a field, or a helper function.

## Example

```rust
fn current_span() -> tracing::Span {
    tracing::Span::current().or_current()
}
```

## Use instead

Remove the `or_current` call:

```rust
fn current_span() -> tracing::Span {
    tracing::Span::current()
}
```
