# expensive_sort_key

## What it does

Checks the standard stable slice `sort_by_key` method for closures that call
recognized methods that return an owned `String`: Unicode and ASCII case
conversion plus owned `String` cloning. Calls must resolve to standard-library
methods, and the key closure must only project a field from its argument before
applying the recognized transformation.

## Why is this bad?

`sort_by_key` may call its key closure repeatedly while comparing elements.
When the key operation allocates, repeated comparisons repeat allocation and
conversion work. `sort_by_cached_key` evaluates the key at most once per element
and preserves stable ordering.

## Known problems

The lint skips arrays with a statically known length of at most two. For two
elements, sorting evaluates each key once, so caching does not reduce key calls.
It does not inspect runtime `Vec` or slice lengths, or unresolved const-generic
array lengths, so it can still diagnose empty or small inputs when their length
is unknown during linting. Case-conversion calls through user-defined
`Deref<Target = str>` receivers are skipped; only actual `str` and standard
`String` receivers qualify. The lint also skips arbitrary helpers, custom
methods, closure blocks with additional statements, and `sort_unstable_by_key`.
It does not report side-effecting or non-deterministic callbacks.

Caching
changes callback count and order, so no automatic fix is offered. The cached
method uses temporary storage, and the lint does not measure whether a
particular collection is large enough to benefit.

## Example

```rust
fn sort_names(names: &mut [String]) {
    names.sort_by_key(|name| name.to_lowercase());
}
```

## Use instead

```rust
fn sort_names(names: &mut [String]) {
    names.sort_by_cached_key(|name| name.to_lowercase());
}
```
