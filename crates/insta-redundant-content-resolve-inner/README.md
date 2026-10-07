# insta-redundant-content-resolve-inner

## What it does

Checks for a `Content::as_*` accessor called directly on the result of
`Content::resolve_inner()`, such as `content.resolve_inner().as_str()`.

## Why is this bad?

The `Content::as_*` accessors already remove Insta's internal wrappers. The
extra `resolve_inner()` call does nothing and suggests that the accessors need
it.

## Known problems

The lint checks only a direct method chain. It does not check
`resolve_inner()` stored in a variable and passed to an accessor later.

## Example

```rust
use insta::internals::Content;

fn text(content: &Content) -> Option<&str> {
    content.resolve_inner().as_str()
}
```

## Use instead

```rust
use insta::internals::Content;

fn text(content: &Content) -> Option<&str> {
    content.as_str()
}
```
