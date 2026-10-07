# insta-settings-bind-future

## What it does

Checks for `insta::Settings::bind` calls whose callable returns a `Future`,
including futures returned by function calls or stored closures.

## Why is this bad?

`Settings::bind` applies the settings only while its callable runs. If the
caller polls the returned future, that happens after the scope ends, so snapshots
inside it do not see the settings.

## Known problems

The lint detects any callable whose result implements `Future`, but its
machine-applicable fix remains limited to a parameterless synchronous closure
that returns a direct async block. That fix renames `bind` to `bind_async` and
removes the closure head. A `move` closure needs an `async move` block to
preserve captures. Async closures, function calls, stored closures, and other
callables receive help without a fix. For those cases, review captures and side
effects because constructing the future directly can change when callable code
runs.

## Example

```rust
use insta::Settings;

async fn work() {}

async fn snapshot_work() {
    let settings = Settings::clone_current();
    let future = settings.bind(|| async { work().await });
    future.await;
}
```

## Use instead

```rust
use insta::Settings;

async fn work() {}

async fn snapshot_work() {
    let settings = Settings::clone_current();
    settings.bind_async(async { work().await }).await;
}
```
