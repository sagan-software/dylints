# ad_hoc_as_ref

## What it does

Checks inherent methods named `as_*` or `get_*` that take only a `&self` or
`&mut self` receiver and return a reference. A shared reference suggests
`AsRef`, and a mutable reference suggests `AsMut`. Names that start with
`get_or_` are skipped, and so are types that already implement the matching
trait for the returned type.

## Why is this bad?

A custom accessor cannot be used where generic code asks for `impl AsRef<T>`.
Callers must learn the local method name instead of using the standard trait.

## Known problems

The lint checks only the name and signature. It warns on any `as_*` or `get_*`
accessor, even when the returned value is one field among several rather than
the type's main borrowed view. A type can implement `AsRef<T>` only once for
each `T`, so two accessors that return the same type both warn.

## Example

```rust
use std::path::{Path, PathBuf};

struct ConfigPath {
    path: PathBuf,
}

impl ConfigPath {
    fn as_path(&self) -> &Path {
        self.path.as_path()
    }
}
```

## Use instead

```rust
use std::path::{Path, PathBuf};

struct ConfigPath {
    path: PathBuf,
}

impl AsRef<Path> for ConfigPath {
    fn as_ref(&self) -> &Path {
        self.path.as_path()
    }
}
```
