//! Shared runner failures used by the outer process and compiler runtime.

use std::{error::Error as StdError, fmt};

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
        "no writable Sagan-lints cache directory; set SAGAN_LINTS_CACHE_DIR to a writable path (tried: {candidates:?})"
    )]
    NoWritableCache {
        /// Candidate directories inspected by the runner.
        candidates: Vec<std::path::PathBuf>,
    },
    /// A child process did not expose an expected output pipe.
    #[error("{stream} pipe unavailable for phase `{phase}`")]
    PipeUnavailable {
        /// Phase whose pipe was absent.
        phase: String,
        /// Missing standard stream.
        stream: &'static str,
    },
    /// A child stream reader panicked before returning its output.
    #[error("{stream} reader terminated unexpectedly for phase `{phase}`: {source}")]
    ReaderPanicked {
        /// Phase whose reader panicked.
        phase: String,
        /// Affected standard stream.
        stream: &'static str,
        /// Readable representation of the panic payload.
        source: PanicMessage,
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

/// Displayable child-reader panic payload retained without a broad error string variant.
#[derive(Debug)]
pub(super) struct PanicMessage(pub(super) Box<str>);

impl fmt::Display for PanicMessage {
    /// Write the original readable panic payload.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl StdError for PanicMessage {}

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
