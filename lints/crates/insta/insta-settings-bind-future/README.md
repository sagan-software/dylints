# insta-settings-bind-future

## What it does

Checks for `insta::Settings::bind` calls whose closure returns an async block,
or whose argument is an async closure, such as
`settings.bind(|| async { ... })`.

## Why is this bad?

`Settings::bind` applies the settings only while its closure runs. The closure
only creates the future, and the settings are removed before the future runs.
Snapshots inside the future do not see the settings.

## Known problems

The lint does not check a closure that returns a future from a function call,
such as `settings.bind(|| work())`. The suggested fix renames `bind` to
`bind_async` but keeps the closure. Remove `||` as well, because `bind_async`
takes the future itself.

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
