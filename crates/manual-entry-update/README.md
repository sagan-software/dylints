# manual-entry-update

## What it does

Checks for a two-arm `match` on a `HashMap` or `BTreeMap` `Entry` where the
`Occupied` arm updates the value through one `get_mut()` or `into_mut()` call
and the `Vacant` arm only calls `insert`. The match must produce `()`.

## Why is this bad?

The match spends several lines on an update-or-insert step. The reader must
check both arms to see that they only change or add one value.
`and_modify(...).or_insert(...)` states the same single-lookup update in one
chain.

## Known problems

- Each arm must use its entry binding once. The lint ignores an `Occupied` arm
  that also calls `get()` or `remove()`, or a `Vacant` arm that reads `key()`.
- The lint ignores an arm with a guard, a wildcard arm, or an arm that contains
  `return`, `break`, `continue`, `?`, or `.await`, because the arm body moves
  into a closure.
- The lint ignores `if let Entry::Occupied(..) = ...` without a `Vacant` branch.

## Example

```rust
use std::collections::{HashMap, hash_map::Entry};

fn count(map: &mut HashMap<String, i32>, key: String) {
    match map.entry(key) {
        Entry::Occupied(mut entry) => *entry.get_mut() += 1,
        Entry::Vacant(entry) => {
            entry.insert(1);
        }
    }
}
```

## Use instead

```rust
use std::collections::HashMap;

fn count(map: &mut HashMap<String, i32>, key: String) {
    map.entry(key).and_modify(|value| *value += 1).or_insert(1);
}
```
