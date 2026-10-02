#![allow(dead_code)]

use thiserror::{Error, Error as ThisError};

#[derive(thiserror::Error, Debug)]
#[error("io")]
pub struct QualifiedRedundantNamedSource {
    #[source]
    source: std::io::Error,
}

#[derive(Error, Debug)]
#[error("io")]
pub struct ImportedRedundantNamedSource {
    #[source]
    source: std::io::Error,
}

#[derive(ThisError, Debug)]
#[error("io")]
pub struct RenamedRedundantNamedSource {
    #[source]
    source: std::io::Error,
}

#[cfg(any())]
#[derive(ThisError, Debug)]
#[error("io")]
pub struct DisabledRedundantNamedSource {
    #[source]
    source: std::io::Error,
}

#[derive(ThisError, Debug)]
#[error("io")]
pub struct ImplicitNamedSource {
    source: std::io::Error,
}

#[derive(ThisError, Debug)]
#[error("route {source}")]
pub struct RawSourceData {
    #[source]
    r#source: std::io::Error,
}

// Active `#[source]` outside a thiserror derive is rejected before this lint can run.
#[derive(Debug)]
pub struct DebugOnlyNamedSource {
    source: std::io::Error,
}

fn main() {}
