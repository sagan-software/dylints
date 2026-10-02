# manual_partition_loop

## What it does

Checks for two empty mutable collections declared just before a `for` loop
whose body is one `if`/`else` that pushes or inserts the same item into the
first collection or the second, when the block returns `(first, second)`.

## Why is this bad?

The split takes two mutable bindings, a loop, and two branches that repeat the
item. A later edit can change the item in one branch only. `Iterator::partition`
states the two-way split in one call.

## Known problems

- The collections must be standard `Vec`, `VecDeque`, `HashSet`, `BTreeSet`,
  `HashMap`, or `BTreeMap` values created with `::new()` or `::default()`.
  `vec![]` and `Vec::with_capacity(n)` are ignored.
- The block must end with the tuple of both collections in declaration order.
  A loop whose results are used another way is ignored.
- The lint does not check that both collections have the same type.
  `partition` needs one type for both sides, so a `Vec` paired with a `HashSet`
  triggers but cannot use `partition` directly.
- The predicate closure receives `&T`, so it can need a dereference. The lint
  emits help without an automatic fix.

## Example

```rust
fn split_sign(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    let mut positive = Vec::new();
    let mut other = Vec::new();
    for value in values {
        if value > 0 {
            positive.push(value);
        } else {
            other.push(value);
        }
    }
    (positive, other)
}
```

## Use instead

```rust
fn split_sign(values: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    values.into_iter().partition(|value| *value > 0)
}
```
