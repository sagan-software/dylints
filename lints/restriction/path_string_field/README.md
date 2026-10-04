# path_string_field

## What it does

Checks for named fields whose type is `String` or `&str` and whose name contains
a filesystem word. The words are `path`, `paths`, `dir`, `dirs`, `directory`,
`directories`, `folder`, `folders`, `file`, `filename`, and `filepath`, matched
between underscores and with case.

## Why is this bad?

A string cannot hold every path the operating system allows, and it invites
joining paths with `format!` and `/`. `PathBuf` and `Path` keep non-UTF-8 paths
and provide `join`, `parent`, and `extension`.

## Known problems

The compiler resolves type aliases before the lint peels up to eight
consecutive standard `Option` layers. Longer chains, local `Option`
lookalikes, and user-defined wrappers remain opaque. After peeling, it checks only `String` and
`&str`, including type aliases and `use` renames. It does not inspect
`Vec<String>`, `Box<str>`, or `Cow<'_, str>`.

It warns on any field that contains one of the words, including display text
such as `display_path_label` or `file_label`. It skips a field whose name also
contains a word for a non-filesystem path: `api`, `crate`, `def`, `endpoint`,
`http`, `import`, `item`, `json`, `key`, `mod`, `module`, `public`, `query`,
`route`, `symbol`, `type`, `uri`, `url`, `web`, or `xpath`. So `module_path`
and `url_path` do not warn, and neither does a `public_file` that is a real
file. It does not flag other path names, such as `location` or `cachedir`.

## Example

```rust
struct CacheConfig<'a> {
    cache_dir: String,
    manifest_path: &'a str,
}
```

## Use instead

```rust
use std::path::{Path, PathBuf};

struct CacheConfig<'a> {
    cache_dir: PathBuf,
    manifest_path: &'a Path,
}
```
