// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use thiserror::{Error, Error as ThisError};

#[derive(thiserror::Error, Debug)]
#[error("io")]
pub struct QualifiedRedundantNamedSource {
    #[source]
    source: std::io::Error,
}

#[derive(Error, Debug)]
pub enum ImportedRedundantNamedSource {
    #[error("io")]
    Io {
        #[source]
        source: std::io::Error,
        path: String,
    },
}

mod glob {
    use thiserror::*;

    #[derive(Error, Debug)]
    #[error("io")]
    pub struct GlobRedundantNamedSource {
        #[source]
        source: std::io::Error,
    }
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

#[derive(ThisError, Debug)]
#[error("io")]
pub struct FromNamedSource {
    #[from]
    source: std::io::Error,
}

fn main() {}
