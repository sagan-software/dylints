# manual_arg_parsing

## What it does

Checks for calls to `std::env::args` and `std::env::args_os`, including calls
through `use` imports and renamed imports.

## Why is this bad?

Code that reads `env::args()` directly must handle help text, unknown flags,
missing values, and error messages itself, and those cases are easy to get
wrong. A parser such as `clap` derives that handling from one type definition.

## Known problems

- The lint suppresses a call only when its iterator feeds a resolved
  `std::process::Command::args` or `Iterator::count` call through at most four
  standard `skip` or `take` adapters; longer chains remain linted.
- The lint still flags stored iterators, filtered or mixed streams, other
  consumers, and parsing operations such as `next`, `nth`, and `collect`.
- Methods named `args` or `count` on other types do not qualify for suppression.
- Small throwaway binaries that do not need a parser also trigger.
- Argument reads hidden behind a wrapper function in another crate are not
  detected.
- The lint cannot resolve calls through function-pointer bindings back to
  `std::env::args` or `std::env::args_os`.

## Example

```rust
fn main() {
    let _command = std::env::args().nth(1);
}
```

## Use instead

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    command: Option<String>,
}

fn main() {
    let _command = Cli::parse().command;
}
```
