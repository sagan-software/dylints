# style

Rust review lints for idiomatic style.

## Lints

- [`ad_hoc_as_ref`](ad_hoc_as_ref): flags `as_*` and `get_*` accessors that
  could be `AsRef`.
- [`ad_hoc_borrow`](ad_hoc_borrow): flags `borrow_*` accessors that could use `Borrow`.
- [`ad_hoc_default`](ad_hoc_default): flags zero-argument constructors that
  could be `Default`.
- [`ad_hoc_display`](ad_hoc_display): flags string-formatting methods that
  could be `Display`.
- [`ad_hoc_from`](ad_hoc_from): flags infallible conversion functions that
  could be `From`.
- [`ad_hoc_from_str`](ad_hoc_from_str): flags string parser functions that
  could be `FromStr`.
- [`ad_hoc_into_iterator`](ad_hoc_into_iterator): flags iteration methods that
  could be `IntoIterator`.
- [`ad_hoc_iterator`](ad_hoc_iterator): flags `next`-style methods that could
  be `Iterator`.
- [`ad_hoc_try_from`](ad_hoc_try_from): flags fallible conversion functions
  that could be `TryFrom`.
- [`bool_name_prefix`](bool_name_prefix): flags `bool` fields, variables,
  parameters, functions, methods, constants, and statics whose names do not
  read as predicates, such as `is_ready`.
- [`doc_attr_comment`](doc_attr_comment): flags `#[doc = "..."]` attributes
  that could be `///` or `//!` comments.
- [`internal_import_self`](internal_import_self): flags imports from a child
  module that lack a `self::` prefix.
- [`manual_debug_impl`](manual_debug_impl): flags hand-written `Debug` impls
  that could be `#[derive(Debug)]`.
- [`manual_default_impl`](manual_default_impl): flags hand-written `Default`
  impls that could be `#[derive(Default)]`.
- [`manual_error_impl`](manual_error_impl): flags hand-written `Display` and
  empty `Error` impl pairs that could be `thiserror::Error`.
- [`module_type_first`](module_type_first): flags a module's namesake type when
  it is not the first item after imports.
- [`rumdl_doc_comments`](rumdl_doc_comments): runs rumdl's Markdown rules on
  Rust doc comments.
- [`top_level_item_line_break`](top_level_item_line_break): flags module items
  that start on the same line where the previous item ends.
