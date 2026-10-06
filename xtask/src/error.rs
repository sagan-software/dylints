//! Development-command failures with their original diagnostic sources.

use std::{io, process::ExitStatus};

/// A rejected development operation.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// A filesystem or process operation failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A JSON report could not be decoded or encoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Workspace metadata was not valid TOML.
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    /// A workspace version failed semver parsing.
    #[error("workspace version is not valid semver: {0}")]
    Semver(#[from] semver::Error),
    /// A directory could not be traversed.
    #[error(transparent)]
    Walk(#[from] walkdir::Error),
    /// A child process rejected the operation.
    #[error("{program} failed with {status}")]
    Process {
        /// Program that failed.
        program: String,
        /// Child exit status.
        status: ExitStatus,
    },
    /// An argument or report violated its contract.
    #[error("{0}")]
    Invalid(String),
}
