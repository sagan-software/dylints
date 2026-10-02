#![allow(unconditional_recursion, unused_imports)]

use std::fmt;

use thiserror::{Error, Error as ThisError};

#[derive(thiserror::Error, Debug)]
#[error("invalid error: {self}")]
pub struct QualifiedRecursiveDisplay;

#[derive(Error, Debug)]
#[error("padded error: {self:>10}")]
pub struct PaddedRecursiveDisplay;

#[derive(Error, Debug)]
#[error("positional error: {}", self)]
pub struct PositionalRecursiveDisplay;

#[derive(Error, Debug)]
pub enum VariantDisplay {
    #[error("tuple error: {0}")]
    Tuple(u8),
}

mod glob {
    use thiserror::*;

    #[derive(Error, Debug)]
    #[error("glob error: {self:}")]
    pub struct GlobRecursiveDisplay;
}

#[derive(ThisError, Debug)]
#[error("debug is explicit: {self:?}")]
pub struct DebugDisplay;

#[derive(ThisError, Debug)]
#[error("escaped {{self}} text")]
pub struct EscapedDisplay;

#[derive(ThisError, Debug)]
#[error("static message {}", 1)]
pub struct StaticDisplay;

// The recursive display shape can exist outside thiserror without using its helper attrs.
#[derive(Debug)]
pub struct DebugOnlySelfDisplay;

impl fmt::Display for DebugOnlySelfDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

fn main() {}
