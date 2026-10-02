# ad_hoc_from

## What it does

Checks functions and methods named `make_*`, `build_*`, `convert_*`,
`extract_*`, `extracted_*`, or `from_*` that take one argument and return a
struct, enum, or union other than `Option` or `Result`. The argument or the
return type must be defined in the current crate, so that a `From`
implementation is allowed. Names that contain `_and_` and the `from` method of
a `From` implementation are skipped.

## Why is this bad?

A custom conversion function cannot be used with `.into()`, `?` error
conversion, or generic `T: Into<U>` bounds. Callers must learn the local
function name instead of using the standard trait.

## Known problems

The lint does not read the function body, so it warns on conversions that have
side effects or one of several valid policies. A receiver counts as the one
argument, so a method such as `fn build_request(&self) -> Request` warns. The
lint also warns on methods in trait implementations whose names the trait
fixes, and when a matching `From` implementation already exists.

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
