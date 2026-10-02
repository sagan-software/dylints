# repeated_cfg_gate

## What it does

Checks for `#[cfg(...)]` predicates that appear more than three times on items
across a crate. It warns once per predicate, at the first occurrence. Gates on
test-only items and on anything inside them are not counted.

## Why is this bad?

Each gate is a place where the code differs between builds. When one predicate
is scattered across many items and files, readers cannot see the full set of
code the feature adds, and a change to the feature must touch every gate. A
single gated module keeps that code in one place.

## Known problems

Predicates are compared by structure, so `all(unix, feature = "abc")` and
`all(feature = "abc", unix)` count as the same predicate. The lint counts gates
on items, impl and trait items, enum variants, and fields. It does not count
`cfg_attr`, gates on statements or expressions, or gates produced by macros.

An item is test-only when it has an attribute whose last path segment is
`test`, or a `cfg` that requires `test`. The lint reads only files under the
package directory and skips files that `syn` cannot parse. Files of modules
disabled by `cfg` are not loaded, so their gates are not counted.

## Example

```rust
#[cfg(feature = "abc")]
fn first() {}

#[cfg(feature = "abc")]
fn second() {}

#[cfg(feature = "abc")]
fn third() {}

#[cfg(feature = "abc")]
fn fourth() {}
```

## Use instead

```rust
#[cfg(feature = "abc")]
mod abc {
    pub fn first() {}
    pub fn second() {}
    pub fn third() {}
    pub fn fourth() {}
}
```
