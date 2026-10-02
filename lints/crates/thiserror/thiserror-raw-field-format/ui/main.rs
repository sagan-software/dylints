// run-rustfix
// rustfix-only-machine-applicable
use thiserror::Error as ThisError;

#[cfg(any())]
#[derive(thiserror::Error, Debug)]
#[error("qualified {r#type}")]
pub struct QualifiedRawFieldFormat {
    r#type: String,
}

#[cfg(any())]
#[derive(ThisError, Debug)]
#[error("renamed {r#kind}")]
pub struct RenamedRawFieldFormat {
    r#kind: String,
}

#[cfg(any())]
#[derive(thiserror::Error, Debug)]
#[error("same-line {r#value}")]
pub struct SameLineRawFieldFormat {
    r#value: String,
}

#[derive(ThisError, Debug)]
#[error("unraw {type}")]
pub struct UnrawFieldFormat {
    r#type: String,
}

fn main() {}
