# clap-last-option

## What it does

Checks for a `clap::Arg` method chain that calls `last(true)` on an argument
that also has a `long` or `short` name.

## Why is this bad?

`last` applies only to positional arguments. An argument with a `long` or
`short` name is an option, so clap panics in debug builds when the command is
built.

## Known problems

The lint checks only one method chain. It does not check an `Arg` changed in
later statements. It does not check a `last` call whose argument is not the
literal `true`. A `long` or `short` name counts only when it is a string or
character literal, so names passed through variables or constants are missed.

## Example

```rust
use clap::Arg;

fn input_arg() -> Arg {
    Arg::new("input").long("input").last(true)
}
```

## Use instead

Remove `last(true)` for an option, or remove `long` and `short` for a
positional argument.

```rust
use clap::Arg;

fn input_arg() -> Arg {
    Arg::new("input").last(true)
}
```
