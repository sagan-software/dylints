# runtime_env_read

## What it does

Checks calls to `std::env::var`, `std::env::var_os`, `std::env::vars`, and
`std::env::vars_os` outside configuration, startup, build-script, and test
code.

## Why is this bad?

A function that reads the environment has a hidden input that its signature does
not show. Callers cannot pass a different value, and tests that set environment
variables affect each other because all tests share one process environment.
Parsing and default rules for the variable spread to every read site.

## Known problems

The lint allows a function when any of these conditions applies:

- The function's name is `main`.
- Its name or the name of any enclosing module, type, or impl contains one of
  these words: `cli`, `config`, `configuration`, `bootstrap`, `settings`,
  `setting`, `env`, or `environment`.
- Its path contains the word `test`, `tests`, or `testing`.
- It is a `#[test]` function, or it or an enclosing item has a `cfg` that
  requires `test`, such as `#[cfg(test)]` or `#[cfg(all(test, unix))]`.
- It is in `build.rs`, or the file name or its parent directory contains one of
  the allowed words, such as `config.rs` or `settings/mod.rs`.

The lint compares ASCII letters without regard to case. ASCII letters and
digits form tokens. Lower-to-upper CamelCase and acronym-to-word transitions
split tokens. Other non-alphanumeric ASCII characters separate tokens. Each
non-ASCII Unicode scalar acts as a separator. This allows `load_config`,
`AppConfig::load`, and `APIConfig::load`.

The lint judges a read inside a closure, such as a `LazyLock` initializer, by
the item that owns the closure. It does not follow reads through wrapper
functions.

## Example

```rust
fn handle_request() {
    let endpoint = std::env::var("API_ENDPOINT").unwrap();
    send(&endpoint);
}
```

## Use instead

```rust
struct AppConfig {
    endpoint: String,
}

fn load_config() -> AppConfig {
    AppConfig {
        endpoint: std::env::var("API_ENDPOINT").unwrap(),
    }
}

fn handle_request(config: &AppConfig) {
    send(&config.endpoint);
}
```
