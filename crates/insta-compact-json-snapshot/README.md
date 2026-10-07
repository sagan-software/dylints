# insta-compact-json-snapshot

## What it does

Checks for `insta::assert_compact_json_snapshot!` calls.

## Why is this bad?

Compact JSON puts a small value on one line, so a change to any field changes
the whole line in the diff. When the value grows past the size limit for one
line, the whole snapshot switches to multiple lines. Insta's documentation
notes that this macro has worse diff behavior.

## Known problems

The lint warns even when one-line JSON is the output under test.

## Example

```rust
fn snapshot_ids(ids: Vec<u32>) {
    insta::assert_compact_json_snapshot!(ids);
}
```

## Use instead

```rust
fn snapshot_ids(ids: Vec<u32>) {
    insta::assert_yaml_snapshot!(ids);
}
```
