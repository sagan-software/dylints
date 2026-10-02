//! Web interface generation for the custom Rust lint catalog.
//!
//! The binary discovers lint README files, validates their shared documentation
//! contract, joins them with the runner's registered lint levels, and writes a
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
    io::{self, Write as _},
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

    /// Output of `sagan-lints --list-private-lints`, used for default lint levels.
    #[arg(long)]
    lint_list: PathBuf,

    /// Directory that receives `index.html` and its static assets.
    #[arg(long, default_value = "public")]
    out_dir: PathBuf,
}

/// Build the web interface and return a process-friendly result.
fn main() -> ExitCode {
    // Convert the domain result into the process exit contract.
    match run(&Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Write one stable diagnostic without panicking on a closed stderr stream.
            let diagnostic = format!("error: {error}\n");
            drop(io::stderr().lock().write_all(diagnostic.as_bytes()));
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
    use std::fs;

    use super::{Cli, run};
    use crate::catalog::tests::{fixture_repository, repository_root};

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
