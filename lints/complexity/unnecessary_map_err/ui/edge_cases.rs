#![allow(dead_code)]

use std::num::ParseIntError;

#[derive(Debug)]
struct Error;

impl From<ParseIntError> for Error {
    fn from(_error: ParseIntError) -> Self {
        Self
    }
}

fn block_closure(raw: &str) -> Result<u16, Error> {
    raw.parse::<u16>().map_err(|error| Error::from(error))
}

fn nested_result(raw: &str) -> Result<Result<u16, Error>, Error> {
    Ok(raw.parse::<u16>().map_err(Error::from))
}

fn adds_context(raw: &str) -> Result<u16, String> {
    raw.parse::<u16>()
        .map_err(|error| format!("invalid `{raw}`: {error}"))
}

fn main() {}
