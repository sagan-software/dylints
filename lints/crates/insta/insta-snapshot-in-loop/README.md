# insta-snapshot-in-loop

## What it does

Checks for Insta snapshot assertion macros, such as `assert_snapshot!` and
`assert_debug_snapshot!`, inside a `for`, `while`, or `loop` body that is not
wrapped in `insta::allow_duplicates!`.

## Why is this bad?

Each pass through the loop runs the same assertion. For a file snapshot with
no explicit name, Insta stores a new numbered snapshot on each pass, tied to
the iteration order. An inline snapshot repeated in a loop fails. Inside
`allow_duplicates!`, Insta checks that every pass produces the same snapshot.

## Known problems

The lint warns even when each pass uses a different snapshot name. It does not
check assertions inside closures passed to iterator methods such as
`for_each`, or inside functions called from a loop.

## Example

```rust
fn trims_whitespace() {
    for input in [" a", "a ", " a "] {
        insta::assert_snapshot!(input.trim());
    }
}
```

## Use instead

```rust
fn trims_whitespace() {
    insta::allow_duplicates! {
        for input in [" a", "a ", " a "] {
            insta::assert_snapshot!(input.trim());
        }
    }
}
```
