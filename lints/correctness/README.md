# correctness

Rust review lints for code that is likely to behave incorrectly.

## Lints

- [`logged_error_continue`](logged_error_continue): flags `Result`-returning
  functions that catch an error, log it, and continue instead of
  propagating it.
- [`panic_in_main`](panic_in_main): flags obvious panicking operations in
  infallible `fn main()` bodies in favor of returning `Result`.
