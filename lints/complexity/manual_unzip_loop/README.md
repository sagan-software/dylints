# manual_unzip_loop

## What it does

Checks for two empty mutable collections declared just before a
`for (first, second) in ...` loop whose body pushes or inserts `first` into the
first collection and `second` into the second, when the block returns both
collections as a tuple.

## Why is this bad?

The split takes two mutable bindings and a loop to do what one call does. A
later edit can swap the targets or drop one push. `Iterator::unzip` states the
pair split in one call.

## Known problems

- The collections must be standard `Vec`, `VecDeque`, `HashSet`, `BTreeSet`,
  `HashMap`, or `BTreeMap` values created by an argument-free `new` or
  `default` call owned by that collection. Lookalike associated functions on
  other types, `vec![]`, and `Vec::with_capacity(n)` are ignored.
- The loop pattern must be a two-name tuple, and the body must insert the names
  unchanged in pattern order with `push`, `push_back`, or `insert`. Swapped or
  transformed items are ignored.
- The block must end with the tuple of both collections in declaration order.
  A loop whose results are used another way is ignored.
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
