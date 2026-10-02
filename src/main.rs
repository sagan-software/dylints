#![feature(rustc_private)]
#![allow(
    unknown_lints,
    reason = "private Dylint names register after Cargo compiles this runner"
)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the synchronous runner owns child processes, cache files, and polling"
)]

//! Cargo-installed runner and compiler driver for strict Rust checks.
//!
//! The executable has two process roles. The outer role parses the typed CLI,
//! resolves cache and compiler state, and runs ordered lint phases. Cargo
//! re-enters the same executable as a rustc wrapper; that role registers the
//! statically linked private lint groups and then delegates compilation to
//! rustc's driver API. Keeping both roles in one binary makes the packaged
//! skill self-contained while preserving the compiler's process contract.

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

mod category;
mod category_parse_error;
mod cli;
mod code_quality;
mod diagnostics;
mod driver;
mod error;
mod runner;
mod runtime;

use std::{env, path::PathBuf, process::ExitCode};

use clap::Parser as _;
use tempfile as _;

use self::{cli::Cli, runner::run};

/// Parse the command line and preserve the runner's numeric exit status.
fn main() -> ExitCode {
    // Cargo re-enters this executable as rustc before ordinary CLI parsing can run.
    if driver::is_enabled() {
        return driver::run();
    }

    let cli = Cli::parse();
    run_cli(&cli)
}

/// Run the outer command-line role after rustc-wrapper dispatch has been checked.
fn run_cli(cli: &Cli) -> ExitCode {
    // Read environment-backed cache policy once before entering the runner.
    let cache_roots = cache_roots();
    // Preserve the runner's stable numeric status at the process boundary.
    match run(cli, &cache_roots) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            // Emit one diagnostic without panicking when stderr is unavailable.
            use std::io::Write as _;

            let diagnostic = format!("sagan-lints: {error}\n");
            drop(std::io::stderr().lock().write_all(diagnostic.as_bytes()));
            ExitCode::FAILURE
        }
    }
}

/// Resolve caller-prioritized cache roots without creating any directories.
#[expect(
    runtime_env_read,
    reason = "the CLI bootstrap resolves environment-backed cache configuration"
)]
fn cache_roots() -> Vec<PathBuf> {
    [
        env::var_os("SAGAN_LINTS_CACHE_DIR").map(PathBuf::from),
        env::var_os("RUST_PERSONAL_LINTS_CACHE_DIR").map(PathBuf::from),
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .map(|path| path.join("sagan-lints")),
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".cache/sagan-lints")),
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .map(|path| path.join("rust-personal-lints")),
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".cache/rust-personal-lints")),
    ]
    .into_iter()
    .flatten()
    .collect()
}
