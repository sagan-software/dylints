# manual_unzip_loop

## What it does

Checks for two empty mutable collections declared just before a loop.
The loop must use the pattern `for (first, second) in ...`.
Its body must push or insert each binding into the matching collection.
The block must return both collections as a tuple.

## Why is this bad?

The split takes two mutable bindings and a loop to do what one call does. A
later edit can swap the targets or drop one push. `Iterator::unzip` states the
pair split in one call.

## Known problems

- The collections must be standard `Vec`, `VecDeque`, `HashSet`, `BTreeSet`,
  `HashMap`, or `BTreeMap` values created by an argument-free `new` or
  `default` call owned by that collection. The lint ignores lookalike
  associated functions on other types, `vec![]`, and `Vec::with_capacity(n)`.
- The loop pattern must be a two-name tuple, and the body must insert the names
  unchanged in pattern order with `push`, `push_back`, or `insert`. The lint
  ignores swapped or transformed items.
- The block must end with the tuple of both collections in declaration order.
  The lint ignores a loop when another expression consumes its results.
- The lint emits help without an automatic fix.

## Example

```rust
fn split_pairs(pairs: Vec<(i32, String)>) -> (Vec<i32>, Vec<String>) {
    let mut ids = Vec::new();
    let mut names = Vec::new();
    for (id, name) in pairs {
        ids.push(id);
        names.push(name);
    }
    (ids, names)
}
```

## Use instead

```rust
fn split_pairs(pairs: Vec<(i32, String)>) -> (Vec<i32>, Vec<String>) {
    pairs.into_iter().unzip()
}
```
