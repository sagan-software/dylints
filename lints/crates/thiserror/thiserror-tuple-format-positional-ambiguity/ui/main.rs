use thiserror::{Error, Error as ThisError};

const fn expected() -> &'static str {
    "expected"
}

#[derive(thiserror::Error, Debug)]
#[error("qualified {0} {}", expected())]
pub struct QualifiedTuple(String);

#[derive(Error, Debug)]
#[error("imported {0} {}", expected())]
pub struct ImportedTuple(String);

#[derive(ThisError, Debug)]
#[error("renamed {0:?} {}", expected())]
pub struct RenamedTuple(String);

#[derive(ThisError, Debug)]
pub enum TupleVariant {
    #[error("variant {0} {}", expected())]
    Ambiguous(String),

    #[error("named {0} {expected}", expected = expected())]
    NamedArg(String),

    #[error("escaped {{0}} {}", expected())]
    EscapedNumeric(String),

    #[error("field only {0}")]
    FieldOnly(String),
}

#[derive(ThisError, Debug)]
#[error("named {0} {expected}", expected = expected())]
pub struct NamedTupleArg(String);

#[derive(ThisError, Debug)]
#[error("field only {0}")]
pub struct FieldOnlyTuple(String);

#[cfg(any())]
#[derive(ThisError, Debug)]
#[error("disabled {0} {}", expected())]
pub struct DisabledTuple(String);

#[cfg(any())]
#[derive(Debug)]
#[error("debug only {0} {}", expected())]
pub struct DebugOnlyTuple(String);

fn main() {}
