# tracing-instrument-current-span

## What it does

Checks for `Instrument::instrument` called with a direct
`tracing::Span::current()` argument.

## Why is this bad?

`Instrument::in_current_span` does the same thing. The longer form makes readers
inspect the argument to learn that the future uses the current span.

## Known problems

The lint only checks a direct `Span::current()` argument. It does not follow the
span through a local binding, a field, or a helper function.

## Example

```rust
use tracing::Instrument as _;

async fn work() {}

fn spawn_work() {
    let _task = work().instrument(tracing::Span::current());
}
```

## Use instead

Call `in_current_span`:

```rust
use tracing::Instrument as _;

async fn work() {}

fn spawn_work() {
    let _task = work().in_current_span();
}
```
