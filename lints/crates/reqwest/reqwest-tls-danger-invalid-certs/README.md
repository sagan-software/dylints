# reqwest-tls-danger-invalid-certs

## What it does

Checks `tls_danger_accept_invalid_certs` and deprecated
`danger_accept_invalid_certs` on a Reqwest `ClientBuilder`. It reports `true`,
local boolean constants set to `true`, and bounded `!`, `&&`, and `||`
expressions that resolve to `true`.

## Why is this bad?

The client then trusts any certificate for any site, including expired and
self-signed ones. An attacker on the network can present their own certificate
and read or change the traffic.

## Known problems

Unknown runtime values remain unknown unless a known left operand of `&&` or
`||` short-circuits to a result without inspecting the right operand.
Associated and external constants, comparisons, and expressions over 16 visited
nodes remain unknown. It can still report test code that intentionally accepts
a self-signed certificate from a local server.

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
