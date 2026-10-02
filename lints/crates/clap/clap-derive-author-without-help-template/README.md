# clap-derive-author-without-help-template

## What it does

Checks for a `#[command(author)]` or `#[command(author = ...)]` attribute on a
Clap-derived type or `Subcommand` variant that does not also set
`help_template`.

## Why is this bad?

Clap's default help template does not show the author. The author set by the
attribute never appears in `-h` or `--help` output unless a custom
`help_template` includes `{author}`. Code that adds the attribute to show the
author in help does nothing.

## Known problems

The lint warns when the template is set later through the builder API, such as
`Cli::command().help_template(...)`. It does not check whether the template
contains an author placeholder.

## Example

```rust
use clap::Parser;

#[derive(Parser)]
#[command(author)]
struct Cli {}
```

## Use instead

Add a template that contains `{author}`, or remove `author`.

```rust
use clap::Parser;

#[derive(Parser)]
#[command(author, help_template = "{about}\n\nBy {author}\n\n{usage}\n\n{all-args}")]
struct Cli {}
```
