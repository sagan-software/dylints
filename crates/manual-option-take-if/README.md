# manual-option-take-if

## What it does

Checks for `if option.as_ref().is_some_and(predicate) { option.take() } else { None }`
on a standard `Option`, where both calls use the same local place: a local
binding, a field of one, or a `*` projection of one.

## Why is this bad?

The code writes the receiver twice. A later edit can test one `Option` and take
another. `Option::take_if` tests and takes the value in one call.

## Known problems

- Only the `as_ref().is_some_and(..)` condition and a literal `None` in the
  `else` branch trigger. Conditions such as `matches!(option, Some(..))` are
  ignored.
- The lint ignores a receiver outside a local place, such as `holder().slot`.
- The lint ignores a receiver that uses an overloaded `Deref` or `DerefMut`
  adjustment. Repeating that receiver can run user code twice.
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
