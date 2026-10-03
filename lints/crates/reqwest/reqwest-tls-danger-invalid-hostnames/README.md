# reqwest-tls-danger-invalid-hostnames

## What it does

Checks `tls_danger_accept_invalid_hostnames` and deprecated
`danger_accept_invalid_hostnames` on a Reqwest `ClientBuilder`. It reports
`true`, local boolean constants set to `true`, and bounded `!`, `&&`, and `||`
expressions that resolve to `true`.

## Why is this bad?

The client then accepts a valid certificate issued for any host. An attacker
with a certificate for their own domain can impersonate the server and read or
change the traffic.

## Known problems

Unknown runtime values remain unknown unless a known left operand of `&&` or
`||` short-circuits to a result without inspecting the right operand.
Associated and external constants, comparisons, and expressions over 16 visited
nodes remain unknown. It can still report test code that intentionally accepts
a mismatched certificate name from a local server.

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
