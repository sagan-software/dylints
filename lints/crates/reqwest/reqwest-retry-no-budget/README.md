# reqwest-retry-no-budget

## What it does

Checks for calls to `reqwest::retry::Builder::no_budget`.

## Why is this bad?

`no_budget` removes the limit on retry traffic, which Reqwest documents as not
recommended. When a service starts failing, every client retries without limit
and adds load to the service that is already failing. This can turn a short
outage into a retry storm.

## Known problems

The lint flags every `no_budget` call, including ones in tests or benchmarks
that need unlimited retries.

## Example

```rust
fn retry_policy() -> reqwest::retry::Builder {
    reqwest::retry::for_host("example.com").no_budget()
}
```

## Use instead

```rust
fn retry_policy() -> reqwest::retry::Builder {
    reqwest::retry::for_host("example.com").max_extra_load(0.2)
}
```
