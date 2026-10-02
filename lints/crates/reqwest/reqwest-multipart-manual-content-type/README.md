# reqwest-multipart-manual-content-type

## What it does

Checks for a `RequestBuilder` method chain that calls `.multipart(form)` and
also sets the `Content-Type` header with `.header(...)`, in either order.

## Why is this bad?

`.multipart` sets `Content-Type` to `multipart/form-data` with the boundary
string that separates the form parts. A manual value can drop that boundary or
replace the generated header, so the server cannot split the body into parts.

## Known problems

The lint only follows one method chain. It misses a header set in a separate
statement or through `.headers(map)`. It recognizes the header name only as the
`CONTENT_TYPE` constant or a string literal equal to `content-type`, ignoring
case.

## Example

```rust
fn upload(client: &reqwest::Client, form: reqwest::multipart::Form) -> reqwest::RequestBuilder {
    client
        .post("https://example.com")
        .header(reqwest::header::CONTENT_TYPE, "multipart/form-data")
        .multipart(form)
}
```

## Use instead

```rust
fn upload(client: &reqwest::Client, form: reqwest::multipart::Form) -> reqwest::RequestBuilder {
    client.post("https://example.com").multipart(form)
}
```
