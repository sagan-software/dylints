//! This package's manifest inherits `thiserror` from the workspace with
//! `default-features = false`, so the lint treats `std` as disabled. Another
//! dependency enables `std` through Cargo feature unification, which lets this
//! fixture compile.

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
#[error("missing {path}")]
pub struct MissingPath {
    path: PathBuf,
}

mod glob {
    use std::path::Path;
    use thiserror::*;

    #[derive(Error, Debug)]
    pub enum Borrowed<'a> {
        #[error("missing {path}")]
        Missing { path: &'a Path },
    }
}

#[derive(Error, Debug)]
#[error("missing {}", path.display())]
pub struct DisplayPath {
    path: PathBuf,
}

#[derive(Error, Debug)]
#[error("missing {path:?}")]
pub struct DebugPath<'a> {
    path: &'a Path,
}

#[derive(Error, Debug)]
#[error("missing {name}")]
pub struct NamedFile {
    name: String,
}

#[derive(Error, Debug)]
#[error("duplicate")]
struct DuplicateName {
    marker: (),
}

#[allow(dead_code, non_upper_case_globals)]
const DuplicateName: u8 = 0;

fn main() {}
