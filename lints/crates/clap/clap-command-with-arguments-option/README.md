# clap-command-with-arguments-option

## What it does

Checks for a `clap::Arg` method chain that sets
`value_hint(ValueHint::CommandWithArguments)` on an argument that also has a
`long` or `short` name.

## Why is this bad?

Clap allows `ValueHint::CommandWithArguments` only on a positional argument. An
argument with a `long` or `short` name is an option, so clap panics in debug
builds when the command is built.

## Known problems

The lint checks only one method chain. It does not check an `Arg` changed in
later statements. A `long` or `short` name counts when its final call passes any value
other than `None`. A name passed as an `Option` value computed at runtime does
not count.

## Example

```rust
use clap::{Arg, ValueHint};

fn command_arg() -> Arg {
    Arg::new("command")
        .long("command")
        .value_hint(ValueHint::CommandWithArguments)
}
```

## Use instead

Remove the `long` and `short` names to make the argument positional. If it must
stay an option, use a value hint that applies to one value, such as
`ValueHint::CommandName`.

```rust
use clap::{Arg, ValueHint};

fn command_arg() -> Arg {
    Arg::new("command").value_hint(ValueHint::CommandWithArguments)
}
```
