// run-rustfix
// rustfix-only-machine-applicable
//! Documentation fixtures for `rumdl_doc_comments`.
//!
//! These crate docs model a small library overview instead of a one-line
//! placeholder. The text includes paragraphs, inline code, and intra-doc links
//! such as [`Documented`] so the lint sees common rustdoc input.
//!
//! # Crate example
//!
//! ```
//! # let title = "guide";
//! assert_eq!(title.len(), 5);
//! ```

/// Do not write * emphasis * with spaces.
pub fn bad_emphasis() {}

/// Build a short rendered document.
///
/// The public docs usually need more than one sentence. This paragraph keeps a
/// normal prose shape with `inline_code`, *emphasis*, and a link to
/// [`Documented::value`].
///
/// # Examples
///
/// ```
/// # use crate::clean_docs;
/// let output = clean_docs();
/// assert_eq!(output, "rendered");
/// ```
///
/// Rustdoc also accepts attributes on code fences. The lint should treat this
/// as a normal fenced block and leave the example text untouched.
///
/// ```rust,no_run
/// let output = crate::clean_docs();
/// println!("{output}");
/// ```
pub fn clean_docs() -> &'static str {
    "rendered"
}

/// A small documented data structure.
///
/// The struct-level comment references [`Self::value`] and contains a rustdoc
/// section heading. It should remain clean even though the surrounding fixture
/// contains intentionally bad comments.
///
/// # Field notes
///
/// Consumers read [`Documented::value`] when rendering a summary.
pub struct Documented {
    /// More * good * text on a field.
    pub value: usize,
}

fn main() {}
