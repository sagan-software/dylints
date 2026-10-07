# struct-update-default

## What it does

Checks for struct literals whose update base is `Default::default()`, such as
`Request { timeout: 5, ..Default::default() }`.

## Why is this bad?

The literal does not show which fields take default values. When someone adds a
field to the struct, every such literal still compiles and takes the default.
Adding a field triggers no call-site review.

## Known problems

When the base is the built-in `#[derive(Default)]` of a struct in the same crate,
the lint suggests listing each remaining field as `field: Default::default()`.
It makes this suggestion only when those fields have no default field values.
That derive calls
`Default::default()` for each field, so the values do not change. Other cases,
such as a handwritten `Default` impl, get help text only.
When a generic whole-struct bound does not identify the default implementation,
the lint also offers help text only.

It warns on types from other crates that have many fields, where listing every
field is long and the crate documents `..Default::default()` as the intended
style. It does not flag other update bases, such as `..base` or
`..Request::new()`.

## Example

```rust
#[derive(Default)]
struct Request {
    timeout: u64,
    retries: u8,
}

fn build() -> Request {
    Request {
        timeout: 5,
        ..Default::default()
    }
}
```

## Use instead

```rust
#[derive(Default)]
struct Request {
    timeout: u64,
    retries: u8,
}

fn build() -> Request {
    Request {
        timeout: 5,
        retries: 0,
    }
}
```
