//! Web interface generation for the custom Rust lint catalog.
//!
//! The binary discovers lint README files, validates their shared documentation
//! contract, joins them with Dylint's registered lint levels, and writes a
//! static site modeled on the Clippy lint list. Typed category, level, and
//! applicability values keep the client-side filters closed, while filesystem
//! and rendering failures retain the path that caused them.

#![expect(
    clippy::disallowed_methods,
    reason = "the synchronous site generator owns its input and output files"
)]

mod applicability;
mod catalog;
mod category;
mod error;
mod level;
mod markdown;
mod readme;
mod site;

use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use clap::Parser;

use self::{catalog::read_to_string, error::SiteError, level::RegisteredLints};

/// Command-line interface for building the Rust lint catalog.
#[derive(Debug, Parser)]
#[command(name = "sagan-lints-web")]
struct Cli {
    /// Repository root containing `lints`.
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Output of `cargo dylint list --all`, used for default lint levels.
    #[arg(long)]
    lint_list: PathBuf,

    /// Directory that receives `index.html` and its static assets.
    #[arg(long, default_value = "public")]
    out_dir: PathBuf,
}

/// Build the web interface and return a process-friendly result.
fn main() -> ExitCode {
    exit_code(run(&Cli::parse()), &mut io::stderr().lock())
}

/// Convert the domain result into a process exit code and one diagnostic.
fn exit_code(result: Result<(), SiteError>, stderr: &mut impl Write) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Write one stable diagnostic without panicking on a closed stderr stream.
            let diagnostic = format!("error: {error}\n");
            drop(stderr.write_all(diagnostic.as_bytes()));
            ExitCode::FAILURE
        }
    }
}

/// Build the web interface from the parsed paths.
fn run(cli: &Cli) -> Result<(), SiteError> {
    // Parse the registry before discovery so a bad list fails fast.
    let registered: RegisteredLints = read_to_string(&cli.lint_list)?.parse()?;
    let lints = site::generate_site(&cli.root, &registered, &cli.out_dir)?;
    let index = cli.out_dir.join("index.html");
    let (summary, warning) = site::report(&index, &lints);
    site::print_report(&summary, warning.as_deref()).map_err(|source| SiteError::Io {
        path: index,
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::{fs, io};

    use super::{Cli, ExitCode, exit_code, run};
    use crate::catalog::tests::{fixture_repository, repository_root};

    /// An output sink that exercises the closed-stderr failure path.
    struct BrokenWriter;

    impl Write for BrokenWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Successful command results map to a clean process status.
    #[test]
    fn success_uses_success_exit_code() {
        let mut stderr = Vec::new();
        assert_eq!(exit_code(Ok(()), &mut stderr), ExitCode::SUCCESS);
        assert!(stderr.is_empty());
    }

    /// Failed command results retain one path-aware stderr diagnostic.
    #[test]
    fn errors_use_failure_exit_code_and_print_diagnostic() {
        let mut stderr = Vec::new();
        let error = crate::error::SiteError::NoLints {
            path: "crates".into(),
        };
        assert_eq!(exit_code(Err(error), &mut stderr), ExitCode::FAILURE);
        assert_eq!(
            stderr,
            b"error: no lint UI directories found under crates\n"
        );
    }

    /// A broken stderr stream does not replace the command failure status.
    #[test]
    fn errors_do_not_panic_when_stderr_is_closed() {
        let error = crate::error::SiteError::NoLints {
            path: "crates".into(),
        };
        assert_eq!(exit_code(Err(error), &mut BrokenWriter), ExitCode::FAILURE);
    }

    /// A broken writer has no buffered bytes to flush.
    #[test]
    fn broken_writer_flush_succeeds() {
        assert!(BrokenWriter.flush().is_ok());
    }

    /// The command line runs end to end on a fixture repository.
    #[test]
    fn run_generates_the_site() {
        // Write the lint list next to the fixture and point the CLI at both.
        let root = fixture_repository();
        let lint_list = root.path().join("lints.txt");
        fs::write(&lint_list, "    fixable_style    warn    Fixable\n")
            .expect("lint list should be writable");
        let cli = Cli {
            root: root.path().to_path_buf(),
            lint_list,
            out_dir: root.path().join("public"),
        };

        // The run succeeds and leaves the page on disk.
        run(&cli).expect("the CLI run should succeed");
        assert!(root.path().join("public/index.html").is_file());
    }

    /// A missing lint list fails before discovery.
    #[test]
    fn missing_lint_list_fails() {
        // Name a lint list that does not exist.
        let root = fixture_repository();
        let cli = Cli {
            root: repository_root().to_path_buf(),
            lint_list: root.path().join("absent.txt"),
            out_dir: root.path().join("public"),
        };

        // The error names the missing file.
        assert!(run(&cli).is_err_and(|error| error.to_string().contains("absent.txt")));
    }
}
