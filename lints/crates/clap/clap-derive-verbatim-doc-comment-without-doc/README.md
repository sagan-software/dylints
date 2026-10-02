# clap-derive-verbatim-doc-comment-without-doc

## What it does

Checks for `verbatim_doc_comment` in a `#[command(...)]` or `#[arg(...)]`
attribute on a Clap-derived type, variant, or field that has no doc comment.

## Why is this bad?

`verbatim_doc_comment` changes only how Clap turns a doc comment into `about`
or `help` text. With no doc comment, the setting does nothing. It can also hide
a help message that was deleted by mistake.

## Known problems

Any `doc` attribute counts as a doc comment, including `#[doc = ""]` and
`#[doc = include_str!(...)]`. The lint does not check that the text is
nonempty.

## Example

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, verbatim_doc_comment)]
    output: String,
}
```

## Use instead

Add the doc comment whose formatting must be kept, or remove
`verbatim_doc_comment`.

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// Output path.
    ///
    /// Use `-` for standard output.
    #[arg(long, verbatim_doc_comment)]
    output: String,
}
```
