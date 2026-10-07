# manual-iterator-loop

## What it does

Checks for a mutable accumulator declared just before a `for` loop that fills
it in one of four ways:

- `Vec::new()` followed by a body that is only `out.push(..)`.
- An integer `0` followed by one conditional increment using `count += 1`,
  `count = count + 1`, or `count = 1 + count`.
- `false` followed by a body that is only `if condition { found = true }`.
- `true` followed by a body that is only `if condition { all_ok = false }`.

## Why is this bad?

The mutable binding, the loop, and the update spread one result over several
lines. The boolean loops also keep iterating after they find the answer.
`map(...).collect()`, `filter(...).count()`, `any(...)`, and `all(...)` compute
the same result in one expression, and `any` and `all` stop at the first
decisive item.

## Known problems

- The lint requires the accumulator declaration directly before the loop. It
  ignores a loop separated from its accumulator by another statement.
- The loop body must be the single push or the single `if` shown above. The
  lint ignores a `push` inside an `if`, a body with extra statements, and a
  pushed value or condition that contains `break`, `continue`, `return`, `?`,
  `.await`, a loop, or an assignment.
- The lint ignores a pushed value or condition that reads or captures the
  accumulator. The lint accepts a nested closure that shadows the accumulator
  with a separate binding.
- `any` and `all` stop at the first decisive item. If the condition or the
  iterator has side effects, the rewrite runs them fewer times.
- A custom iterator's `size_hint()` can have side effects or change later
  `next()` values. Collection adapters can call `size_hint()` during source
  iteration, so use `.collect()` only when the source iterator's `size_hint()`
  has no side effects.
- The lint ignores an accumulator declared inside a macro expansion.
- Only `Vec::new()` starts a collection. The lint ignores `vec![]` and
  `Vec::with_capacity(n)`.
- A `Vec` push loop can also trigger `manual_extend_loop`.

## Example

```rust
fn count_even(values: &[i32]) -> usize {
    let mut count = 0;
    for value in values {
        if *value % 2 == 0 {
            count += 1;
        }
    }
    count
}
```

## Use instead

```rust
fn count_even(values: &[i32]) -> usize {
    values.iter().filter(|value| **value % 2 == 0).count()
}
```
