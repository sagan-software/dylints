# clap-index-on-option

## What it does

Checks for a `clap::Arg` method chain that calls `index` on an argument that
also has a `long` or `short` name.

## Why is this bad?

`index` sets the position of a positional argument. An argument with a `long`
or `short` name is an option, and options have no position. Clap panics in
debug builds when the command is built.

## Known problems

The lint checks only one method chain. It does not check an `Arg` changed in
later statements.

## Example

```rust
use clap::Arg;

fn input_arg() -> Arg {
    Arg::new("input").long("input").index(1)
}
```

## Use instead

Remove `index` for an option, or remove `long` and `short` for a positional
argument.

```rust
use clap::Arg;

fn input_arg() -> Arg {
    Arg::new("input").long("input")
}
```
