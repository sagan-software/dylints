# ad_hoc_from_str

## What it does

Checks functions and associated functions named `parse_*` or `from_str*` that
take one `&str` argument, have no `self` receiver, and return `Result<T, E>`,
where `T` is not `()`, `!`, or a reference. Names that contain `_and_`,
`_lenient`, `_lossy`, `_or_`, `_strict`, `_unchecked`, or `_with_` are skipped.
The `from_str` method of a `FromStr` implementation is skipped.

## Why is this bad?

A custom string parser cannot be used with `str::parse`, and generic code that
asks for `T: FromStr` cannot accept the type. Callers must learn the local
function name instead of writing `"42".parse::<UserId>()`.

## Known problems

The lint does not read the function body, so it warns on parsers that are one
of several valid formats for the type. It also warns on associated functions
in trait implementations whose names the trait fixes. `FromStr` cannot borrow
from the input, so a parser that returns data borrowed from the `&str` cannot
follow the suggestion; the lint skips only reference return types, not types
with a lifetime parameter.

## Example

```rust
struct UserId(u64);
struct InvalidUserId;

fn parse_user_id(raw: &str) -> Result<UserId, InvalidUserId> {
    raw.parse().map(UserId).map_err(|_| InvalidUserId)
}
```

## Use instead

```rust
use std::str::FromStr;

struct UserId(u64);
struct InvalidUserId;

impl FromStr for UserId {
    type Err = InvalidUserId;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        raw.parse().map(UserId).map_err(|_| InvalidUserId)
    }
}
```
