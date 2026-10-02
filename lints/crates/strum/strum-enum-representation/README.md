# strum-enum-representation

## What it does

Checks fieldless enums for two problems:

- The enum has hand-written implementations of both `Display` and `FromStr`.
- The enum derives Serde and Strum traits, and the two produce or accept
  different names for a variant. Output names are compared when the enum
  derives `Serialize` and one of `strum::Display`, `AsRefStr`,
  `IntoStaticStr`, or `VariantNames`. Accepted input names are compared when
  it derives `Deserialize` and `strum::EnumString`.

The comparison applies Serde's `rename_all`, `rename`, `alias`, `skip`,
`skip_serializing`, `skip_deserializing`, and `other` attributes, and Strum's
`serialize_all`, `to_string`, `serialize`, `prefix`, `suffix`, `disabled`,
`default`, and `ascii_case_insensitive` attributes.

## Why is this bad?

Hand-written `Display` and `FromStr` implementations repeat the same mapping
twice, and the two can drift apart. Strum derives both from one declaration.

When Serde and Strum use different names, the enum serializes one spelling and
displays or parses another. The same rule name does not guarantee the same
result. With `snake_case`, Serde turns `HTTPResponse` into `h_t_t_p_response`,
while Strum produces `http_response`.

## Known problems

The lint does not read the bodies of the hand-written implementations. It also
flags an enum whose `Display` output is meant for people and differs on
purpose from the names that `FromStr` parses.

The lint reports only the first divergent variant of each enum. It skips
variants with non-ASCII names.

The lint matches derives to enums by name, so two enums with the same name in
one crate can share the same derive result.

## Example

```rust
use std::{fmt, str::FromStr};

enum Status {
    Ready,
    Busy,
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready => formatter.write_str("ready"),
            Self::Busy => formatter.write_str("busy"),
        }
    }
}

impl FromStr for Status {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ready" => Ok(Self::Ready),
            "busy" => Ok(Self::Busy),
            _ => Err(()),
        }
    }
}
```

The lint also flags Serde and Strum names that differ:

```rust
#[derive(serde::Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum ResponseKind {
    HTTPResponse,
}
```

## Use instead

```rust
#[derive(strum::Display, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
enum Status {
    Ready,
    Busy,
}
```

Spell the variant so that both case rules give the same name:

```rust
#[derive(serde::Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum ResponseKind {
    HttpResponse,
}
```
