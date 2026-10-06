//! Child-process execution shared by development operations.

use std::process::{Command, Output};

use crate::error::Error;

/// Run a child with inherited output and reject unsuccessful exit statuses.
pub(crate) fn run(command: &mut Command) -> Result<(), Error> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Process {
            program: command.get_program().to_string_lossy().into_owned(),
            status,
        })
    }
}

/// Capture a successful command's output without treating failure as report
/// data.
pub(crate) fn output(command: &mut Command) -> Result<Output, Error> {
    let result = command.output()?;
    if result.status.success() {
        Ok(result)
    } else {
        Err(Error::Process {
            program: command.get_program().to_string_lossy().into_owned(),
            status: result.status,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    /// Inherited and captured commands preserve successful and unsuccessful
    /// exits.
    #[test_case::test_case("true", false, true; "inherited success")]
    #[test_case::test_case("true", true, true; "captured success")]
    #[test_case::test_case("false", false, false; "inherited failure")]
    #[test_case::test_case("false", true, false; "captured failure")]
    #[test_case::test_case("/missing-xtask-tool", false, false; "missing inherited tool")]
    #[test_case::test_case("/missing-xtask-tool", true, false; "missing captured tool")]
    fn process_success_and_failure(program: &str, is_captured: bool, is_successful: bool) {
        // Both capture modes must preserve the same process success boundary.
        let mut command = Command::new(program);
        let result = if is_captured {
            super::output(&mut command).map(|_output| ())
        } else {
            super::run(&mut command)
        };
        assert_eq!(result.is_ok(), is_successful);
    }
}
