# reqwest-client-wrapped-in-shared-pointer

## What it does

Checks for an async `reqwest::Client` passed directly to `Arc::new` or
`Rc::new`.

## Why is this bad?

`Client` already holds its state in an `Arc`, and cloning it shares the same
connection pool. The outer `Arc` or `Rc` adds an allocation, a second reference
count, and an extra pointer hop on every use, with no benefit.

## Known problems

The lint only checks the `Arc::new` and `Rc::new` calls themselves. It misses a
client wrapped through `Arc::from`, `Into`, or a helper function. It does not
check `reqwest::blocking::Client`.

## Example

```rust
use std::sync::Arc;

fn shared_client() -> Arc<reqwest::Client> {
    let client = reqwest::Client::new();
    Arc::new(client)
}
```

## Use instead

```rust
fn shared_client() -> reqwest::Client {
    let client = reqwest::Client::new();
    client.clone()
}
```
