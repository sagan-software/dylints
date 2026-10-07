# abc-size

## What it does

Checks the ABC size of each function, method, and closure, and warns when
`sqrt(assignments² + calls² + conditions²)` exceeds 25.

Assignments are `let` statements with an initializer, `=`, and compound
assignments such as `+=`. Calls are function, method, and closure calls.
Conditions are `if`, `while`, `for`, each match arm after the first, each match
guard, `&&`, and `||`. The diagnostic shows all three counts. The limit of 25
follows the agent-feedback threshold in the
[big-code-analysis threshold guide](https://dekobon.github.io/big-code-analysis/recipes/thresholds.html).

## Why is this bad?

A large ABC size marks a long function that does many separate steps, even when
it has few branches. Path metrics such as `cyclomatic_complexity` miss a
function made of 26 straight-line calls. A reader still has to follow every
step and every change of state.

## Known problems

Tuple struct and enum variant constructors such as `Some(value)` count as
calls. A plain `loop` and `?` add no conditions. Comparisons outside control
flow do not count, unlike some other ABC analyzers. The lint skips code a macro
generates but counts expressions written as macro arguments. It does not check
a function a macro generates.

## Example

```rust
fn step() {}

fn run_pipeline() {
    step(); step(); step(); step(); step(); step(); step(); step(); step();
    step(); step(); step(); step(); step(); step(); step(); step(); step();
    step(); step(); step(); step(); step(); step(); step(); step();
}
```

The 26 calls give an ABC size of `<0, 26, 0>` with magnitude 26.

## Use instead

Repeat work with a loop, or split the steps into focused functions.

```rust
fn step() {}

fn run_pipeline() {
    for _ in 0..26 {
        step();
    }
}
```

## Interpretation and sources

ABC counts assignments, branches as calls, and conditions. Its magnitude combines three dimensionless counts as `sqrt(A² + B² + C²)`. Inspect the components to distinguish mechanical work from branching. Compare values within the same analyzer profile. The Maintainability Index combines correlated size and complexity measurements; it is a separate report rather than this lint's score. [Metric definitions](https://dekobon.github.io/big-code-analysis/metrics.html).
