# clap-allow-negative-numbers-without-num-args

## What it does

Checks for a `clap::Arg` method chain that calls `allow_negative_numbers(true)`
but never calls `num_args`.

## Why is this bad?

`allow_negative_numbers` lets Clap treat a value such as `-5` as this
argument's value instead of a short option. Clap requires the argument to take
values, and the number of tokens it can take decides how much of the command
line it can capture. Without `num_args`, the action supplies that count, but
this setting does not show it. If the action is a flag action such as
`ArgAction::SetTrue`, clap panics in debug builds.

## Known problems

The lint checks only one method chain. It warns when a later statement calls
`num_args` on the same `Arg`. It does not warn when the final `action` call
is `ArgAction::Set` or `ArgAction::Append`, because that action already makes
the argument take one value. It checks only the final `allow_negative_numbers` call, and
only when its argument is the literal `true`.

## Example

```rust
use clap::Arg;

fn offset_arg() -> Arg {
    Arg::new("offset").long("offset").allow_negative_numbers(true)
}
```

## Use instead

```rust
use clap::Arg;

fn offset_arg() -> Arg {
    Arg::new("offset")
        .long("offset")
        .allow_negative_numbers(true)
        .num_args(1)
}
```
