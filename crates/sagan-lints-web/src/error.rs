//! Diagnostic errors for catalog discovery and rendering.

use std::{io, path::PathBuf};

use thiserror::Error;

/// Errors produced while discovering or rendering the catalog.
#[derive(Debug, Error)]
pub(crate) enum SiteError {
    /// A filesystem operation failed.
    #[error("could not access {path}: {source}")]
    Io {
        /// Affected path.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// Recursive directory traversal failed.
    #[error("could not traverse {path}: {source}")]
    Walk {
        /// Directory being traversed.
        path: PathBuf,
        /// Underlying traversal error.
        source: walkdir::Error,
    },

    /// A lint manifest could not be parsed.
    #[error("could not parse {path}: {source}")]
    Manifest {
        /// Manifest path.
        path: PathBuf,
        /// TOML parse error.
        source: toml::de::Error,
    },

    /// A lint title is not a stable URL-safe identity.
    #[error("lint README {path} has invalid title {value:?}")]
    InvalidLintId {
        /// README path.
        path: PathBuf,
        /// Invalid title.
        value: String,
    },

    /// A lint title does not match its crate directory.
    #[error("lint title {title:?} does not match directory {directory:?} in {path}")]
    MismatchedLintId {
        /// README path.
        path: PathBuf,
        /// Parsed title.
        title: String,
        /// Directory name.
        directory: PathBuf,
    },

    /// A README is missing its level-one lint title.
    #[error("lint README has no level-one title: {path}")]
    MissingTitle {
        /// README path.
        path: PathBuf,
    },

    /// A README does not use the required section contract.
    #[error("lint README {path} is missing or misorders {heading:?}")]
    InvalidReadmeStructure {
        /// README path.
        path: PathBuf,
        /// Missing or misplaced heading.
        heading: &'static str,
    },

    /// A lint manifest has no supported category.
    #[error("lint manifest has no supported category: {path}")]
    UnknownCategory {
        /// Lint manifest path.
        path: PathBuf,
    },

    /// A discovered path escaped the expected repository root.
    #[error("path {path} is not below {root}: {source}")]
    PathOutsideRoot {
        /// Path being reduced.
        path: PathBuf,
        /// Expected ancestor.
        root: PathBuf,
        /// Underlying prefix error.
        source: std::path::StripPrefixError,
    },

    /// No lint crates were discovered.
    #[error("no lint UI directories found under {path}")]
    NoLints {
        /// Searched lint tree.
        path: PathBuf,
    },

    /// A lint list row lacks a name or a level.
    #[error("lint list row has no name and level: {row:?}")]
    InvalidLintListRow {
        /// Offending row without its indentation.
        row: String,
    },

    /// A lint list row names an unknown level.
    #[error("lint list has unknown level {level:?}")]
    InvalidLintLevel {
        /// Unrecognized level spelling.
        level: String,
        /// Underlying parse error.
        source: strum::ParseError,
    },

    /// A lint list names one lint twice.
    #[error("lint list registers {name} more than once")]
    DuplicateRegisteredLint {
        /// Repeated lint name.
        name: String,
    },

    /// Registered lints have no README in the lint tree.
    #[error("registered lints have no README: {names}")]
    UndocumentedLints {
        /// Comma-separated lint names.
        names: String,
    },

    /// Askama could not render the HTML template.
    #[error("could not render the site template: {source}")]
    Template {
        /// Underlying template error.
        #[from]
        source: askama::Error,
    },
}
