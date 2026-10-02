# serde-expecting-style

## What it does

Checks for a `#[serde(expecting = "...")]` message on a struct or enum that
starts with a capitalized word or ends with a period.

## Why is this bad?

Serde inserts the message into its own error text, which reads like "invalid
type: integer `1`, expected a user id". Serde documents that the message should
complete the sentence "This Visitor expects to receive ...", so it should not
be capitalized and should not end with a period.

## Known problems

The lint keeps the case of a first word that looks like an acronym, such as
`"UUID string"` or `"I/O path"`, but it still lowercases a single capital
letter followed by a space. The fix edits the literal as written, so a literal
whose first letter or final period is written as an escape gets help without a
fix.

## Example

```rust
#[derive(serde::Deserialize)]
#[serde(expecting = "A user id.")]
struct UserId(String);
```

## Use instead

```rust
#[derive(serde::Deserialize)]
#[serde(expecting = "a user id")]
struct UserId(String);
```
