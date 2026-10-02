# rumdl_doc_comments

## What it does

Runs the [rumdl](https://github.com/rvben/rumdl) Markdown rules on doc comments
attached to the crate, items, associated items, fields, and enum variants. It
reports the first rule violation in each doc comment block.

The rules come from the nearest project configuration that `rumdl check` would
find: `.rumdl.toml`, `rumdl.toml`, `.config/rumdl.toml`, `pyproject.toml` with
`[tool.rumdl]`, or a markdownlint file. The search starts in the directory of
the documented source file and stops at the repository root, marked by `.git`.
Without a project configuration, rumdl's defaults apply.

## Why is this bad?

Rustdoc renders doc comments as Markdown. Broken emphasis, missing blank lines
around headings, and similar mistakes render incorrectly or inconsistently, and
nothing else in a normal build reports them.

## Known problems

The lint does not read the user-level rumdl configuration, and a project
configuration that fails to load falls back to rumdl's defaults without a
warning. The default line-length rule (MD013) reports doc lines longer than 80
characters. The lint reports only the first violation in each block, so fixing
one can reveal the next. It offers a machine-applicable fix only for `///` and
`//!` comments with LF line endings. Block doc comments, `#[doc = "..."]`
attributes, and generated docs receive a warning on the whole item without a
fix.

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
