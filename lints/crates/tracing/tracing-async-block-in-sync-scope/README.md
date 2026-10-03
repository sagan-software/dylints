# tracing-async-block-in-sync-scope

## What it does

Checks whether `Span::in_scope`, `tracing::subscriber::with_default`, or
`tracing::dispatcher::with_default` returns a `Future`, regardless of whether
its callable argument is a closure, function item, or stored value.

## Why is this bad?

These functions set the span or subscriber only while the callable runs. If the
caller polls the returned future, that happens after the scope ends, so its
events miss the span or subscriber.

## Known problems

The lint flags any returned `Future`, but cannot determine whether or when the
caller polls it or whether that context is intended for its events.

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
