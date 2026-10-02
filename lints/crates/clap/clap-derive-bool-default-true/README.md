# clap-derive-bool-default-true

## What it does

Checks for a `bool` field in a Clap-derived type that uses the
`ArgAction::SetTrue` action, inferred or explicit, and sets
`default_value_t = true` or `default_value = "true"`.

## Why is this bad?

`ArgAction::SetTrue` sets the field to `true` when the flag is present. With a
default of `true`, the field is `true` whether or not the flag is present, so
the flag does nothing and users cannot turn the behavior off.

## Known problems

The lint checks only the literal defaults `default_value_t = true` and
`default_value = "true"`. It does not check a default computed by an
expression.

## Example

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value_t = true)]
    color: bool,
}
```

## Use instead

Remove the default for a flag that turns a behavior on. For a flag that turns a
default-on behavior off, use a negative name and `ArgAction::SetFalse`.

```rust
use clap::{ArgAction, Parser};

#[derive(Parser)]
struct Cli {
    #[arg(long = "no-color", action = ArgAction::SetFalse, default_value_t = true)]
    color: bool,
}
```
