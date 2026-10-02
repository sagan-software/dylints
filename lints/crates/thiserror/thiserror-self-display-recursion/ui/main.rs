#![allow(clippy::recursive_format_impl, unconditional_recursion, unused_imports)]

use std::fmt;

use thiserror::{Error, Error as ThisError};

#[derive(thiserror::Error, Debug)]
#[allow(unconditional_recursion)]
#[error("invalid error: {self}")]
#[cfg(not(clippy))]
pub struct QualifiedRecursiveDisplay;

#[derive(Error, Debug)]
#[allow(unconditional_recursion)]
#[error("imported error: {self}")]
#[cfg(not(clippy))]
pub struct ImportedRecursiveDisplay;

#[derive(ThisError, Debug)]
#[allow(unconditional_recursion)]
#[error("renamed error: {self:}")]
#[cfg(not(clippy))]
pub struct RenamedRecursiveDisplay;

#[cfg(any())]
#[derive(ThisError, Debug)]
#[error("disabled error: {self}")]
pub struct DisabledRecursiveDisplay;

#[derive(ThisError, Debug)]
#[error("debug is explicit: {self:?}")]
pub struct DebugDisplay;

#[derive(ThisError, Debug)]
#[error("static message")]
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
