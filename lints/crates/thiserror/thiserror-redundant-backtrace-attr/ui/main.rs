// run-rustfix
// rustfix-only-machine-applicable
#![feature(error_generic_member_access)]
#![allow(dead_code)]

use std::backtrace::Backtrace;

use thiserror::{Error, Error as ThisError};

#[derive(thiserror::Error, Debug)]
#[error("failed")]
pub struct QualifiedRedundantBacktrace {
    #[backtrace]
    backtrace: std::backtrace::Backtrace,
}

#[derive(Error, Debug)]
#[error("failed")]
pub struct ImportedRedundantBacktrace {
    #[backtrace]
    backtrace: Backtrace,
}

#[derive(ThisError, Debug)]
#[error("failed")]
pub struct RenamedRedundantBacktrace {
    #[backtrace]
    backtrace: Backtrace,
}

#[derive(ThisError, Debug)]
#[error("failed")]
pub struct SameLineRedundantBacktrace {
    #[backtrace]
    backtrace: Backtrace,
}

#[cfg(any())]
#[derive(ThisError, Debug)]
#[error("disabled")]
pub struct DisabledRedundantBacktrace {
    #[backtrace]
    backtrace: Backtrace,
}

#[derive(ThisError, Debug)]
#[error("automatic")]
pub struct AutomaticBacktrace {
    backtrace: Backtrace,
}

#[derive(ThisError, Debug)]
pub enum ForwardingBacktrace {
    #[error("io")]
    Io {
        #[backtrace]
        source: std::io::Error,
    },
}

// A literal active `#[backtrace]` without a helper-declaring derive is rejected by rustc.
#[derive(Debug)]
pub struct DebugOnlyBacktrace {
    backtrace: Backtrace,
}

fn main() {}
