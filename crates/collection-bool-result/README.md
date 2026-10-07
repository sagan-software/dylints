# collection-bool-result

## What it does

Checks function return types, including the output of an `async fn`, for a
two-element tuple of a collection and a `bool`, in either order. The tuple
can also be inside `Result` or `Option`. The collections are arrays and the
standard `Vec`, `VecDeque`, `HashMap`, `HashSet`, `BTreeMap`, and `BTreeSet`
types.

## Why is this bad?

The type does not say what the `bool` means or which combinations can occur.
A caller must guess whether `(vec![], true)` is valid. An enum names each
outcome and makes invalid combinations impossible to return.

## Known problems

Some flags are independent of the collection, such as a `truncated` marker
next to a list of lines. The lint warns in that case too.

The lint misses tuples with more than two elements, structs with a
collection field and a `bool` field, references such as `(&[T], bool)`, and
collections from other crates.

## Example

```rust
struct Action;

fn pending_actions() -> Option<(Vec<Action>, bool)> {
    None
}
```

## Use instead

```rust
struct Action;

enum PendingActions {
    Ready(Vec<Action>),
    Waiting(Vec<Action>),
}

fn pending_actions() -> Option<PendingActions> {
    None
}
```
