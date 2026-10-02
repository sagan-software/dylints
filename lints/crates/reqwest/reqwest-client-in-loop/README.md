# reqwest-client-in-loop

## What it does

Checks for `Client::new`, `Client::builder`, and `ClientBuilder::new` calls
inside a `loop`, `while`, or `for` body. It covers both the async client and
`reqwest::blocking::Client`.

## Why is this bad?

Each Reqwest client owns its own connection pool. A new client on every
iteration throws away the idle connections from the previous one. Each network
call then pays for a new DNS lookup, TCP connection, and TLS handshake.

## Known problems

The lint only finds direct constructor calls inside a loop expression. It
misses a constructor inside a function that the loop calls, and a constructor
inside an iterator closure such as `for_each`.

A loop that needs a separate client per iteration, for example to isolate
cookies or proxies, also triggers the lint.

## Example

```rust
async fn fetch_all(urls: &[&str]) -> Result<(), reqwest::Error> {
    for url in urls {
        let client = reqwest::Client::new();
        client.get(*url).send().await?;
    }
    Ok(())
}
```

## Use instead

```rust
async fn fetch_all(urls: &[&str]) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    for url in urls {
        client.get(*url).send().await?;
    }
    Ok(())
}
```
