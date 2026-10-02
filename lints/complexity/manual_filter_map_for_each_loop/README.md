# manual_filter_map_for_each_loop

## What it does

Checks for a `for` loop whose body is only `if let Some(..) = expr` without
`else`, where `expr` is an `Option` and the branch holds one call, method call,
or assignment.

## Why is this bad?

The loop mixes the conversion, the skip of `None`, and the action in nested
blocks. `filter_map(...).for_each(...)` names the conversion and the action as
separate steps.

## Known problems

- The lint skips bodies that contain `?`, `.await`, `break`, `continue`, or
  `return` outside a closure.
- The lint skips `if let Some(..) = result.ok()`, which reads better as
  `if let Ok(..) = result`.
- The `Some` payload pattern must be `_` or a binding without a subpattern.
  The lint ignores tuple patterns, `x @ _`, and partial patterns such as
  `Some(0)`. `filter_map` would run the action for all `Some` values.
- Compound assignments such as `total += value` do not count as an action.
  The lint ignores those loops.
- The lint emits help without an automatic fix because closure arguments can
  need adjustment.

## Example

```rust
fn consume_numbers(values: &[&str]) {
    for value in values {
        if let Some(number) = parse(value) {
            consume(number);
        }
    }
}
```

## Use instead

```rust
fn consume_numbers(values: &[&str]) {
    values
        .iter()
        .filter_map(|value| parse(value))
        .for_each(consume);
}
```
