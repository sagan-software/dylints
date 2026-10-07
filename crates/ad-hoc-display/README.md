# ad-hoc-display

## What it does

Checks inherent methods named `to_string`, `display`, `format`, or `render`
that take only a `&self` receiver and return `String`. The lint skips types that
already implement `Display`.

## Why is this bad?

A custom formatting method does not work with `format!`, `println!`, `{}`
placeholders, logging macros, or generic `T: Display` bounds. It also
allocates a `String` even when the caller only writes the text to a stream.

## Known problems

The lint checks only the name and signature. It warns on a `render` or `format`
method that produces a document or report rather than the type's one textual
form.

## Example

```rust
struct UserId(String);

impl UserId {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}
```

## Use instead

```rust
use std::fmt;

struct UserId(String);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
```
