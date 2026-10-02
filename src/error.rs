//! Shared runner failures used by the outer process and compiler runtime.

use std::error::Error as StdError;

use thiserror::Error;

/// Failures before or around child-process lint diagnostics.
#[derive(Debug, Error)]
pub(super) enum RunnerError {
    /// The target path is not a repository directory.
    #[error("not a directory: {path}")]
    NotDirectory {
        /// Rejected target path.
        path: std::path::PathBuf,
    },
    /// A packaged file or directory is absent.
    #[error("packaged asset does not exist: {path}")]
    MissingAsset {
        /// Missing packaged path.
        path: std::path::PathBuf,
    },
    /// The Cargo wrapper setting has no executable token.
    #[error("--cargo-cmd must contain an executable")]
    MissingCargoCommand,
    /// The target-cache limit is not an unsigned byte count.
    #[error("SAGAN_LINTS_TARGET_CACHE_MAX_BYTES must be an unsigned byte count, got `{0}`")]
    InvalidTargetCacheLimit(String),
    /// Every configured cache candidate was unavailable or unsafe.
    #[error(
        "no writable Sagan-lints cache directory; set SAGAN_LINTS_CACHE_DIR to a writable absolute path outside the repository (tried: {candidates:?})"
    )]
    NoWritableCache {
        /// Candidate directories inspected by the runner.
        candidates: Vec<std::path::PathBuf>,
    },
    /// Failure retaining its operation context and source chain.
    #[error("{context}: {source}")]
    External {
        /// Operation that failed.
        context: String,
        /// Original typed failure.
        source: Box<dyn StdError + Send + Sync>,
    },
}

impl RunnerError {
    /// Attach operation context without erasing the original error source.
    pub(super) fn external(
        context: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::External {
            context: context.into(),
            source: Box::new(source),
        }
    }
}

impl From<crate::diagnostics::DiagnosticError> for RunnerError {
    /// Retain diagnostic-processing failures as typed sources.
    fn from(source: crate::diagnostics::DiagnosticError) -> Self {
        Self::external("diagnostic processing failed", source)
    }
}
