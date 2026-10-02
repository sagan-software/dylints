// run-rustfix
// rustfix-only-machine-applicable
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
#[error("{source}")]
pub struct ImportedManualSourceDisplay {
    #[from]
    source: Inner,
}

#[derive(ThisError, Debug)]
#[error("{0}")]
pub struct RenamedManualTupleDisplay(#[from] Inner);

#[derive(ThisError, Debug)]
pub enum VariantManualSourceDisplay {
    #[error("{0}")]
    Variant(#[from] Inner),
}

#[cfg(any())]
#[derive(ThisError, Debug)]
#[error("{source}")]
pub struct DisabledManualSourceDisplay {
    #[from]
    source: Inner,
}

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
#[error("{0}")]
pub struct SourceAttrTupleDisplay(#[source] Inner);

#[derive(ThisError, Debug)]
#[error("{source}")]
pub struct SourceAndFromDisplay {
    #[from]
    #[source]
    source: Inner,
}

// Active `#[error]` outside a thiserror derive is rejected before this lint can run.
#[derive(Debug)]
pub struct DebugOnlyWrapper {
    source: Inner,
}

fn main() {}
