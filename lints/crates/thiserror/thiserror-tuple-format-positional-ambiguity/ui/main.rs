use thiserror::{Error, Error as ThisError};

const fn expected() -> &'static str {
    "expected"
}

#[derive(thiserror::Error, Debug)]
#[error("qualified {0} {}", expected())]
pub struct QualifiedTuple(String);

#[derive(Error, Debug)]
#[error("named first {0} {named} {}", expected(), named = 1)]
pub struct NamedFirstTuple(String);

#[derive(ThisError, Debug)]
#[error("renamed {0:?} {}", expected())]
pub struct RenamedTuple(String);

mod glob {
    use super::expected;
    use thiserror::*;

    #[derive(Error, Debug)]
    pub enum TupleVariant {
        #[error("variant {0} {}", expected())]
        Ambiguous(String),

        #[error("named {0} {expected}", expected = expected())]
        NamedArg(String),

        #[error("escaped {{0}} {}", expected())]
        EscapedNumeric(String),

        #[error("field only {0}")]
        FieldOnly(String),

        #[error("named field {} {0}", expected())]
        NamedField { value: String },

        #[error("unit {}", expected())]
        Unit,

        #[error("empty tuple {0} {}", expected())]
        EmptyTuple(),
    }
}

#[derive(ThisError, Debug)]
#[error("transparent")]
pub struct Plain(String);

fn main() {}
