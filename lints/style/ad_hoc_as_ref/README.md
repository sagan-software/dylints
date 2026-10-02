# ad_hoc_as_ref

## What it does

Checks methods named `as_*` or `get_*` that take only a `&self` or `&mut self`
receiver and return a reference. Names that start with `get_or_` and the
`as_ref` method of an `AsRef` implementation are skipped.

## Why is this bad?

A custom accessor cannot be used where generic code asks for `impl AsRef<T>`.
Callers must learn the local method name instead of using the standard trait.

## Known problems

The lint checks only the name and signature. It warns on any `as_*` or `get_*`
accessor, even when the returned value is one field among several rather than
the type's main borrowed view. It also warns on `as_mut_*` methods that return
`&mut T`, where `AsMut` fits better than `AsRef`, and on methods in trait
implementations whose names the trait fixes.

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
