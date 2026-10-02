# contains_key_before_any

## What it does

Checks for `map.contains_key("literal") || [..].iter().any(|key| map.contains_key(*key))`,
where the array holds only string literals and both calls use the same local
receiver and the same resolved `contains_key` method. The lint ignores a
differently named method such as `has_key`.

## Why is this bad?

The code checks the first key in a separate call that repeats the receiver and
the method. A reader must compare both halves to see that they ask the same
question. Adding the first key to the array keeps all keys in one list.

## Known problems

- The separate call must be the direct left operand of `||`. In
  `a || b || [..].iter().any(..)`, the left operand is `a || b`, so the lint
  ignores the expression.
- The receiver must be a local binding. The lint ignores field receivers such
  as `self.map.contains_key(..)`.
- The lint ignores computed keys, closures with extra conditions, and sources
  other than an inline array with `.iter().any(..)`.
- The machine-applicable fix moves the separate key to the front of the array,
  so the keys keep their search order. A disjunction produced by a macro gets
  help without a fix.

## Example

```rust
use std::collections::HashMap;

fn has_identity(document: &HashMap<String, String>) -> bool {
    document.contains_key("address")
        || ["full_name", "date_of_birth"]
            .iter()
            .any(|key| document.contains_key(*key))
}
```

## Use instead

```rust
use std::collections::HashMap;

fn has_identity(document: &HashMap<String, String>) -> bool {
    ["address", "full_name", "date_of_birth"]
        .iter()
        .any(|key| document.contains_key(*key))
}
```
