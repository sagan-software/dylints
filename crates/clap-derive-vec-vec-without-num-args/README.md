# clap-derive-vec-vec-without-num-args

## What it does

Checks for a Clap-derived field of type `Vec<Vec<T>>` or `Option<Vec<Vec<T>>>`
that does not set `num_args`.

## Why is this bad?

Clap groups the values of a `Vec<Vec<T>>` field by occurrence of the argument.
Each occurrence takes the number of values set by `num_args`. Without
`num_args`, each occurrence takes one value, so every inner `Vec` holds a
single value and the grouping does nothing. Clap's derive reference says this
type needs `num_args` to be meaningful. Clap supports these field types only
with its `unstable-v5` feature.

## Known problems

The lint matches the type names `Vec` and `Option` as written. Like Clap, it
does not treat a qualified path such as `std::vec::Vec` or a type alias as a
`Vec`.

## Example

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    define: Vec<Vec<String>>,
}
```

## Use instead

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, num_args = 2)]
    define: Vec<Vec<String>>,
}
```
