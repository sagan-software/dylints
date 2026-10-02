# manual_error_impl

## What it does

Checks types that have both a hand-written `Display` implementation whose
`fmt` body is one `write!(...)` expression and an empty `Error` implementation.

## Why is this bad?

The message lives in a separate `Display` implementation, away from the type,
and both implementations are boilerplate that must be kept in sync by hand.
`#[derive(thiserror::Error)]` with an `#[error("...")]` attribute keeps the
message on the type and generates both implementations.

## Known problems

The suggestion adds a dependency on the `thiserror` crate. The lint reads
source text, not types, and matches the two implementations by the type's
last path segment, so two types with the same name in different modules are
treated as one. A local trait named `Display` or `Error` also counts. The lint
skips generic types, `Error` implementations with methods such as `source`,
and `Display` bodies that contain `if `, `match `, `redact`, `secret`, `***`,
or `<redacted>`. It misses `Display` bodies that use `f.write_str` or more
than one statement.

## Example

```rust
use std::{error::Error, fmt};

#[derive(Debug)]
struct MissingUser {
    user_id: u64,
}

impl fmt::Display for MissingUser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing user {}", self.user_id)
    }
}

impl Error for MissingUser {}
```

## Use instead

```rust
#[derive(Debug, thiserror::Error)]
#[error("missing user {user_id}")]
struct MissingUser {
    user_id: u64,
}
```
