# expensive_as_method

## What it does

Checks inherent methods named `as_*` that return `Option` or `Result`, or whose
source contains a clone, allocation, parse, or decode call such as `.clone()`,
`.to_string()`, `format!`, `.parse()`, `decode(..)`, or the `?` operator.

## Why is this bad?

By Rust API naming conventions, `as_*` methods are cheap borrowed views. A
caller who sees `as_string()` expects no allocation or failure, so hidden
clones and parses end up in loops and hot paths. A `to_*`, `try_*`, or
`parse_*` name shows the cost at the call site.

## Known problems

Every `as_*` method that returns `Option` or `Result` is reported, including
cheap borrowed accessors such as `fn as_text(&self) -> Option<&str>` on an
enum.

The body check matches source text. It reports cheap calls, such as
`.clone()` on an `Arc`, and it matches text inside comments and string
literals. It misses work done through helper functions with other names.

Trait methods and trait impl methods are not checked.

## Example

```rust
struct Token {
    raw: String,
}

impl Token {
    fn as_string(&self) -> String {
        self.raw.clone()
    }

    fn as_number(&self) -> Result<u64, std::num::ParseIntError> {
        self.raw.parse()
    }
}
```

## Use instead

```rust
struct Token {
    raw: String,
}

impl Token {
    fn to_raw_string(&self) -> String {
        self.raw.clone()
    }

    fn parse_number(&self) -> Result<u64, std::num::ParseIntError> {
        self.raw.parse()
    }
}
```
