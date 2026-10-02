# reqwest-tls-danger-invalid-hostnames

## What it does

Checks for a literal `true` passed to `tls_danger_accept_invalid_hostnames` or
`danger_accept_invalid_hostnames` on a Reqwest `ClientBuilder`.

## Why is this bad?

The client then accepts a valid certificate issued for any host. An attacker
with a certificate for their own domain can impersonate the server and read or
change the traffic.

## Known problems

The lint only checks the literal `true`. It misses a variable or constant set
to `true`. Test code that talks to a local server with a mismatched certificate
name also triggers the lint.

## Example

```rust
fn build_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_hostnames(true)
        .build()
}
```

## Use instead

Keep hostname verification enabled and use a certificate whose names match the
URL host:

```rust
fn build_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder().build()
}
```
