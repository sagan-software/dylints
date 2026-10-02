# clap-trailing-var-arg-without-num-args

## What it does

Checks for a `clap::Arg` method chain that calls `trailing_var_arg(true)` but
never calls `num_args`.

## Why is this bad?

`trailing_var_arg` makes the last positional argument capture the rest of the
command line, so the argument must accept multiple values. Without `num_args`,
the argument takes one value by default, and clap panics in debug builds when
code builds the command.

## Known problems

The lint checks only one method chain. It warns when a later statement calls
`num_args` on the same `Arg`. It does not warn when the final `action` call
is `ArgAction::Append`, because that action already lets the argument take
multiple values. It checks only the final `trailing_var_arg` call, and only when
its argument is the literal `true`.

## Example

```rust
use clap::Arg;

fn command_args() -> Arg {
    Arg::new("args").trailing_var_arg(true)
}
```

## Use instead

```rust
use clap::Arg;

fn command_args() -> Arg {
    Arg::new("args").trailing_var_arg(true).num_args(1..)
}
```
