# reqwest-cookie-provider-overridden

## What it does

Checks for a `ClientBuilder` method chain that calls `cookie_store(true)` after
`cookie_provider(...)`.

## Why is this bad?

`cookie_store(true)` installs Reqwest's default cookie store and replaces the
provider set earlier in the chain. The client silently drops the custom
provider and any persistence or filtering it implements.

## Known problems

The lint only follows one method chain. It misses a builder stored in a
variable and configured in separate statements. It only checks the literal
`true`, so `cookie_store(flag)` does not trigger the lint.

## Example

```rust
use std::sync::Arc;

fn build_client() -> Result<reqwest::Client, reqwest::Error> {
    let jar = Arc::new(reqwest::cookie::Jar::default());
    reqwest::Client::builder()
        .cookie_provider(jar)
        .cookie_store(true)
        .build()
}
```

## Use instead

```rust
use std::sync::Arc;

fn build_client() -> Result<reqwest::Client, reqwest::Error> {
    let jar = Arc::new(reqwest::cookie::Jar::default());
    reqwest::Client::builder()
        .cookie_provider(jar)
        .build()
}
```
