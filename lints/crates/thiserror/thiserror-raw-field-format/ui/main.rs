// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

use thiserror::Error as ThisError;

#[derive(thiserror::Error, Debug)]
#[error("keyword {r#type}")]
pub struct KeywordRawFieldFormat {
    r#type: String,
}

#[derive(ThisError, Debug)]
#[error("renamed {r#kind}")]
pub struct RenamedRawFieldFormat {
    r#kind: String,
}

mod glob {
    use thiserror::*;

    #[derive(Error, Debug)]
    pub enum GlobRawFieldFormat {
        #[error("variant {r#value:?} and {r#value}")]
        Variant { value: String },
    }
}

#[derive(ThisError, Debug)]
#[error("escaped \"{r#kind}\"")]
pub struct EscapedRawFieldFormat {
    kind: String,
}

#[derive(ThisError, Debug)]
#[error("unraw {kind} {0:?}", 0)]
pub struct UnrawFieldFormat {
    r#kind: String,
}

fn main() {}
