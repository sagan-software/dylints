# ad_hoc_try_from

## What it does

Checks free functions and inherent associated functions named `make_*`,
`build_*`, `convert_*`, `map_*`, `try_*`, or `validate_*` that take one
argument, have no `self` receiver, and return `Result<T, E>`.
The argument or `T` must come from the current crate, so this crate can add a
`TryFrom` implementation. The lint skips a `T` that is `()`, `!`, or the argument
type itself. It also skips an argument that is a bare type parameter and
conversions for which a
`TryFrom` implementation already applies, including through `From`.

## Why is this bad?

A custom fallible conversion does not support `.try_into()` or generic
`T: TryInto<U>` bounds. Callers must learn the local function name instead of
using the standard trait.

## Known problems

The lint does not read the function body. The `try_*` and `map_*` prefixes
also match operations that are not conversions, such as
`fn try_connect(address: Address) -> Result<Connection, Error>`.

## Example

```rust
struct RawUserId(u64);
struct UserId(u64);
struct InvalidUserId;

fn make_user_id(raw: RawUserId) -> Result<UserId, InvalidUserId> {
    if raw.0 == 0 {
        return Err(InvalidUserId);
    }
    Ok(UserId(raw.0))
}
```

## Use instead

```rust
struct RawUserId(u64);
struct UserId(u64);
struct InvalidUserId;

impl TryFrom<RawUserId> for UserId {
    type Error = InvalidUserId;

    fn try_from(raw: RawUserId) -> Result<Self, Self::Error> {
        if raw.0 == 0 {
            return Err(InvalidUserId);
        }
        Ok(Self(raw.0))
    }
}
```
