# trivial_public_constructor

## What it does

Checks for an associated function named `new` without `self` whose body is only
a struct literal that uses field shorthand for every parameter, when every
field is `pub`.

## Why is this bad?

Callers can already build the value with `User { id, name }`. The constructor
adds a public function to document and maintain, and it does not enforce any
invariant that the struct literal skips.

## Known problems

- Removing a public `new` breaks callers that use it. Keep it when the crate's
  API must stay stable.
- The lint does not check `#[non_exhaustive]`. Code in other crates cannot use a
  struct literal for a non-exhaustive struct, so `new` is the only way to build
  it there.
- The lint does not check whether `new` itself or the struct is public.
- Fields marked `pub(crate)` or private, tuple structs, and literals with
  non-shorthand fields such as `name: name.trim().to_owned()` are ignored.

## Example

```rust
pub struct User {
    pub id: UserId,
    pub name: String,
}

impl User {
    pub fn new(id: UserId, name: String) -> Self {
        Self { id, name }
    }
}
```

## Use instead

```rust
pub struct User {
    pub id: UserId,
    pub name: String,
}

fn build(id: UserId, name: String) -> User {
    User { id, name }
}
```
