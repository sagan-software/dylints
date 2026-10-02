#[derive(Debug)]
struct Error;

mod kslog {
    pub fn warn(_message: &str) {}
}

fn fallible() -> Result<(), Error> {
    Err(Error)
}

fn other_error() -> Result<(), &'static str> {
    Err("different")
}

fn if_let_logs_and_continues() -> Result<(), Error> {
    if let Err(error) = fallible() {
        eprintln!("fallible failed: {error:?}");
    }

    Ok(())
}

fn match_logs_and_continues() -> Result<(), Error> {
    match fallible() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("fallible failed: {error:?}");
        }
    }

    Ok(())
}

fn namespaced_log_function_continues() -> Result<(), Error> {
    if let Err(_error) = fallible() {
        kslog::warn("fallible failed");
    }

    Ok(())
}

fn propagates_with_question() -> Result<(), Error> {
    fallible()?;
    Ok(())
}

fn returns_error_from_branch() -> Result<(), Error> {
    if let Err(error) = fallible() {
        return Err(error);
    }

    Ok(())
}

fn logs_and_recovers_different_error_type() -> Result<(), Error> {
    if let Err(error) = other_error() {
        eprintln!("non-propagatable error: {error}");
    }

    Ok(())
}

fn error_branch_does_more_than_log() -> Result<(), Error> {
    if let Err(error) = fallible() {
        eprintln!("fallible failed: {error:?}");
        let _recovered = true;
    }

    Ok(())
}

fn non_result_function() {
    if let Err(error) = fallible() {
        eprintln!("fallible failed: {error:?}");
    }
}

fn main() {}
