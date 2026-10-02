# manual_debug_impl

## What it does

Checks hand-written `Debug` implementations whose `fmt` method uses
`debug_struct(...)` or `debug_tuple(...)` and ends with `.finish()`.

## Why is this bad?

A hand-written `Debug` implementation must be updated whenever a field is added,
removed, or renamed. `#[derive(Debug)]` stays in sync with the type definition
and produces the same output.

## Known problems

The lint reads source text, not types. It warns when the implementation leaves
out a field or uses a different type name in `debug_struct`, where a derive
would change the output. A derive also fails when a field type does not
implement `Debug`. The lint skips any implementation whose source contains
`if `, `match `, `redact`, `secret`, `***`, or `<redacted>` anywhere, including
in type names and comments. It misses implementations that use
`.finish_non_exhaustive()` or write the output with `write!`.

## Example

```rust
use std::fmt;

struct User {
    id: u64,
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User").field("id", &self.id).finish()
    }
}
```

## Use instead

```rust
#[derive(Debug)]
struct User {
    id: u64,
}
```
