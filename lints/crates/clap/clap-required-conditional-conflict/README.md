# clap-required-conditional-conflict

## What it does

Checks for a `clap::Arg` method chain whose final `required` setting is
`true` and that also calls a conditional requirement method:
`required_if_eq`, `required_if_eq_any`, `required_if_eq_all`,
`required_unless_present`, `required_unless_present_any`, or
`required_unless_present_all`.

## Why is this bad?

An argument cannot be required always and required only under a condition.
Clap panics in debug builds when the command is built.

## Known problems

The lint checks only one method chain. It does not check an `Arg` changed in
later statements. `required_if_eq` and `required_unless_present` count with any
arguments. The `_any` and `_all` forms count only when their argument is a
nonempty array literal, so conditions passed through variables are missed.

## Example

```rust
use clap::Arg;

fn config_arg() -> Arg {
    Arg::new("config")
        .long("config")
        .required(true)
        .required_if_eq("mode", "custom")
}
```

## Use instead

```rust
use clap::Arg;

fn config_arg() -> Arg {
    Arg::new("config")
        .long("config")
        .required_if_eq("mode", "custom")
}
```
