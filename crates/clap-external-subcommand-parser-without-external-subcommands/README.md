# clap-external-subcommand-parser-without-external-subcommands

## What it does

Checks for a `clap::Command` method chain that calls
`external_subcommand_value_parser` but never calls
`allow_external_subcommands(true)`.

## Why is this bad?

Clap uses the external subcommand value parser only for the arguments of an
external subcommand. Without `allow_external_subcommands(true)`, clap rejects
unknown subcommands, so the parser never runs. The code suggests support for
external subcommands that the command does not have.

## Known problems

The lint checks only one method chain. It warns when code calls `allow_external_subcommands(true)` on the same `Command` in a later statement. It also warns when `allow_external_subcommands` gets a value other
than the literal `true`, even if that value is `true` at runtime.

## Example

```rust
use std::ffi::OsString;

use clap::{value_parser, Command};

fn cli() -> Command {
    Command::new("app").external_subcommand_value_parser(value_parser!(OsString))
}
```

## Use instead

Enable external subcommands, or remove the parser if the command does not need
them.

```rust
use std::ffi::OsString;

use clap::{value_parser, Command};

fn cli() -> Command {
    Command::new("app")
        .allow_external_subcommands(true)
        .external_subcommand_value_parser(value_parser!(OsString))
}
```
