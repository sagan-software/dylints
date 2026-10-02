// run-rustfix
// rustfix-only-machine-applicable
#![feature(error_generic_member_access)]
#![allow(dead_code)]

use std::backtrace::Backtrace;

use thiserror::{Error, Error as ThisError};

/// A type alias that thiserror does not recognize as a backtrace.
type Trace = Backtrace;

#[derive(thiserror::Error, Debug)]
#[error("failed")]
pub struct QualifiedRedundantBacktrace {
    #[backtrace]
    backtrace: std::backtrace::Backtrace,
}

#[derive(Error, Debug)]
pub enum ImportedRedundantBacktrace {
    #[error("failed")]
    Failed(String, #[backtrace] Backtrace),
}

mod glob {
    use std::backtrace::Backtrace;
    use thiserror::*;

    #[derive(Error, Debug)]
    #[error("failed")]
    pub struct GlobRedundantBacktrace {
        #[backtrace]
        trace: Backtrace,
    }
}

#[derive(ThisError, Debug)]
#[error("automatic")]
pub struct AutomaticBacktrace {
    backtrace: Backtrace,
}

#[derive(ThisError, Debug)]
#[error("alias")]
pub struct AliasedBacktrace {
    #[backtrace]
    trace: Trace,
}

#[derive(ThisError, Debug)]
#[error("second")]
pub struct SecondBacktrace {
    first: Backtrace,
    #[backtrace]
    second: Backtrace,
}

#[derive(ThisError, Debug)]
pub enum ForwardingBacktrace {
    #[error("io")]
    Io {
        #[backtrace]
        source: std::io::Error,
    },
}

fn main() {}
