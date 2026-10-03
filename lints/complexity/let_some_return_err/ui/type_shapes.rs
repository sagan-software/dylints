#![allow(dead_code)]

enum Error {
    Missing,
}

fn make_error() -> Error {
    Error::Missing
}

const ERROR_FACTORY: fn() -> Error = make_error;

// An opaque function-item error has a resolved function path, not a constant path.
fn function_item_error(value: Option<u64>) -> Result<u64, impl Fn() -> Error> {
    let Some(value) = value else {
        return Err(make_error);
    };
    Ok(value)
}

// A constant function pointer is callable but does not resolve to a function definition.
fn pointer_error(value: Option<u64>) -> Result<u64, Error> {
    let Some(value) = value else {
        return Err(ERROR_FACTORY());
    };
    Ok(value)
}

// A block expression remains outside the fixed-type rewrite profile.
fn block_error(value: Option<u64>) -> Result<u64, Error> {
    let Some(value) = value else {
        return Err({ Error::Missing });
    };
    Ok(value)
}

fn main() {}
