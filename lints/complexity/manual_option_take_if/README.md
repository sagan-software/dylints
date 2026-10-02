# manual_option_take_if

## What it does

Checks for `if option.as_ref().is_some_and(predicate) { option.take() } else { None }`
on a standard `Option`, where both branches name the same receiver.

## Why is this bad?

The receiver is written twice, so a later edit can test one `Option` and take
another. `Option::take_if` tests and takes the value in one call.

## Known problems

- The lint compares the receivers by source text. It only matches the
  `as_ref().is_some_and(..)` condition and a literal `None` in the `else`
  branch. Conditions such as `matches!(option, Some(..))` are ignored.
- The `take_if` predicate receives `&mut T` instead of `&T`, so the closure can
  need changes. The lint emits help without an automatic fix.

## Example

```rust
fn take_positive(option: &mut Option<i32>) -> Option<i32> {
    if option.as_ref().is_some_and(|value| *value > 0) {
        option.take()
    } else {
        None
    }
}
```

## Use instead

```rust
fn take_positive(option: &mut Option<i32>) -> Option<i32> {
    option.take_if(|value| *value > 0)
}
```
