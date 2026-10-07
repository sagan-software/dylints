#![cfg_attr(test, feature(rustc_private))]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the synchronous development command owns filesystem and child-process operations"
)]

//! Development commands for building, checking and publishing Dylint artifacts.
//!
//! This private command replaces the former runner and shell scripts. Native Dylint
//! owns lint discovery and compiler invocation. The command coordinates coverage,
//! benchmarks, documentation and releases while preserving child-process failures.
//! Environment configuration is captured at startup and passed into each operation.

mod coverage;
mod error;
mod lcov;
mod paths;
mod percentage;
mod process;
mod release;
mod site;

use std::{
    env,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use clap::{Parser, Subcommand, ValueEnum};
#[cfg(test)]
use criterion as _;
#[cfg(test)]
use dylint_support as _;
use tempfile as _;

use crate::{coverage::Coverage, error::Error};

/// Workspace development command.
#[derive(Debug, Parser)]
struct Cli {
    /// Development operation to execute.
    #[command(subcommand)]
    command: Task,
}

/// Supported development operations.
#[derive(Debug, Subcommand)]
enum Task {
    /// Measure instrumented workspace coverage.
    Coverage(Coverage),
    /// Generate the catalog, book and available reports.
    Site {
        /// Destination directory for the Pages artifact.
        #[arg(long, default_value = "public")]
        out_dir: PathBuf,
    },
    /// Check ordinary library, binary and integration-test targets with Dylint.
    Lint,
    /// List registered lint names and default levels.
    List,
    /// Measure all independently loadable lint libraries.
    Bench {
        /// Use standard statistical samples instead of the quick inventory sweep.
        #[arg(long, num_args = 0, default_missing_value = "enabled")]
        full: Option<Presence>,
    },
    /// Validate the shared semver or publish after all required checks.
    Release {
        /// Commit before the push; require its workspace version to increase.
        #[arg(long)]
        base: Option<release::GitCommit>,
        /// Create immutable tags, maintenance branches and the GitHub release.
        #[arg(long, num_args = 0, default_missing_value = "enabled")]
        publish: Option<Presence>,
    },
}

/// Presence-only development switch.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Presence {
    /// The switch was provided.
    Enabled,
}

/// Parse environment values once and preserve command failure at the process boundary.
fn main() -> ExitCode {
    // Capture process configuration before dispatching typed operations.
    let cli = Cli::parse();
    let target = env::var_os("COVERAGE_TARGET_DIR");
    let rustflags = env::var_os("RUSTFLAGS");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf);
    // Missing workspace context and task failures share the CLI failure boundary.
    let result = root
        .ok_or_else(|| Error::Invalid("xtask has no workspace root".to_owned()))
        .and_then(|root| run(&cli.command, &root, target.as_deref(), rustflags));
    // Preserve the diagnostic without panicking if the error stream closes.
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            drop(writeln!(std::io::stderr().lock(), "xtask: {error}"));
            ExitCode::FAILURE
        }
    }
}

/// Dispatch typed development operations with environment reads at this CLI boundary.
fn run(
    task: &Task,
    root: &Path,
    target: Option<&std::ffi::OsStr>,
    rustflags: Option<std::ffi::OsString>,
) -> Result<(), Error> {
    match task {
        Task::Coverage(options) => options.run(root, target, rustflags),
        Task::Site { out_dir } => site::generate(root, &root.join(out_dir)),
        Task::Lint => process::run(
            Command::new("cargo")
                .current_dir(root)
                .env("DYLINT_RUSTFLAGS", "-D warnings")
                .args([
                    "dylint",
                    "--all",
                    "--workspace",
                    "--",
                    "--lib",
                    "--bins",
                    "--tests",
                ]),
        ),
        Task::List => process::run(
            Command::new("cargo")
                .current_dir(root)
                .args(["dylint", "list", "--all"]),
        ),
        Task::Bench { full } => benchmark(root, full.is_some()),
        Task::Release { publish, base } => release::run(root, base.as_ref(), publish.is_some()),
    }
}

/// Keep the fast inventory sweep separate from full statistical measurements.
fn benchmark(root: &Path, is_full: bool) -> Result<(), Error> {
    // Reuse the development profile to avoid rebuilding the entire compiler integration.
    let mut command = Command::new("cargo");
    let _configured = command.current_dir(root).args([
        "bench",
        "-p",
        "xtask",
        "--bench",
        "lint_passes",
        "--profile",
        "dev",
        "--",
    ]);
    if !is_full {
        let _configured = command.env("SAGAN_BENCH_FAST_SWEEP", "1");
    }
    process::run(&mut command)
}
