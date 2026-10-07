# tracing-redundant-field-assignment

## What it does

Checks for a field written as `name = name` in a `tracing` event or span macro,
such as `info!` or `debug_span!`. Dotted paths such as `user.id = user.id` and
values with the `%` or `?` sigil, such as `user = ?user`, also match.

## Why is this bad?

Tracing macros accept a local variable or field path as shorthand for a field
with the same name. Repeating the path adds noise and hides the fields that
rename or transform a value.

## Known problems

The lint compares source text after removing whitespace. It matches only
identifiers and dotted paths, so `name = *name` or `name = self.name` does not
trigger it.

When a reported field contains a comment, the lint gives help text but no
automatic fix.

## Example

```rust
# #[derive(Debug)] struct User { id: u64 }
fn handle(request_id: u64, user: &User) {
    tracing::info!(request_id = request_id, user.id = user.id);
    let _span = tracing::debug_span!("request", user = ?user);
}
```

## Use instead

Keep the `%` or `?` sigil before the shorthand field.

```rust
# #[derive(Debug)] struct User { id: u64 }
fn handle(request_id: u64, user: &User) {
    tracing::info!(request_id, user.id);
    let _span = tracing::debug_span!("request", ?user);
}
```
