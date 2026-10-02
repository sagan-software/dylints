// compile-flags: --edition 2024
// normalize-stderr-test: "(\n)\n\z" -> "$1"

#![allow(dead_code)]
#![doc = "Crate docs written as an attribute."]
#![doc(html_root_url = "https://example.com")]
#![doc = "Crate docs with\nmultiple lines."]
//! Existing crate docs already use the preferred inner comment form.
/*! Block crate docs keep their original shape. */

macro_rules! generated_doc_attr {
    () => {
        #[doc = "Generated docs keep their macro source."]
        pub fn keep_generated_doc_attr() {}
    };
}

#[doc = "Parses a port number."]
pub fn warn_outer_doc_attr(raw: &str) -> Option<u16> {
    raw.parse().ok()
}

#[doc = ""]
pub fn warn_empty_outer_doc_attr() {}

#[doc = r"Raw one-line docs are still simple."]
pub fn warn_raw_outer_doc_attr() {}

#[doc = "Module outer docs written as an attribute."]
mod warn_module_outer_doc_attr {
    //! Existing module inner docs already use comments.

    #![doc = "Module inner docs written as an attribute."]

    #[doc = "Nested item docs written as an attribute."]
    pub struct WarnNestedItemDocAttr;

    /// Existing nested outer docs already use comments.
    pub struct KeepNestedDocComment;
}

/// Existing outer doc comments are already preferred.
pub fn keep_line_comment() {}

/** Existing block doc comments keep their original shape. */
pub fn keep_block_comment() {}

#[doc(hidden)]
pub fn keep_hidden_doc_attr() {}

#[doc(alias = "lookup")]
pub fn keep_alias_doc_attr() {}

#[cfg_attr(any(), doc = "Conditional docs are not a direct doc attribute.")]
pub fn keep_cfg_attr_doc() {}

#[cfg_attr(all(), doc = "Enabled cfg_attr docs are not a direct doc attribute.")]
pub fn keep_enabled_cfg_attr_doc() {}

#[doc = concat!("Composed ", "docs stay in attribute form.")]
pub fn keep_concat_doc_attr() {}

#[doc = stringify!(Stringified docs stay in attribute form.)]
pub fn keep_stringify_doc_attr() {}

#[allow(deprecated)]
pub fn keep_non_doc_attr() {}

generated_doc_attr!();

#[doc = " Leading space needs human review."]
pub fn keep_leading_space_doc_attr() {}

#[doc = "Trailing space needs human review. "]
pub fn keep_trailing_space_doc_attr() {}

#[doc = "Outer docs with\nmultiple lines."]
pub fn keep_multiline_doc_attr() {}

fn main() {}
