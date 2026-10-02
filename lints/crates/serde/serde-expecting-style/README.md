# serde-expecting-style

## What it does

Checks for a `#[serde(expecting = "...")]` message on a struct or enum that
starts with an uppercase ASCII letter or ends with a period.

## Why is this bad?

Serde inserts the message into its own error text, which reads like "invalid
type: integer `1`, expected a user id". Serde documents that the message should
complete the sentence "This Visitor expects to receive ...", so it should not
be capitalized and should not end with a period.

## Known problems

The lint flags a message that starts with an acronym, such as `"UUID string"`,
and suggests `"uUID string"`. It only checks plain string literals and misses
raw strings.

The lint reads source files as text, so it also checks items disabled by
`cfg`.

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
