# http_method_string

## What it does

Checks fields, function parameters, and function return types that use `String`
or `&str` when the field or parameter has the name `http_method` or
`request_method`. The lint checks a return type when the function has one of
those names. It also checks the generic names `method` and `verb` when the
crate depends on an HTTP library: `actix_web`, `axum`, `http`, `hyper`, `isahc`,
`poem`, `reqwest`, `rocket`, `surf`, `tide`, `ureq`, or `warp`.

## Why is this bad?

A string accepts any text, so a typo such as `"PSOT"` or a lowercase `"get"`
reaches the HTTP client before it fails. `http::Method` parses the verb once and
compares it without string matching.

## Known problems

The compiler resolves type aliases before the lint peels up to eight
consecutive standard `Option` layers. Longer chains, local `Option`
lookalikes, and user-defined wrappers remain opaque. After peeling, it checks only `String` and
`&str`, including type aliases and `use` renames. It does not inspect `Box<str>`
or closure parameters.

It matches only exact names, so it does not flag `http_verb` or `method_str`.
In a crate with an HTTP library, it still warns on a `method` that names
something else, such as a builder method. It skips destructured parameters,
functions with other names that return a method string, and methods of trait
impls, whose signature comes from the trait.

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
