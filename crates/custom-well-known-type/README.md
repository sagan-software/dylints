# custom-well-known-type

## What it does

Checks for local structs, enums, and type aliases named after a well-known
semantic type. The names are `StatusCode`, `HttpStatusCode`, `Method`,
`HttpMethod`, `Url`, `URL`, `Uri`, `URI`, `Uuid`, `UUID`, `Email`,
`EmailAddress`, `Path`, `PathBuf`, `Duration`, `Instant`, `Timestamp`, and
`DateTime`.

## Why is this bad?

A local `Url` or `Duration` looks like the established type but parses,
validates, and converts differently. Readers assume the familiar behavior, and
code that crosses crate boundaries needs adapters between two types for one
concept.

## Known problems

The lint matches exact names, so `PaymentMethod` or `RouteMethod` do not warn.
It warns on every declaration with a listed name, including a compatibility
type that mirrors an external API on purpose.

A type alias does not warn when it resolves to the established type, such as
`type Duration = std::time::Duration` or `type Uri = http::Uri`. An alias named
`Email`, `EmailAddress`, or `Timestamp` always warns, because the lint knows no
established type for those names.

## Example

```rust
struct StatusCode(u16);

enum HttpMethod {
    Get,
    Post,
}

type Url = String;
```

## Use instead

```rust
use http::{Method, StatusCode};
use url::Url;
```
