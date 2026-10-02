# consecutive_iterator_loops

## What it does

Checks for two adjacent `for` loops that iterate over plain local names (or a
borrow of one) and have the same body text.

## Why is this bad?

The same body is written twice. A later change to one copy can miss the other,
and the two loops then do different work by accident. `Iterator::chain` states
one ordered sequence and keeps one copy of the body.

## Known problems

- The lint compares the body source text only. It does not compare the loop
  patterns, so `for a in x { f(v); }` followed by `for b in y { f(v); }` also
  triggers.
- Sources other than a plain local name, `&name`, or `&mut name` never trigger.
  Field accesses such as `self.items` and calls such as `list.iter()` are
  ignored.
- `chain` converts the second source into an iterator before the first loop
  runs. The lint only emits help and does not offer an automatic fix.

## Example

```rust
fn report(first: &[i32], second: &[i32]) {
    for value in first {
        consume(value);
    }
    for value in second {
        consume(value);
    }
}
```

## Use instead

```rust
fn report(first: &[i32], second: &[i32]) {
    for value in first.iter().chain(second) {
        consume(value);
    }
}
```
