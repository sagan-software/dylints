# unnecessary-module-directory

## What it does

Checks for module files named `mod.rs` that are the only file in their
directory, including its subdirectories. Empty subdirectories do not count as
files.

## Why is this bad?

The directory adds a level to the source tree but holds nothing beside
`mod.rs`. Readers open a directory to find one file, and editor tabs show
`mod.rs` instead of the module name. A plain `worker.rs` holds the same module.

## Known problems

The lint reads the directory from disk. Any other entry keeps the directory,
including non-Rust files such as `README.md` or test data, symbolic links, and
entries it cannot read. It checks only `mod.rs` files compiled into the crate
under the package directory, and never the crate root.

It also warns when `mod.rs` reaches the `large_rust_file` limits. In that case
the help text says to keep the directory and split the module into child
modules instead of moving it.

## Example

The abbreviated snippets require their separate module files and omitted implementation.

```rust,ignore
// src/lib.rs
mod worker;

// src/worker/mod.rs, the only file in src/worker/
pub fn run() {}
```

## Use instead

Move the module to a file named after it and remove the directory:

```rust,ignore
// src/lib.rs
mod worker;

// src/worker.rs
pub fn run() {}
```
