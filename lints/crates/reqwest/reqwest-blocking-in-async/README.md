# reqwest-blocking-in-async

## What it does

Checks for calls to `reqwest::blocking` functions and methods inside an async
function, async block, or async closure.

## Why is this bad?

The blocking API runs its own runtime and waits for it on the current thread.
Inside an async body, that wait stalls the executor thread, so other futures on
it stop making progress. Reqwest can also panic when a blocking client is
created or dropped inside a Tokio runtime.

## Known problems

The lint only looks at the nearest enclosing closure or async body. It misses a
blocking call inside a synchronous closure that runs in the async body, such as
an iterator adapter. This same rule keeps calls inside a
`tokio::task::spawn_blocking` closure from triggering the lint.

The lint does not follow function calls. A synchronous helper that uses
`reqwest::blocking` and is called from an async function does not trigger it.

## Example

```rust
async fn fetch() -> Result<(), reqwest::Error> {
    let _response = reqwest::blocking::get("https://example.com")?;
    Ok(())
}
```

## Use instead

```rust
async fn fetch() -> Result<(), reqwest::Error> {
    let _response = reqwest::get("https://example.com").await?;
    Ok(())
}
```
