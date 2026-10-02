# manual_partition_loop

## What it does

Checks for two empty mutable collections of the same type declared just before
a `for` loop whose body is one `if`/`else` that inserts the loop item into the
first collection or the second, when the block returns `(first, second)`.

## Why is this bad?

The split takes two mutable bindings, a loop, and two branches that repeat the
item. A later edit can change the item in one branch only. `Iterator::partition`
states the two-way split in one call.

## Known problems

- The collections must be standard `Vec`, `VecDeque`, `HashSet`, or `BTreeSet`
  values created with the collection's own `new` function (including `vec![]`)
  or with `Default::default`. `Vec::with_capacity(n)` is ignored. Maps are
  ignored because their insertion takes a key and a value.
- Each branch must call `push`, `push_back`, or `insert` with the loop binding
  itself. A loop pattern that destructures the item is ignored.
- The condition must not use either collection, and must not contain `return`,
  `break`, `continue`, `?`, or `.await`, because it moves into a closure.
- The block must end with the tuple of both collections in declaration order.
  A loop whose results are used another way is ignored.
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
