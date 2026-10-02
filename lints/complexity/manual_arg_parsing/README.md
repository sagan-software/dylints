# manual_arg_parsing

## What it does

Checks for calls to `std::env::args` and `std::env::args_os`, including calls
through `use` imports and renamed imports.

## Why is this bad?

Code that reads `env::args()` directly must handle help text, unknown flags,
missing values, and error messages itself, and those cases are easy to get
wrong. A parser such as `clap` derives that handling from one type definition.

## Known problems

- The lint flags every call, including code that only forwards the arguments
  to a child process or counts them.
- Small throwaway binaries that do not need a parser also trigger.
- Argument reads hidden behind a wrapper function in another crate are not
  detected.

## Example

```rust
fn main() {
    let command = std::env::args().nth(1);
    run(command);
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
    let cli = Cli::parse();
    run(cli.command);
}
```
