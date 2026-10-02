# reqwest-get-in-loop

## What it does

Checks for the `reqwest::get` and `reqwest::blocking::get` shortcut functions
inside a `loop`, `while`, or `for` body.

## Why is this bad?

Each shortcut call builds a new `Client` with its own connection pool. In a
loop, no connection is reused, so every request pays for a new DNS lookup, TCP
connection, and TLS handshake.

## Known problems

The lint only finds direct calls inside a loop expression. It misses a shortcut
inside a helper function called from the loop, and a shortcut inside an
iterator closure such as `for_each`.

## Example

```rust
async fn fetch_all(urls: &[&str]) -> Result<(), reqwest::Error> {
    for url in urls {
        let _response = reqwest::get(*url).await?;
    }
    Ok(())
}
```

## Use instead

```rust
async fn fetch_all(urls: &[&str]) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    for url in urls {
        let _response = client.get(*url).send().await?;
    }
    Ok(())
}
```
