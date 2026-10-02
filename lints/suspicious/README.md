# suspicious

Rust review lints for suspicious code and error handling.

## Lints

- [`broad_string_error_variant`](broad_string_error_variant): flags broad string
  payloads on error enum variants in favor of structured failure data.
- [`collection_bool_result`](collection_bool_result): flags return types that
  pair a collection with a `bool`.
- [`logged_conversion_impl`](logged_conversion_impl): flags logging in `From`,
  `TryFrom`, and `FromStr` impl methods in favor of leaving logging to callers.
- [`string_error_result`](string_error_result): flags `Result<_, String>` error
  types in favor of structured error values.
