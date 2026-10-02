# insta-json-snapshot

## What it does

Checks for `insta::assert_json_snapshot!` calls.

## Why is this bad?

JSON snapshots add braces, quotes, and commas that make diffs harder to read.
Adding a field after the last one also changes the line before it, because
that line gains a comma. Insta's serializer guide recommends YAML over JSON
for most values.

## Known problems

The lint warns even when the exact JSON output is what the test checks, such
as the body of a JSON API.

## Example

```rust
fn snapshot_ids(ids: Vec<u32>) {
    insta::assert_json_snapshot!(ids);
}
```

## Use instead

```rust
fn snapshot_ids(ids: Vec<u32>) {
    insta::assert_yaml_snapshot!(ids);
}
```
