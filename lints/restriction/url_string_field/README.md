# url_string_field

## What it does

Checks for named fields whose type is `String` or `&str` and whose name contains
a URL word. The words are `url`, `uri`, `endpoint`, `endpoints`, `webhook`, and
`link`, matched between underscores and with case.

## Why is this bad?

A string field accepts any text, so a malformed URL moves through the program
until a request fails far from where the value entered. A URL type parses once,
at the boundary, and gives typed access to the scheme, host, and path.

## Known problems

The lint checks only the exact types `String` and `&str`, after resolving type
aliases and `use` renames. It does not flag `Option<String>`, `Vec<String>`,
`Box<str>`, or `Cow<'_, str>`.

It skips a field whose last word is `description`, `format`, `label`, `name`,
`pattern`, `prefix`, `suffix`, `template`, `text`, or `title`, such as
`url_label` or `link_text`. It still warns on other text that is not a URL,
such as `endpoint_kind`. It does not flag other URL names, such as `href` or
`base`.

## Example

```rust
struct ServiceConfig<'a> {
    callback_url: String,
    endpoint: &'a str,
}
```

## Use instead

```rust
use url::Url;

struct ServiceConfig {
    callback_url: Url,
    endpoint: Url,
}
```
