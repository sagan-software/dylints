#![expect(
    unused_crate_dependencies,
    reason = "this process-boundary target uses only the dependencies required by its own command"
)]

//! Run the development command's help from the workspace root.

use std::process::{Command, ExitCode};

/// Print the development commands and preserve command failure.
fn main() -> ExitCode {
    match Command::new("cargo").args(["xtask", "--help"]).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
