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

- The lint skips bodies whose source text contains `?`, `.await`, `break`,
  `continue`, `return`, or `.ok()`. The check is a text match, so a name such as
  `returned` also prevents the lint.
- Compound assignments such as `total += value` do not count as an action, so
  those loops are ignored.
- The same loop can also trigger `manual_filter_for_each_loop`.
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
