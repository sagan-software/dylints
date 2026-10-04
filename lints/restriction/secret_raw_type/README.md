# secret_raw_type

## What it does

Checks for fields, function parameters, and `let` bindings whose type is a raw
string or byte buffer and whose name marks a secret. A name marks a secret when
it contains the word `token`, `password`, or `secret`, or the words `api_key`,
or equals `apikey`, compared without case. The raw types are `String`, `&str`,
`Vec<u8>`, `Box<[u8]>`, `&[u8]`, `[u8]`, and `[u8; N]`.

## Why is this bad?

A raw string or byte buffer prints in full through `Debug`, so a secret reaches
logs through a derived `Debug` or a `{:?}` format. It also stays in memory after
drop. A secret type such as `secrecy::SecretString` redacts `Debug` output and
zeroes the memory on drop.

## Known problems

It warns on values that only mention a secret word, such as a hashed
`password_hash`, or a parser's `next_token: String`. It does not flag other
secret names, such as `private_key` or `credentials`.

The compiler resolves type aliases before the lint peels up to eight
consecutive standard `Option` layers. Longer chains, local `Option`
lookalikes, and user-defined wrappers remain opaque. It does not inspect `Box<str>`, closure
parameters, or destructured bindings. The lint skips trait impl method
parameters because the trait supplies their types; it checks the trait
declaration instead.

## Example

```rust
struct Credentials<'a> {
    api_key: String,
    session_token: &'a str,
    password: Vec<u8>,
}

fn authenticate(access_token: String) {
    let refresh_token = "raw-token";
}
```

## Use instead

```rust
use secrecy::{SecretBox, SecretString};

struct Credentials {
    api_key: SecretString,
    session_token: SecretString,
    password: SecretBox<[u8]>,
}

fn authenticate(access_token: SecretString) {
    let refresh_token = SecretString::from("raw-token");
}
```
