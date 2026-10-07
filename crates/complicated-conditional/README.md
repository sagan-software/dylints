# complicated-conditional

## What it does

Checks for `bool` expressions with too much inline work in `if` and `while`
conditions and match guards.

### Scoring

The lint splits the condition at top-level `&&` and `||` and scores each term.
A method or function call adds 2 plus its receiver or callee chain and boolean
argument scores. An index in a receiver or comparison operand adds 2 plus its
receiver chain score. A comparison adds 1 plus the scores of its operands.
A field access adds nothing on a variable or on a field of a
variable, such as `self.config.is_enabled`. On any other value, such as
`order.customer().address`, it adds 1.

A `!`, cast, or reference adds 1. An `if` or `match` adds 4, a block 2, and an
`if let` pattern 2. A `match` inside a call chain or comparison operand adds 3.
A `?` or `.await` adds nothing, and the lint scores the expression before it as
part of the chain. A named `bool` variable scores 0.

Each inline closure argument adds its work minus 2. Work counts 2 per call, 4
per branch, `match`, loop, or nested closure, and 1 per statement, binary
operator, or assignment, up to 8. A closure with one call or one comparison
adds nothing, and one closure adds at most 6.

A term with a score of 6 or more is complex. The lint warns when two terms are
complex, or when one term is complex and the total score reaches 9. A condition
without `&&` or `||` warns when its score reaches 9.

## Why is this bad?

A dense condition puts data access, calls, and the branch decision in one
expression. The reader must evaluate all of it to learn what the branch tests.
Named `bool` bindings state each part of the decision and give each part a
place for a comment.

## Known problems

- The score is a heuristic. It can warn on a fluent API call chain that reads
  well, and it misses work hidden inside helpers.
- Non-`bool` call arguments other than inline closures add nothing to the
  score.
- The lint ignores a condition that contains a macro call outside a closure
  body. It also skips code that `cfg` removes.
- The lint skips `let` initializers and assignments because the binding already
  names the result. It also skips `const` and `static` initializers and `return`
  expressions.
- Moving every term into a binding before the `if` evaluates all of them. That
  can change short-circuit behavior, side effects, borrows, `.await` points, or
  `?` propagation. In a `while` condition, a binding computed once is not
  re-evaluated on each iteration. Keep dependent terms inside the branch, or
  move them into a named helper. The lint emits help without an automatic
  fix.

## Example

```rust
fn accepts(values: &[i32], expected: usize) -> bool {
    if values.iter().filter(|value| **value > 0).count() == expected
        && values.iter().all(|value| value.abs() < 100 && *value % 2 == 0)
    {
        return true;
    }
    false
}
```

## Use instead

```rust
fn accepts(values: &[i32], expected: usize) -> bool {
    let has_expected_count = values.iter().filter(|value| **value > 0).count() == expected;
    let all_values_fit = || {
        values.iter().all(|value| value.abs() < 100 && *value % 2 == 0)
    };
    has_expected_count && all_values_fit()
}
```
