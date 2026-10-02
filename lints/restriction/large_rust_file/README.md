# large_rust_file

## What it does

Checks each Rust source file compiled into the crate. It warns when a file has
1,500 or more non-test lines or 2,000 or more total lines. Blank and comment
lines count. When both limits are reached, it reports only the total-line
violation.

A line is a test line when it belongs to an item marked `#[test]`, an attribute
whose last path segment is `test` such as `#[tokio::test]`, or a `#[cfg(...)]`
that requires `test`, such as `#[cfg(test)]` or `#[cfg(all(test, unix))]`. Test
lines count only toward the total limit.

## Why is this bad?

A file of this size usually holds several unrelated responsibilities. Reviewers
and editors must scroll through code that does not concern the change, and the
file becomes a frequent merge-conflict site.

## Known problems

The lint reads files from disk under the compiler's current directory. It skips
files outside that directory, files from inactive `cfg` modules, and files whose
path contains a directory named `support`, `toolchains`, or `rustlib`.

If `syn` cannot parse a file, every line counts as non-test. Test helpers
without a test attribute or `cfg(test)`, such as a `tests.rs` module loaded with
a plain `mod tests;`, also count as non-test.

## Example

```rust
// src/main.rs, 1,800 lines long
fn main() {
    let config = load_config();
    run_commands(&config);
}

fn load_config() -> Config {
    // 600 lines of configuration loading
}

fn run_commands(config: &Config) {
    // 1,100 lines of command handling
}
```

## Use instead

```rust
// src/main.rs
mod commands;
mod config;

fn main() {
    let config = config::load();
    commands::run(&config);
}
```
