# ad_hoc_from_str

## What it does

Checks free functions and inherent associated functions named `parse_*` or
`from_str*`. They must take one `&str` argument, have no `self` receiver, and
return `Result<T, E>`, where `T` is a struct, enum, or union defined in the
current crate without lifetime arguments. The lint skips types that already
implement `FromStr` and names that contain `_and_`, `_lenient`, `_lossy`, `_or_`,
`_strict`, `_unchecked`, or `_with_`.

## Why is this bad?

Callers cannot use a custom string parser with `str::parse`, and generic code
that asks for `T: FromStr` cannot accept the type. Callers must learn the local
function name instead of writing `"42".parse::<UserId>()`.

## Known problems

The lint does not read the function body, so it warns on parsers that are one
of several valid formats for the type.

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
