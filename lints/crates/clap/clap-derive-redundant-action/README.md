# clap-derive-redundant-action

## What it does

Checks for an `action = ...` setting on a Clap-derived field when the action is
the one Clap already infers from the field type:

- `bool` infers `ArgAction::SetTrue`.
- `T`, `Option<T>`, and `Option<Option<T>>` infer `ArgAction::Set`.
- `Vec<T>`, `Option<Vec<T>>`, `Vec<Vec<T>>`, and `Option<Vec<Vec<T>>>` infer
  `ArgAction::Append`.

## Why is this bad?

The setting does not change field parsing. It makes the field look
like it has special behavior, and it can go stale when the field type changes.

## Known problems

The lint matches only the paths `ArgAction::X`, `clap::ArgAction::X`, and
`::clap::ArgAction::X`. It does not report an action reached through a
constant, a re-export, or another expression.

## Example

```rust
use clap::{ArgAction, Parser};

#[derive(Parser)]
struct Cli {
    #[arg(long, action = ArgAction::SetTrue)]
    verbose: bool,
}
```

## Use instead

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    verbose: bool,
}
```
