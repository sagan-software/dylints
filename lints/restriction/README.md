# restriction

Rust review lints for local policy restrictions.

## Lints

- [`ambiguous_numeric_unit_field`](ambiguous_numeric_unit_field): flags integer
  size, length, and offset fields whose name and doc comment omit the unit.
- [`country_string_field`](country_string_field): flags country-code fields
  stored as `String` or `&str`.
- [`custom_well_known_type`](custom_well_known_type): flags local types named
  after well-known semantic types such as `Url` or `Duration`.
- [`date_string_field`](date_string_field): flags date fields stored as
  `String` or `&str`.
- [`datetime_integer_field`](datetime_integer_field): flags timestamp fields
  stored as primitive integers.
- [`duration_integer_field`](duration_integer_field): flags integer fields whose
  names carry a time unit instead of using `std::time::Duration`.
- [`hardcoded_path`](hardcoded_path): flags string literals that hold absolute,
  home-relative, Windows drive, or UNC paths.
- [`http_method_string`](http_method_string): flags HTTP method fields,
  parameters, and return types stored as `String` or `&str`.
- [`insufficient_public_documentation`](insufficient_public_documentation):
  flags doc comments on exported definitions below a prose-word minimum.
- [`interpolated_logging`](interpolated_logging): flags logging and output
  macros that format values into the message instead of structured fields.
- [`large_rust_crate`](large_rust_crate): flags crates whose combined source
  exceeds configurable non-test or total line limits.
- [`large_rust_file`](large_rust_file): flags files with 1,500 or more non-test
  lines or 2,000 or more total lines.
- [`manual_test_cases`](manual_test_cases): flags tests that loop over a literal
  case list instead of using `test-case`.
- [`many_assertions_in_test`](many_assertions_in_test): flags tests with four or
  more standard assertion macros.
- [`missing_doctest_examples`](missing_doctest_examples): flags public
  functions and methods whose doc comment lacks a `# Examples` Rust code block.
- [`missing_intent_comments`](missing_intent_comments): flags function bodies
  with fewer than one `//` comment per five statements.
- [`path_attribute_outside_root`](path_attribute_outside_root): flags `path`
  attributes outside `main.rs`, `lib.rs`, and `build.rs` files.
- [`path_string_field`](path_string_field): flags filesystem path fields stored
  as `String` or `&str`.
- [`public_serde_schema_derive`](public_serde_schema_derive): flags `pub` serde
  types without `JsonSchema` in crates that use schemars.
- [`repeated_cfg_gate`](repeated_cfg_gate): flags `cfg` predicates repeated more
  than three times across a crate outside test code.
- [`runtime_env_read`](runtime_env_read): flags `std::env::var` and
  `std::env::var_os` calls outside configuration, startup, build-script, and
  test code.
- [`secret_raw_type`](secret_raw_type): flags secret fields, parameters, and
  local variables stored as raw strings or byte buffers.
- [`semantic_primitive_type`](semantic_primitive_type): flags IDs, HTTP
  statuses, and reason codes stored as primitives, and string matches over a
  closed vocabulary.
- [`struct_update_default`](struct_update_default): flags struct literals that
  fill remaining fields with `..Default::default()`.
- [`unnecessary_module_directory`](unnecessary_module_directory): flags module
  directories that contain only `mod.rs`.
- [`unnecessary_public_type`](unnecessary_public_type): flags `pub` types that
  nothing in an unpublished crate uses.
- [`url_string_field`](url_string_field): flags URL fields stored as `String` or
  `&str`.
