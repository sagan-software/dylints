# rumdl_doc_comments

## What it does

Runs the [rumdl](https://github.com/rvben/rumdl) Markdown rules on the doc
comments of the crate, items, associated items, fields, and enum variants, and
reports the first rule violation in each doc comment block.

## Why is this bad?

Rustdoc renders doc comments as Markdown. Broken emphasis, missing blank lines
around headings, and similar mistakes render incorrectly or inconsistently, and
nothing else in a normal build reports them.

## Known problems

The lint uses rumdl's default configuration and ignores any rumdl
configuration file in the project. The default line-length rule (MD013)
reports doc lines longer than 80 characters. Only the first violation in each
block is reported, so fixing one can reveal the next. A machine-applicable fix
is offered only for `///` and `//!` comments with LF line endings; block doc
comments, `#[doc = "..."]` attributes, and generated docs get a warning on the
whole item without a fix.

## Example

```rust
/// Do not write * emphasis * with spaces.
pub fn render_docs() {}
```

## Use instead

```rust
/// Do not write *emphasis* with spaces.
pub fn render_docs() {}
```
