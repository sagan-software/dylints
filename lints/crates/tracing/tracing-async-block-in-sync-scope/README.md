# tracing-async-block-in-sync-scope

## What it does

Checks for an async closure, or a closure whose body is an async block, passed
directly to `Span::in_scope`, `tracing::subscriber::with_default`, or
`tracing::dispatcher::with_default`.

## Why is this bad?

These functions set the span or subscriber only while the closure runs. The
closure only creates the future and returns it. The future runs later, when it
is polled after the scope has ended, so its events miss the span or
subscriber.

## Known problems

The lint checks only a closure written directly as the argument. A closure
stored in a variable first, or a closure that calls a function returning a
future, does not trigger the lint.

## Example

```rust
async fn work() {}

async fn run(span: tracing::Span) {
    let future = span.in_scope(|| async { work().await });
    future.await;
}
```

## Use instead

Attach the span with `Instrument::instrument`, or attach a subscriber with
`WithSubscriber::with_subscriber`.

```rust
use tracing::Instrument;

async fn work() {}

async fn run(span: tracing::Span) {
    work().instrument(span).await;
}
```
