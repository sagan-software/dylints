// run-rustfix
// rustfix-only-machine-applicable
use std::num::ParseIntError;
use std::str::Utf8Error;

#[derive(Debug)]
struct Error;

impl From<ParseIntError> for Error {
    fn from(_error: ParseIntError) -> Self {
        Self
    }
}

impl From<Utf8Error> for Error {
    fn from(_error: Utf8Error) -> Self {
        Self
    }
}

#[derive(Debug)]
struct IntoOnlyError;

impl Into<Error> for IntoOnlyError {
    fn into(self) -> Error {
        Error
    }
}

fn explicit_from(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(Error::from)?;
    Ok(port)
}

fn trait_from(raw: &str) -> Result<u16, Error> {
    raw.parse::<u16>().map_err(From::from)
}

fn into_conversion(raw: &str) -> Result<u16, Error> {
    raw.parse::<u16>().map_err(Into::into)
}

fn closure_from(raw: &str) -> Result<u16, Error> {
    let port = raw.parse::<u16>().map_err(|error| Error::from(error))?;
    Ok(port)
}

fn closure_into(raw: &str) -> Result<u16, Error> {
    raw.parse::<u16>().map_err(|error| error.into())
}

fn wrapped_try(raw: &str) -> Result<u16, Error> {
    Ok(raw.parse::<u16>().map_err(Error::from)?)
}

fn tail_position(raw: &str) -> Result<u16, Error> {
    raw.parse::<u16>().map_err(Error::from)
}

fn preserves_context(raw: &str) -> Result<u16, String> {
    raw.parse::<u16>()
        .map_err(|error| format!("invalid port `{raw}`: {error}"))
}

fn changes_to_string(raw: &str) -> Result<u16, String> {
    raw.parse::<u16>().map_err(|error| error.to_string())
}

fn different_function_error(raw: &str) -> Result<u16, String> {
    Ok(raw
        .parse::<u16>()
        .map_err(Error::from)
        .map_err(|_| "failed".to_owned())?)
}

fn nested_closure(raw: &str) -> Result<u16, Error> {
    let parse = || raw.parse::<u16>().map_err(Error::from);
    parse()
}

fn into_without_from(result: Result<u16, IntoOnlyError>) -> Result<u16, Error> {
    result.map_err(Into::into)
}

fn from_utf8(bytes: &[u8]) -> Result<&str, Error> {
    let text = std::str::from_utf8(bytes).map_err(|error| Error::from(error))?;
    Ok(text)
}

fn main() {}
