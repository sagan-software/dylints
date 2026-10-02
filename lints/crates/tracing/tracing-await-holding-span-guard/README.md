# tracing-await-holding-span-guard

## What it does

Checks for a `tracing::span::Entered` or `tracing::span::EnteredSpan` guard
that `Span::enter` or `Span::entered` returns and that crosses an `.await` in
an async function or block.

## Why is this bad?

While the future waits at the `.await`, the span stays entered. Other tasks then
run on the same thread inside that span, and tracing records their events under
it. Tracing documents this pattern as producing incorrect traces.

## Known problems

The lint can report a guard that code passes to `drop` before the `.await`. To
avoid this, hold the guard in a block that ends before the `.await`.

The lint checks only values whose type is the guard itself. A guard stored in
another type, such as `Option<EnteredSpan>` or a struct field, does not trigger
the lint.

## Example

```rust
async fn other_work() {}

async fn work(span: &tracing::Span) {
    let _guard = span.enter();
    other_work().await;
}
```

## Use instead

Use `Span::in_scope` for synchronous work, or instrument the future.

```rust
use tracing::Instrument;

async fn other_work() {}

async fn work(span: &tracing::Span) {
    other_work().instrument(span.clone()).await;
}
```
