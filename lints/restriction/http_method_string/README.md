# http_method_string

## What it does

Checks for fields, function parameters, and function return types whose type is
`String` or `&str` when the field or parameter is named `method`, `http_method`,
`request_method`, or `verb`. A return type is checked when the function has one
of those names.

## Why is this bad?

A string accepts any text, so a typo such as `"PSOT"` or a lowercase `"get"`
reaches the HTTP client before it fails. `http::Method` parses the verb once and
compares it without string matching.

## Known problems

The lint checks only the exact types `String` and `&str`, after resolving type
aliases and `use` renames. It does not flag `Option<String>`, `Box<str>`, or
closure parameters.

It matches only the four exact names, so it does not flag `http_verb` or
`method_str`. It skips destructured parameters and functions with other names
that return a method string.

## Example

```rust
struct Route<'a> {
    method: String,
    http_method: &'a str,
}

fn request_method(verb: &'static str) -> &'static str {
    verb
}
```

## Use instead

```rust
use http::Method;

struct Route {
    method: Method,
    http_method: Method,
}

fn request_method(verb: Method) -> Method {
    verb
}
```
