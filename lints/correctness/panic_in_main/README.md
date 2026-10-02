# panic_in_main

## What it does

Checks the crate's entry `main` function, when it does not return `Result`, for
calls to the standard `panic!`, `todo!`, `unimplemented!`, `unreachable!`,
`assert!`, `assert_eq!`, and `assert_ne!` macros, and to `Option` or `Result`
`.unwrap()` and `.expect(..)`.

## Why is this bad?

A panic in `main` prints a panic message and a backtrace hint instead of an
error message, and it exits with code 101. Returning `Result` from `main` lets
`?` report startup failures such as a missing file as ordinary errors.

## Known problems

Macros and methods are matched by their resolved definitions, so local macros
or methods with the same names are not reported. Other panicking operations,
such as indexing, slicing, `.unwrap_err()`, or `debug_assert!`, are not
reported.

A `main` that returns a non-`Result` type, such as `std::process::ExitCode`,
is treated as infallible and is checked.

The lint does not look inside closures, `async` blocks, or functions that
`main` calls. It skips `main` in `build.rs` files and functions marked
`#[test]`.

## Example

```rust
fn main() {
    let config = std::fs::read_to_string("config.toml").expect("config exists");
    println!("{config}");
}
```

## Use instead

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = std::fs::read_to_string("config.toml")?;
    println!("{config}");
    Ok(())
}
```
