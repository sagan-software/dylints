# struct_update_default

## What it does

Checks for struct literals whose update base is `Default::default()`, such as
`Request { timeout: 5, ..Default::default() }`.

## Why is this bad?

The literal does not show which fields take default values. When someone adds a
field to the struct, every such literal compiles unchanged and takes the default,
so no call site is reviewed for the new field.

## Known problems

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
