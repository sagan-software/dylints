# reqwest-blocking-in-async

## What it does

Checks for calls to `reqwest::blocking` functions and methods inside an async
function, async block, or async closure.

## Why is this bad?

The blocking API runs its own runtime and waits for it on the current thread.
Inside an async body, that wait stalls the executor thread, so other futures on
it stop making progress. Tokio can also panic when code creates or drops a
blocking client inside a Tokio runtime.

## Known problems

The lint only looks at the nearest enclosing closure or async body. It misses a
blocking call inside a synchronous closure that runs in the async body, such as
an iterator adapter. This same rule keeps calls inside a
`tokio::task::spawn_blocking` closure from triggering the lint.

The lint does not follow function calls. A synchronous function that uses
`reqwest::blocking` does not trigger the lint when async code calls it.

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
