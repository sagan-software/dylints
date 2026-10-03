# vec_front_removal_in_loop

## What it does

Checks for resolved `Vec::remove(0)` calls inside loops that may execute more
than once. It follows one direct local receiver binding declared outside the
loop and recognizes the standard `Vec` and a literal zero index. It skips
element types whose resolved layout is zero-sized.

## Why is this bad?

Removing the first element shifts every remaining element to preserve order.
If a loop drains the same vector through repeated front removals, total shifts
can grow quadratically.

## Known problems

The lint does not follow calls into helper functions, closure bodies, vector
fields, or computed receiver expressions. It skips bindings declared or
directly assigned inside the loop. It also skips a direct final `break`, literal
singleton arrays, and standard literal ranges with bounds that prove at most
one item. Other iterators and control flow may still execute at most once. Keep
contiguous storage when the workload requires it. The lint does not infer a
`Vec`'s current length from its initializer or earlier control flow, so it can
warn when a loop removes a one-element vector once, even though that removal
shifts no tail elements.

When the element layout is unavailable, including a generic `T`, the lint keeps
the warning. Such a generic function can be instantiated with a zero-sized type.

## Example

```rust
fn drain_front(mut values: Vec<String>) {
    while !values.is_empty() {
        values.remove(0);
    }
}
```

## Use instead

```rust
fn drain_front(values: Vec<String>) {
    values.into_iter().for_each(drop);
}
```

When this loop repeatedly removes elements, consume the vector or use
`VecDeque::pop_front` if it must remain a queue.
