# ad-hoc-borrow

## What it does

Checks inherent methods named `borrow_*` that take only a `&self` receiver and
return a shared reference. The lint skips types that already implement `Borrow`
for the returned type.

## Why is this bad?

Generic code that asks for `T: Borrow<U>`, such as `HashMap::get`, cannot call a
custom borrow method. Callers must learn the local method name instead of using
the standard trait.

## Known problems

`Borrow` is correct only when `Eq`, `Ord`, and `Hash` give the same results for
the owned and the borrowed value. The lint cannot check this, so it can warn on
a method that must not become `Borrow`.

## Example

```rust
struct UserName(String);

impl UserName {
    fn borrow_str(&self) -> &str {
        &self.0
    }
}
```

## Use instead

```rust
use std::borrow::Borrow;

struct UserName(String);

impl Borrow<str> for UserName {
    fn borrow(&self) -> &str {
        &self.0
    }
}
```
