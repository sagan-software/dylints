# manual_extend_loop

## What it does

Checks for a `for` loop whose body is only one `Vec::push` or
`VecDeque::push_back` call on a local variable, a field, or a dereference of
one. When the loop pushes its unchanged item, the lint suggests replacing the
loop with one `extend` call.

## Why is this bad?

The loop grows the collection one item at a time. `Extend::extend` adds the
items in one call and can reserve capacity from the iterator size hint. When
the pushed value is transformed, `extend` with `map` states the transformation
in one place.

## Known problems

- Only the standard `Vec::push` and `VecDeque::push_back` trigger. Other
  collections, such as `HashSet::insert`, and local types with a `push` method
  are ignored.
- The lint skips bodies that contain `?`, `.await`, `break`, `continue`, or
  `return`, and pushed values that read the target collection.
- A target produced by a call or an index, such as `target().push(value)`, is
  ignored because the loop evaluates it once per item.
- The automatic fix applies only when the pushed value is the loop variable
  without a coercion and the loop is a statement or a block tail. Other loops,
  such as a loop in a match arm, get help without a fix. The fix removes
  comments inside the loop.

## Example

```rust
fn append_all(output: &mut Vec<i32>, values: Vec<i32>) {
    for value in values {
        output.push(value);
    }
}
```

## Use instead

```rust
fn append_all(output: &mut Vec<i32>, values: Vec<i32>) {
    output.extend(values);
}
```
