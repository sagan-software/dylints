# reqwest-tls-danger-invalid-certs

## What it does

Checks for a literal `true` passed to `tls_danger_accept_invalid_certs` or
`danger_accept_invalid_certs` on a Reqwest `ClientBuilder`.

## Why is this bad?

The client then trusts any certificate for any site, including expired and
self-signed ones. An attacker on the network can present their own certificate
and read or change the traffic.

## Known problems

The lint only checks the literal `true`. It misses a variable or constant set
to `true`. Test code that talks to a local server with a self-signed
certificate also triggers the lint.

## Example

```rust
fn build_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_danger_accept_invalid_certs(true)
        .build()
}
```

## Use instead

Trust the specific certificate the server uses:

```rust
fn build_client(cert: reqwest::Certificate) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .tls_certs_merge([cert])
        .build()
}
```
