# repeated-cfg-gate

## What it does

Checks for `#[cfg(...)]` predicates that appear more than three times on items
across a crate. It warns once per predicate, at the first occurrence. The lint does not count gates on test-only items or anything inside them.

## Why is this bad?

Each gate is a place where the code differs between builds. When one predicate appears across many items and files, readers cannot see the full set of code the feature adds. A change to the feature must then touch every gate. A
single gated module keeps that code in one place.

## Known problems

The lint compares predicates by structure, so `all(unix, feature = "abc")` and `all(feature = "abc", unix)` count as the same predicate. The lint counts gates
on items, impl and trait items, enum variants, and fields. It does not count
`cfg_attr`, gates on statements or expressions, or gates produced by macros.

An item is test-only when it has an attribute whose last path segment is
`test`, or a `cfg` that requires `test`. The lint reads only files under the
package directory and skips files that `syn` cannot parse. The parser does not load module files when `cfg` disables the modules, so the lint does not count their gates.

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
