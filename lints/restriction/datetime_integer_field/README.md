# datetime_integer_field

## What it does

Checks for named fields whose type is a primitive integer and whose name contains
a timestamp word. The words are `timestamp`, `timestamps`, `epoch`, `ts`,
`date`, and `datetime`, matched between underscores and with case. The word
`unix` counts only after `at`, as in `created_at_unix`, or before `time`,
`secs`, `seconds`, `ms`, `millis`, or `nanos`, so `unix_mode` does not warn.

## Why is this bad?

An integer timestamp does not record its epoch, unit, or time zone. Code that
reads seconds where another part wrote milliseconds still compiles. A datetime
type fixes the unit and offers correct comparison and formatting.

## Known problems

The compiler resolves type aliases before the lint examines the type. The lint
peels up to eight consecutive standard `Option` layers at each point in its
traversal. Longer chains, local `Option` lookalikes, and user-defined wrappers
remain opaque. After peeling, it checks only primitive
integer types, including type aliases and `use` renames; integer newtypes
remain opaque.

It can warn where a wire format or database column requires an integer epoch.
It does not flag other timestamp names, such as `created_at` or `modified`.

## Example

```rust
struct Session {
    created_at_ts: i64,
    expires_epoch: u64,
}
```

## Use instead

```rust
use chrono::{DateTime, Utc};

struct Session {
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}
```
