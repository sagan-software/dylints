#![allow(dead_code)]

#[derive(Debug)]
struct Error;

mod tracing {
    pub(crate) use crate::error;
}

#[macro_export]
macro_rules! error {
    ($($tokens:tt)*) => {};
}

mod local_log {
    macro_rules! error {
        ($($tokens:tt)*) => {};
    }
    pub(crate) use error;
}

fn fallible() -> Result<(), Error> {
    Err(Error)
}

fn logs_and_continues() -> Result<(), Error> {
    match fallible() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("continuing after error: {error:?}");
        }
    }
    Ok(())
}

fn logs_and_propagates() -> Result<(), Error> {
    if let Err(error) = fallible() {
        local_log::error!("propagating error: {error:?}");
        return Err(error);
    }
    Ok(())
}

fn main() {}
