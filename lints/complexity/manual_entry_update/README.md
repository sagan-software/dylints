# manual_entry_update

## What it does

Checks for a two-arm `match` on a `HashMap` or `BTreeMap` `Entry` where the
`Occupied` arm updates the value through `get_mut()` or `into_mut()` and the
`Vacant` arm calls `insert`.

## Why is this bad?

The match spends several lines on an update-or-insert step. The reader must
check both arms to see that they only change or add one value.
`and_modify(...).or_insert(...)` states the same single-lookup update in one
chain.

## Known problems

- The lint checks the match by source text. It requires exactly one `;`, one
  `.insert(` call, and one `get_mut()` or `into_mut()` call in the whole match.
  A match with extra statements in either arm is ignored, even when it could
  still use the entry API.
- `if let Entry::Occupied(..) = ...` without a `Vacant` branch is ignored.

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
