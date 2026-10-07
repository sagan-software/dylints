# clap-multicall-no-binary-name

## What it does

Checks for a `clap::Command` method chain whose final `multicall` and
`no_binary_name` settings are both `true`.

## Why is this bad?

Clap cannot combine `multicall` with `no_binary_name` because the two settings
read the first command-line token in different ways. Clap panics in debug builds when code builds the command.

## Known problems

The lint checks only one method chain. It does not check a `Command` changed in
later statements. It does not check a setting whose argument is not a literal
`true` or `false`.

## Example

```rust
use clap::Command;

fn cli() -> Command {
    Command::new("busybox").multicall(true).no_binary_name(true)
}
```

## Use instead

```rust
use clap::Command;

fn cli() -> Command {
    Command::new("busybox").multicall(true)
}
```
