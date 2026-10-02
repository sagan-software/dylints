# runtime_env_read

## What it does

Checks for calls to `std::env::var` and `std::env::var_os` outside
configuration, startup, build-script, and test code.

## Why is this bad?

A function that reads the environment has a hidden input that its signature does
not show. Callers cannot pass a different value, and tests that set environment
variables affect each other because the environment is shared by the whole
process. Parsing and default rules for the variable spread to every read site.

## Known problems

A function is allowed when any of these hold:

- It is named `main`.
- Its name or any module in its path contains the word `cli`, `config`,
  `configuration`, `bootstrap`, `settings`, `setting`, `env`, or `environment`.
  Words are split on non-alphanumeric characters and matched with case, so
  `load_config` is allowed but `AppConfig::load` is not.
- Its path contains the word `test`, `tests`, or `testing`.
- It has `#[test]`, or a `cfg` or `cfg_attr` attribute whose text contains
  `test`.
- It is in `build.rs`, or the file name or its parent directory contains one of
  the allowed words, such as `config.rs` or `settings/mod.rs`.

It does not check reads inside closures, so a read in a `LazyLock` initializer
is not reported. It does not check `std::env::vars` or reads through wrapper
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
