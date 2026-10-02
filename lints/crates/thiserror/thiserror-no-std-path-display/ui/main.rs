use std::path::PathBuf;

use thiserror::Error;

/// thiserror-no-std-path-display: default-features = false
#[derive(Error, Debug)]
#[error("missing {path}")]
pub struct MissingPath {
    path: PathBuf,
}

#[derive(Error, Debug)]
#[error("missing {}", path.display())]
pub struct DisplayPath {
    path: PathBuf,
}

fn main() {}
