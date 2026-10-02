# clap-derive-redundant-value-parser

## What it does

Checks for a `value_parser = value_parser!(T)` setting on a Clap-derived field
when `T` is the field type with any `Option` and `Vec` wrappers removed.

## Why is this bad?

Clap's derive already calls `value_parser!(T)` for the field type when no
parser is set. The setting does not change parsing, and it can go stale when
the field type changes.

## Known problems

The lint compares source text. It does not report `value_parser!(u16)` on a
field whose type is an alias of `u16`, or a parser reached through a constant
or a re-export. Fields marked `value_enum` are skipped.

## Example

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, value_parser = clap::value_parser!(u16))]
    port: u16,
}
```

## Use instead

Remove the setting. Keep `value_parser` when it adds a constraint, such as
`value_parser!(u16).range(1..)`, or uses a custom parse function.

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    port: u16,
}
```
