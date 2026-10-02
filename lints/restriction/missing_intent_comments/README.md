# missing_intent_comments

## What it does

Checks for functions and methods whose body has five or more statements and
fewer `//` comments than one per five statements, rounded up. A body with 11
statements needs 3 comments. Trailing expressions of blocks count as
statements, including those in nested `if`, `match`, and loop blocks. A macro
call counts as one statement no matter how many statements it expands to, and
statements written inside macro arguments count normally.

## Why is this bad?

A long body without comments shows what the code does but not why. Reviewers
must work out the reason for the ordering, the invariants, and the error
handling from the code alone, and a later edit can break an assumption that was
never written down.

## Known problems

The lint counts comment lines in the body's source text. A line counts when it
starts with `//`, which includes doc comments and commented-out code, or when it
contains ` //` after code. Block comments do not count. The lint does not judge
comment quality or check that comments are spread across the body.

Statements inside closures are not counted toward the enclosing function, and
closures are not checked on their own.

## Example

```rust
fn apply(value: u64) -> u64 {
    let doubled = value * 2;
    let adjusted = doubled + 1;
    let bounded = adjusted.min(100);
    let shifted = bounded + 2;
    let restored = shifted - 2;
    restored.saturating_sub(3)
}
```

## Use instead

```rust
fn apply(value: u64) -> u64 {
    // Normalize before applying the overflow policy.
    let doubled = value * 2;
    let adjusted = doubled + 1;
    let bounded = adjusted.min(100);
    let shifted = bounded + 2;
    let restored = shifted - 2;
    // Keep saturation last.
    restored.saturating_sub(3)
}
```
