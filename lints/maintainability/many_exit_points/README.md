# many_exit_points

## What it does

Counts the `return` expressions and `?` operators in each function, method, and
closure, and warns when there are more than 4. Reaching the end of the body
does not count. The limit of 4 follows the agent-feedback threshold in the
[big-code-analysis threshold guide](https://dekobon.github.io/big-code-analysis/recipes/thresholds.html).

## Why is this bad?

Each exit is a place where the function can stop with a different result. Many
exits often mean that several independent checks share one function. A reader
must trace every exit to know which state the function leaves behind.

## Known problems

Guard clauses can make a function easier to read, and the lint counts them the
same as other exits. The lint does not count exits a macro generates, such as the
`return` in `anyhow::bail!`. It counts a `?` written as a macro argument. It
ignores `panic!` and `break`. A `return` or `?` inside a closure counts toward
the closure, not the enclosing function. The lint does not check a function a
macro generates.

## Example

```rust
fn pick(values: [Option<u8>; 3]) -> Option<u8> {
    let first = values[0]?;
    let second = values[1]?;
    let third = values[2]?;
    if first == 0 {
        return None;
    }
    if second == 0 {
        return None;
    }
    Some(third)
}
```

The three `?` operators and two `return` expressions give five exits.

## Use instead

Move a repeated check into a separate fallible function.

```rust
fn nonzero(value: Option<u8>) -> Option<u8> {
    value.filter(|&value| value != 0)
}

fn pick(values: [Option<u8>; 3]) -> Option<u8> {
    nonzero(values[0])?;
    nonzero(values[1])?;
    values[2]
}
```
