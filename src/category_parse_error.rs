//! Error returned when an internal compiler process receives an unknown lint category.

use thiserror::Error;

/// Rejects a category name that is not part of the binary's closed vocabulary.
#[derive(Debug, Error)]
#[error("unknown embedded lint category `{value}`")]
pub(super) struct CategoryParseError {
    /// Unrecognized environment value supplied to the compiler process.
    value: Box<str>,
}

impl CategoryParseError {
    /// Preserve the rejected value for a precise process-boundary diagnostic.
    pub(super) fn new(value: &str) -> Self {
        Self {
            value: value.into(),
        }
    }
}
