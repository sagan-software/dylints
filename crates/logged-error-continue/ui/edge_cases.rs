#![allow(dead_code)]

#[derive(Debug)]
struct Error;

// A local macro that shares a logging macro's name is not a logging call.
macro_rules! error {
    ($($tokens:tt)*) => {
        let _ = format_args!($($tokens)*);
    };
}

// A local wrapper around a real logging macro still logs.
macro_rules! audit {
    ($($tokens:tt)*) => {
        tracing::warn!($($tokens)*)
    };
}

enum Outcome {
    Ok(()),
    Err(Error),
}

fn fallible() -> Result<(), Error> {
    Err(Error)
}

fn outcome() -> Outcome {
    Outcome::Err(Error)
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

fn tracing_logs_and_continues() -> Result<(), Error> {
    if let Err(error) = fallible() {
        tracing::error!(?error, "continuing after error");
    }
    Ok(())
}

fn wrapped_macro_logs_and_continues() -> Result<(), Error> {
    if let Err(error) = fallible() {
        audit!(?error, "continuing after error");
    }
    Ok(())
}

async fn async_logs_and_continues() -> Result<(), Error> {
    if let Err(error) = fallible() {
        tracing::info!(?error, "continuing after error");
    }
    Ok(())
}

fn diverging_body_logs_and_continues() -> Result<(), Error> {
    loop {
        if let Err(error) = fallible() {
            eprintln!("retrying after error: {error:?}");
        }
    }
}

fn local_macro_named_error() -> Result<(), Error> {
    if let Err(error) = fallible() {
        error!("not a logging call: {error:?}");
    }
    Ok(())
}

fn logs_and_propagates() -> Result<(), Error> {
    if let Err(error) = fallible() {
        tracing::error!(?error, "propagating error");
        return Err(error);
    }
    Ok(())
}

fn local_err_variant() -> Result<(), Error> {
    if let Outcome::Err(error) = outcome() {
        eprintln!("local enum variant named Err: {error:?}");
    }
    Ok(())
}

fn guarded_arm() -> Result<(), Error> {
    match fallible() {
        Ok(()) => {}
        Err(error) if format!("{error:?}").is_empty() => {
            eprintln!("guarded arm: {error:?}");
        }
        Err(error) => return Err(error),
    }
    Ok(())
}

fn closure_logs() -> Result<(), Error> {
    let check = || {
        if let Err(error) = fallible() {
            eprintln!("inside a closure: {error:?}");
        }
    };
    check();
    Ok(())
}

fn other_calls_and_wildcard() -> Result<(), Error> {
    if let Err(error) = fallible() {
        (|| ())();
    }
    if let Err(error) = fallible() {
        Some(error);
    }
    match fallible() {
        Ok(()) => {}
        _ => {}
    }
    Ok(())
}

fn empty_branch() -> Result<(), Error> {
    if let Err(_error) = fallible() {}
    Ok(())
}

fn main() {}
