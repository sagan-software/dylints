# ad_hoc_try_from

## What it does

Checks functions and associated functions named `make_*`, `build_*`,
`convert_*`, `map_*`, `try_*`, or `validate_*` that take one argument, have no
`self` receiver, and return `Result<T, E>`. The argument or `T` must be defined
in the current crate, so that a `TryFrom` implementation is allowed. The
`try_from` method of a `TryFrom` implementation is skipped.

## Why is this bad?

A custom fallible conversion cannot be used with `.try_into()` or generic
`T: TryInto<U>` bounds. Callers must learn the local function name instead of
using the standard trait.

## Known problems

The lint does not read the function body. The `try_*` and `map_*` prefixes
also match operations that are not conversions, such as
`fn try_connect(address: Address) -> Result<Connection, Error>`. The lint also
warns on associated functions in trait implementations whose names the trait
fixes.

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
