// run-rustfix
// rustfix-only-machine-applicable
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

fn nested_result_with_try(raw: &str) -> Result<Result<u16, Error>, Error> {
    Ok(Ok(raw.parse::<u16>().map_err(Error::from)?))
}

fn block_closure_with_try(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(|error| Error::from(error))?;
    Ok(port)
}

macro_rules! parse_port {
    ($raw:expr) => {
        $raw.parse::<u16>().map_err(Error::from)?
    };
}

fn macro_body_with_try(raw: &str) -> Result<u16, Error> {
    Ok(parse_port!(raw))
}

fn adds_context(raw: &str) -> Result<u16, String> {
    raw.parse::<u16>()
        .map_err(|error| format!("invalid `{raw}`: {error}"))
}

fn closure_with_statement(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(|error| {
        let _ = 1;
        Error::from(error)
    })?;
    Ok(port)
}

fn closure_converts_other(raw: &str, other: ParseIntError) -> Result<u16, Error> {
    let port = raw
        .parse::<u16>()
        .map_err(|_error| Error::from(other.clone()))?;
    Ok(port)
}

trait Convert {
    fn convert(self) -> Error;
}

impl Convert for ParseIntError {
    fn convert(self) -> Error {
        Error
    }
}

fn custom_conversion(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(Convert::convert)?;
    let other = raw.parse::<u16>().map_err(|error| error.convert())?;
    Ok(port + other)
}

macro_rules! parse {
    ($raw:expr) => {
        $raw.parse::<u16>()
    };
}

fn macro_receiver(raw: &str) -> Result<u16, Error> {
    let port = parse!(raw).map_err(Error::from)?;
    Ok(port)
}

fn pattern_closure(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(|_| Error)?;
    Ok(port)
}

#[rustfmt::skip]
fn braced_closure(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(|error| { Error::from(error) })?;
    Ok(port)
}

fn make_error(_error: ParseIntError) -> Error {
    Error
}

fn free_function_mapper(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(make_error)?;
    Ok(port)
}

fn mapper_factory() -> fn(ParseIntError) -> Error {
    Error::from
}

fn computed_mapper(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(mapper_factory())?;
    Ok(port)
}

fn bound_result(raw: &str) -> Result<u16, Error> {
    let result = raw.parse::<u16>().map_err(Error::from);
    result
}

fn chained_after(raw: &str) -> Result<u16, Error> {
    raw.parse::<u16>().map_err(Error::from).map(|port| port + 1)
}

fn main() {}
