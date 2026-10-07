# semantic-primitive-type

## What it does

Checks for fields, function parameters, and function return types that store a
domain value as a primitive. It flags three cases by name:

- Names ending in `_id` with an integer type.
- Names equal to `http_status` or ending in `_http_status` with an integer type.
- Names `reason`, `reasons`, `reason_code`, `reason_codes`, or ending in
  `_reason_code` or `_reason_codes`, with a `String` or `str` type.

It looks through references, slices, arrays, and `Vec`. The compiler resolves
type aliases before the lint examines the type. The lint peels up to eight
consecutive standard `Option` layers at each point in its traversal. Longer
chains, local `Option` lookalikes, and user-defined wrappers remain opaque. The
lint checks a return type against the function name. It also
flags a `match` on a string that has three or more string-literal arms and a
catch-all arm.

## Why is this bad?

Two `u64` IDs from different tables have the same type, so passing a tenant ID
where an account ID belongs still compiles. A string reason code accepts any
text, and a string `match` with a catch-all arm silently accepts typos. A
newtype or enum makes the compiler reject these mixups.

## Known problems

It warns on a free-text field named `reason` that holds a human-readable message.
It skips the string `match` inside a `FromStr::from_str` or `TryFrom::try_from`
impl, where parsing strings into an enum is the intended fix. It also skips the
methods of trait impls because the trait supplies their signature.

The `match` check counts arms whose pattern is only string literals, including
`"a" | "b"`, and needs an unguarded `_` or binding arm such as `other =>`. It
does not flag a bare `id`, a `status` field, closure parameters, or destructured
parameters.

## Example

```rust
struct Request {
    tenant_id: u64,
    reason_code: String,
}

fn state_slot(state: &str) -> usize {
    match state {
        "queued" => 0,
        "running" => 1,
        "complete" => 2,
        _ => 3,
    }
}
```

## Use instead

```rust
struct TenantId(u64);

enum ReasonCode {
    Queued,
    Expired,
}

struct Request {
    tenant_id: TenantId,
    reason_code: ReasonCode,
}

enum State {
    Queued,
    Running,
    Complete,
}

fn state_slot(state: State) -> usize {
    match state {
        State::Queued => 0,
        State::Running => 1,
        State::Complete => 2,
    }
}
```
