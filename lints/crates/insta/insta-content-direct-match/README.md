# insta-content-direct-match

## What it does

Checks for a `match` on a value of type `insta::internals::Content`, unless the
matched value is the result of `Content::resolve_inner()`.

## Why is this bad?

`Content` can wrap a value in internal variants, such as the variants for
`Option` and newtype structs. A `match` on the outer value misses a string or
number stored inside such a wrapper. This often breaks dynamic redaction
callbacks. The `Content::as_*` accessors and `resolve_inner()` remove the
wrappers first.

## Known problems

The lint warns even when the code means to inspect the outer wrapper.

## Example

```rust
use insta::internals::Content;

fn is_text(content: &Content) -> bool {
    match content {
        Content::String(_) => true,
        _ => false,
    }
}
```

## Use instead

Use an `as_*` accessor, or match on `content.resolve_inner()`.

```rust
use insta::internals::Content;

fn is_text(content: &Content) -> bool {
    content.as_str().is_some()
}
```
