# ad_hoc_from

## What it does

Checks free functions and inherent associated functions named `make_*`,
`build_*`, `convert_*`, `extract_*`, `extracted_*`, or `from_*` that take one
argument and return a struct, enum, or union other than `Option` or `Result`.
The argument or the return type must be defined in the current crate, so that
a `From` implementation is allowed. Names that contain `_and_` are skipped.

The lint skips methods with a `self` receiver, methods in trait
implementations, arguments that are a bare type parameter, conversions whose
`From` implementation already exists, and bodies that can panic through
`panic!`, `assert!`, `unreachable!`, `todo!`, `unimplemented!`, or `unwrap` and
`expect` on `Option` or `Result`. `From` promises an infallible conversion, so
such a body needs `TryFrom` instead.

## Why is this bad?

A custom conversion function cannot be used with `.into()`, `?` error
conversion, or generic `T: Into<U>` bounds. Callers must learn the local
function name instead of using the standard trait.

## Known problems

The lint warns on conversions that have side effects or one of several valid
policies. It does not see panics inside called functions, indexing, or
arithmetic overflow, so a function that fails only through those still warns.

## Example

```rust
struct RawUserId(u64);
struct UserId(u64);

fn make_user_id(raw: RawUserId) -> UserId {
    UserId(raw.0)
}
```

## Use instead

```rust
struct RawUserId(u64);
struct UserId(u64);

impl From<RawUserId> for UserId {
    fn from(raw: RawUserId) -> Self {
        Self(raw.0)
    }
}
```
