# insta-settings-bind-future

## What it does

Checks for `insta::Settings::bind` calls whose closure returns an async block,
or whose argument is an async closure, such as
`settings.bind(|| async { ... })`.

## Why is this bad?

`Settings::bind` applies the settings only while its closure runs. The closure
only creates the future, and the call removes the settings before the future runs.
Snapshots inside the future do not see the settings.

## Known problems

The lint does not check a closure that returns a future from a function call,
such as `settings.bind(|| work())`.

The machine-applicable fix renames `bind` to `bind_async` and removes the
closure head, so `bind(|| async { .. })` becomes `bind_async(async { .. })`. The
lint offers it only when the parameterless closure returns the async block
directly. A `move` closure also needs an `async move` block. Async closures
and other `move` closures get help without a fix.

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
