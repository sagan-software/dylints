# clap-require-equals-without-num-args

## What it does

Checks for a `clap::Arg` method chain that calls `require_equals(true)` but
never calls `num_args`.

## Why is this bad?

`require_equals` makes users write the value after `=`, as in `--color=always`.
Clap requires the argument to take values, and it panics in debug builds if
the argument must take more than one value. Without `num_args`, the value count
comes from the action and is not visible where the setting is made. If the
action is a flag action such as `ArgAction::SetTrue`, clap panics in debug
builds.

## Known problems

The lint checks only one method chain. It warns when `num_args` is called on the
same `Arg` in a later statement. It does not warn when the final `action` call
is `ArgAction::Set` or `ArgAction::Append`, because that action already makes
the argument take one value. It checks only the final `require_equals` call, and
only when its argument is the literal `true`.

## Example

```rust
use clap::Arg;

fn color_arg() -> Arg {
    Arg::new("color").long("color").require_equals(true)
}
```

## Use instead

```rust
use clap::Arg;

fn color_arg() -> Arg {
    Arg::new("color")
        .long("color")
        .require_equals(true)
        .num_args(1)
}
```
