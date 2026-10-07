#![allow(dead_code)]

use thiserror::{Error, Error as ThisError};

#[derive(ThisError, Debug)]
#[error("inner")]
pub struct Inner;

#[derive(thiserror::Error, Debug)]
#[error("{source}")]
pub struct QualifiedManualSourceDisplay {
    #[from]
    source: Inner,
}

#[derive(Error, Debug)]
#[error("{inner}")]
pub struct ImportedManualSourceDisplay {
    #[from]
    inner: Inner,
}

#[derive(ThisError, Debug)]
#[error("{0}")]
pub struct RenamedManualTupleDisplay(#[from] Inner);

mod glob {
    use thiserror::*;

    #[derive(Error, Debug)]
    pub enum VariantManualSourceDisplay {
        #[error("{0}")]
        Variant(#[from] super::Inner),
    }
}

#[derive(ThisError, Debug)]
#[error("{0}")]
pub struct SourceAttrTupleDisplay(#[source] Inner);

#[derive(ThisError, Debug)]
#[error(transparent)]
pub struct TransparentSource {
    #[from]
    source: Inner,
}

#[derive(ThisError, Debug)]
#[error("{source}")]
pub struct MultiFieldDisplay {
    #[source]
    source: Inner,
    context: String,
}

#[derive(ThisError, Debug)]
#[error("wrapped: {source}")]
pub struct PrefixedDisplay {
    source: Inner,
}

#[derive(ThisError, Debug)]
#[error("{source:?}")]
pub struct DebugDisplay {
    source: Inner,
}

#[derive(ThisError, Debug)]
#[error("{}", .0)]
pub struct PositionalDisplay(#[from] Inner);

#[derive(ThisError, Debug)]
#[error("{value}")]
pub struct DataDisplay {
    value: String,
}

fn main() {}
