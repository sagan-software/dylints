// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use thiserror::Error;

/// A value that is not a field.
const LIMIT: u8 = 8;

#[derive(Debug, Error)]
enum Error {
    #[error("failed: {}", source)]
    Bad { source: std::io::Error },
    #[error("failed: {source}")]
    Good { source: std::io::Error },
    #[error("{:?} failed", code)]
    Spec { code: u8 },
    #[error("{} at {}", path, line)]
    Two { path: String, line: u32 },
    #[error("{}, {extra}", path, extra = 1)]
    NamedAfter { path: String },
    #[error("{}", count + 1)]
    Computed { count: u8 },
    #[error("{} over {}", count, LIMIT)]
    Constant { count: u8 },
    #[error("{}", LIMIT)]
    OnlyConstant { count: u8 },
    #[error(transparent)]
    Transparent(std::io::Error),
}

mod glob {
    use thiserror::*;

    #[derive(Debug, Error)]
    #[error("glob: {}", reason)]
    pub struct GlobError {
        reason: String,
    }
}

#[derive(Debug, Error)]
#[error("tuple: {}", .0)]
pub struct TupleError(String);

#[derive(Debug, Error)]
#[error(transparent)]
struct TransparentStruct {
    source: std::io::Error,
}

fn main() {}
