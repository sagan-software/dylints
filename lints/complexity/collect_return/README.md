# collect_return

## What it does

Checks for `pub` functions and methods whose final expression is an
`Iterator::collect` call and whose return type is `Vec`, `HashMap`, `HashSet`,
`BTreeMap`, `BTreeSet`, or `Box<[T]>`, including through type aliases.

## Why is this bad?

The function allocates and fills a collection on every call, and the signature
makes that allocation part of the public API. A caller that only iterates, takes
the first match, or collects into another type still pays for the collection.
Returning `impl Iterator` lets each caller decide whether to collect.

## Known problems

- The lint cannot tell when the caller needs an owned collection, for example
  for indexing, `len`, or several passes. Keep the collection in those cases and
  allow the lint.
- An iterator that borrows an argument ties the return value to that borrow,
  which some callers cannot accept.
- Only a direct final `.collect()` triggers. `Ok(iter.collect())`, a collected
  local returned later, and `VecDeque` return types are ignored.
- Private functions are ignored.

## Example

```rust
pub fn even_ids(ids: &[u64]) -> Vec<u64> {
    ids.iter().copied().filter(|id| id % 2 == 0).collect()
}
```

## Use instead

```rust
pub fn even_ids(ids: &[u64]) -> impl Iterator<Item = u64> + '_ {
    ids.iter().copied().filter(|id| id % 2 == 0)
}
```
