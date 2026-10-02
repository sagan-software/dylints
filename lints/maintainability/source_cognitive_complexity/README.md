# source_cognitive_complexity

## What it does

Checks the Cognitive Complexity of each function, method, and closure, and
warns when the score is 15 or more.

Each `if`, loop, and `match` adds 1 plus its nesting depth. An `if` at the top
level adds 1, an `if` inside it adds 2, and deeper nesting adds its current depth.
An `else if` continues the
chain without extra nesting. Each `&&` and `||` adds 1 with no nesting penalty.
A closure gets its own score. The limit of 14 follows
[PMD's Cognitive Complexity rule](https://docs.pmd-code.org/latest/pmd_rules_java_design.html#cognitivecomplexity),
which is stricter than the Clippy default of 25.

## Why is this bad?

Deep nesting forces a reader to keep several conditions in mind before reaching
the code that does the work. Reviewers can miss which condition controls a side
effect, and an `else` far from its `if` is easy to misread.

## Known problems

The score is structural. It cannot tell whether a condition has an obvious
meaning. A long flat `else if` chain stays below the limit even when a lookup
table would read better.

An `else` adds nothing, and `break` or `continue` to a label adds nothing.
Recursion lies outside the score. The score excludes control flow a macro
generates but includes expressions written as macro arguments. `.await` adds
nothing. Functions that a macro generates also lie outside the score.

## Example

```rust
fn report(a: bool, b: bool, c: bool, d: bool, e: bool, f: bool) {
    if a {
        if b {
            if c {
                if d {
                    if e {
                        if f {
                            println!("all checks passed");
                        }
                    }
                }
            }
        }
    }
}
```

The six nested `if` expressions score 1 + 2 + 3 + 4 + 5 + 6 = 21.

## Use instead

Combine the conditions or return early so the main path stays flat.

```rust
fn report(a: bool, b: bool, c: bool, d: bool, e: bool, f: bool) {
    if a && b && c && d && e && f {
        println!("all checks passed");
    }
}
```
