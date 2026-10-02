# complicated_conditional

## What it does

Checks for `bool` expressions with too much inline work in `if` and `while`
conditions, match guards, `let` initializers, and plain `=` assignments.

### Scoring

The lint splits the expression at top-level `&&` and `||` and scores each term.
A method or function call adds 2, a field access 1, an index 2, and a comparison
1 plus the scores of its operands. A `!`, cast, or reference adds 1. An `if` or
`match` adds 4, a block 2, and an `if let` pattern 2. A named `bool` variable
scores 0.

Each inline closure argument adds the work inside it: 2 per call, 4 per branch,
`match`, or loop, and 1 per statement, binary operator, or assignment. One
closure adds at most 8.

A term with a score of 4 or more is complex. The lint warns when two terms are
complex, or when one term is complex and the total score reaches 8. An
expression without `&&` or `||` warns when its score reaches 8.

## Why is this bad?

A dense condition mixes data access, calls, and the branch decision in one
expression. The reader must evaluate all of it to learn what the branch tests.
Named `bool` bindings state each part of the decision and give each part a
place for a comment.

## Known problems

- The score is a heuristic. It can warn on a fluent API call chain that reads
  well, and it misses work hidden inside helper functions.
- Non-`bool` call arguments other than inline closures add nothing to the
  score.
- A condition that contains a macro call outside a closure body is ignored.
  Code removed by `cfg` is not checked.
- `const` and `static` initializers, `return` expressions, tuple destructuring,
  and compound assignments such as `|=` are not checked.
- Moving every term into a binding before the `if` evaluates all of them. That
  can change short-circuit behavior, side effects, borrows, `.await` points, or
  `?` propagation. In a `while` condition, a binding computed once is not
  re-evaluated on each iteration. Keep dependent terms inside the branch, or
  move them into a helper function. The lint emits help without an automatic
  fix.

## Example

```rust
fn ship(order: &Order, config: &Config) {
    if order.customer().address.is_valid() && config.shipping.is_enabled() {
        send(order);
    }
}
```

## Use instead

```rust
fn ship(order: &Order, config: &Config) {
    let has_valid_address = order.customer().address.is_valid();
    let is_shipping_enabled = config.shipping.is_enabled();
    if has_valid_address && is_shipping_enabled {
        send(order);
    }
}
```
