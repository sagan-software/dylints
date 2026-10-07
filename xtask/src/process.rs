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
        Err(Error::CapturedProcess {
            program: command.get_program().to_string_lossy().into_owned(),
            status: result.status,
            output: captured_diagnostics(&result),
        })
    }
}

/// Retain both child streams when a captured command fails.
fn captured_diagnostics(output: &Output) -> String {
    let mut diagnostics = String::new();
    // Label streams separately to preserve where each captured message came from.
    if !output.stderr.is_empty() {
        diagnostics.push_str("stderr:\n");
        diagnostics.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    // Append stdout second so combined diagnostics have stable, readable ordering.
    if !output.stdout.is_empty() {
        if !diagnostics.is_empty() && !diagnostics.ends_with('\n') {
            diagnostics.push('\n');
        }
        diagnostics.push_str("stdout:\n");
        diagnostics.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    diagnostics
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

    /// Captured child failures retain diagnostics from both streams.
    #[test]
    fn captured_process_failure_includes_child_output() {
        // Make one deterministic failing command with distinct stdout and stderr.
        let mut command = Command::new("sh");
        let _ = command.args([
            "-c",
            "printf 'stdout detail'; printf 'stderr detail' >&2; exit 23",
        ]);
        let error = super::output(&mut command).expect_err("child should fail");
        let message = error.to_string();
        // Both streams must survive capture so callers can diagnose failures.
        assert!(message.contains("stderr detail"));
        assert!(message.contains("stdout detail"));
    }
}
