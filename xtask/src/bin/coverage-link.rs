#![expect(
    unused_crate_dependencies,
    reason = "this linker wrapper shares the xtask package dependencies"
)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the linker wrapper preserves isolated filesystem outputs"
)]
//! Preserve each linked Dylint library before a constituent build replaces it.
//!
//! Cargo's target linker setting selects this wrapper for rustc 1.99.0-nightly.
//! It forwards compiler-generated Unix linker arguments unchanged to Dylint 6.0.3's
//! `dylint-link` and observes the same paired `-o` output option as that linker.
//! <https://doc.rust-lang.org/cargo/reference/environment-variables.html>
//! <https://doc.rust-lang.org/rustc/codegen-options/index.html#linker>
//! <https://github.com/trailofbits/dylint/blob/v6.0.3/dylint-link/src/main.rs>

use clap::Parser;
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Write as _},
    path::Path,
    process::{Command, ExitCode, ExitStatus},
};

/// Compiler-generated linker arguments, preserved in their original order and encoding.
#[derive(Debug, Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct Invocation {
    /// Opaque arguments owned by the selected linker.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
    arguments: Vec<OsString>,
}

/// Preserve linker failure and report archive failures separately.
fn main() -> ExitCode {
    // Parse linker-owned arguments without changing their values.
    let invocation = Invocation::parse();
    // Capture archive configuration once before executing the linker.
    let archive = env::var_os("DYLINT_COVERAGE_OBJECTS");
    match run(&invocation, archive.as_deref().map(Path::new)) {
        Ok(status) => ExitCode::from(
            status
                .code()
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1),
        ),
        Err(error) => {
            drop(writeln!(io::stderr().lock(), "coverage-link: {error}"));
            ExitCode::FAILURE
        }
    }
}

/// Archive only successfully linked shared libraries; executables need no extra copy.
fn run(invocation: &Invocation, archive: Option<&Path>) -> io::Result<ExitStatus> {
    // Complete native linking and Dylint's toolchain-named copy before preserving bytes.
    let status = Command::new("dylint-link")
        .args(&invocation.arguments)
        .status()?;
    if status.success()
        && let Some(library) = linked_library(&invocation.arguments)
    {
        let archive =
            archive.ok_or_else(|| io::Error::other("DYLINT_COVERAGE_OBJECTS is missing"))?;
        preserve(library, archive)?;
    }
    Ok(status)
}

/// Observe the Unix output option without consuming or changing linker arguments.
fn linked_library(arguments: &[OsString]) -> Option<&Path> {
    arguments.windows(2).find_map(|pair| match pair {
        [option, value] if option == "-o" => {
            // Resolve only the filename as text; parent directories retain their original encoding.
            let path = Path::new(value);
            let filename = path.file_name().and_then(OsStr::to_str)?;
            filename.ends_with(env::consts::DLL_SUFFIX).then_some(path)
        }
        _ => None,
    })
}

/// Retain exact linked bytes under a unique name before the next feature variant
/// is built.
fn preserve(library: &Path, archive: &Path) -> io::Result<()> {
    let retained = tempfile::Builder::new()
        .prefix("library-")
        .suffix(env::consts::DLL_SUFFIX)
        .tempfile_in(archive)?;
    let _copied_bytes = fs::copy(library, retained.path())?;
    let (_file, _path) = retained.keep().map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Linker arguments and archive failures have direct evidence without invoking a compiler.
    use super::{Invocation, linked_library, preserve};
    use clap::Parser as _;
    use std::{env, ffi::OsString, fs, os::unix::ffi::OsStringExt as _, path::Path};

    /// Compiler arguments retain flags, ordering, separators and non-UTF-8 values.
    #[test]
    fn retains_opaque_linker_arguments() {
        let opaque = OsString::from_vec(vec![0xff]);
        let args = vec![OsString::from("-o"), "output".into(), "--".into(), opaque];
        let cli = Invocation::try_parse_from(
            std::iter::once(OsString::from("coverage-link")).chain(args.clone()),
        )
        .expect("linker arguments");
        assert_eq!(cli.arguments, args);
    }

    /// An opaque directory name does not prevent preserving a shared-library output.
    #[test]
    fn recognizes_library_in_non_utf8_directory() {
        // Only the library filename needs text for the platform extension check.
        let directory = OsString::from_vec(vec![0xff]);
        let extension = env::consts::DLL_SUFFIX;
        let library = Path::new(&directory).join(format!("library{extension}"));
        let arguments = ["-o".into(), library.clone().into_os_string()];
        // Preserve the exact path, including its opaque parent directory.
        assert_eq!(linked_library(&arguments), Some(library.as_path()));
    }

    /// Only a shared-library output requires an archive copy.
    #[test]
    fn identifies_shared_library_outputs() {
        // A valid shared-library output retains the exact compiler-selected path.
        let extension = env::consts::DLL_SUFFIX;
        let filename = format!("library{extension}");
        let args = vec!["-o".into(), OsString::from(&filename)];
        let opaque_filename = OsString::from_vec(vec![0xff]);
        // Executables, missing values and unrelated options must not select an archive input.
        let observed = [
            linked_library(&args) == Some(Path::new(&filename)),
            linked_library(&["-o".into(), "executable".into()]).is_none(),
            linked_library(&["-o".into()]).is_none(),
            linked_library(&["--other".into(), OsString::from(filename)]).is_none(),
            linked_library(&["-o".into(), "/".into()]).is_none(),
            linked_library(&["-o".into(), opaque_filename]).is_none(),
        ];
        assert_eq!(observed, [true; 6]);
    }

    /// A missing library or invalid archive fails without substituting invented bytes.
    #[test]
    fn reports_archive_io_failures() {
        // A missing linked file fails before any valid bytes can be retained.
        let root = tempfile::tempdir().expect("archive root");
        let library = root.path().join("library");
        assert!(preserve(&library, root.path()).is_err());
        fs::write(&library, b"linked variant").expect("library");
        // A regular file cannot serve as the destination archive directory.
        let invalid_archive = root.path().join("file");
        fs::write(&invalid_archive, b"not a directory").expect("invalid archive");
        assert!(preserve(&library, &invalid_archive).is_err());
    }
}
