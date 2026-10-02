# manual_extend_loop

## What it does

Checks for a `for` loop whose body is only one `Vec::push` or
`VecDeque::push_back` call on a collection other than the loop source.

## Why is this bad?

The loop grows the collection one item at a time. `Extend::extend` adds the
items in one call and can reserve capacity from the iterator size hint. When
the pushed value is transformed, `extend` with `map` states the transformation
in one place.

## Known problems

- Only the standard `Vec::push` and `VecDeque::push_back` trigger. Other
  collections, such as `HashSet::insert`, and local types with a `push` method
  are ignored.
- The lint skips bodies whose source text contains `?`, `.await`, `break`,
  `continue`, or `return`. The check is a text match, so a pushed name such as
  `returned` also prevents the lint.

## Example

```rust
fn double_all(output: &mut Vec<i32>, values: Vec<i32>) {
    for value in values {
        output.push(value * 2);
    }
}
```

## Use instead

```rust
fn double_all(output: &mut Vec<i32>, values: Vec<i32>) {
    output.extend(values.into_iter().map(|value| value * 2));
}
```
