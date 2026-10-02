// run-rustfix
// rustfix-only-machine-applicable
use thiserror::{Error, Error as ThisError};

#[derive(thiserror::Error, Debug)]
pub enum QualifiedRedundantSource {
    #[error("io")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
}

#[derive(Error, Debug)]
pub enum ImportedRedundantSource {
    #[error("io")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
}

#[derive(ThisError, Debug)]
pub enum RenamedRedundantSource {
    #[error("io")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
}

#[cfg(any())]
#[derive(ThisError, Debug)]
pub enum DisabledRedundantSource {
    #[error("io")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
}

#[derive(ThisError, Debug)]
pub enum FromOnly {
    #[error("io")]
    Io(#[from] std::io::Error),
}

#[derive(ThisError, Debug)]
pub enum SourceOnly {
    #[error("io")]
    Io(#[source] std::io::Error),
}

// Active thiserror helper attributes require a helper-declaring derive macro to compile.
#[derive(Debug)]
pub enum DebugOnlyIo {
    Io(std::io::Error),
}

fn main() {}
