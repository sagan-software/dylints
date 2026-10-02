//! Resolves the compiler sysroot without starting another compiler process.

use std::{
    env,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
};

/// Resolve Cargo's selected compiler and return its containing sysroot.
pub(super) fn from_cargo_environment(
    rustc: Option<OsString>,
    path: Option<&OsStr>,
) -> io::Result<PathBuf> {
    // Require the compiler selected by Cargo instead of guessing from the active shell.
    let rustc =
        rustc.ok_or_else(|| io::Error::other("Cargo did not provide the RUSTC executable"))?;
    let rustc = PathBuf::from(rustc);

    // Resolve a bare compiler name through the PATH captured at the Cargo-script boundary.
    let rustc = if rustc.components().count() == 1 {
        resolve_on_path(&rustc, path)?
    } else {
        rustc
    };

    // Canonicalization makes the inferred sysroot independent of symlinked compiler launchers.
    let rustc = fs::canonicalize(&rustc)?;

    validated_sysroot(&rustc)
}

/// Validate Cargo's canonical compiler layout and return its toolchain root.
fn validated_sysroot(rustc: &Path) -> io::Result<PathBuf> {
    // Validate the standard `<sysroot>/bin/rustc` layout before removing path components.
    let bin = rustc.parent().ok_or_else(|| {
        let rustc = rustc.display();
        io::Error::other(format!("rustc has no parent directory: {rustc}"))
    })?;
    if bin.file_name() != Some(OsStr::new("bin")) {
        let rustc = rustc.display();
        return Err(io::Error::other(format!(
            "rustc is not inside a toolchain bin directory: {rustc}"
        )));
    }

    // Return the validated toolchain root for both link-time and runtime configuration.
    bin.parent().map(Path::to_path_buf).ok_or_else(|| {
        let rustc = rustc.display();
        io::Error::other(format!("rustc has no toolchain sysroot: {rustc}"))
    })
}

/// Resolve one bare compiler name through the inherited executable path.
fn resolve_on_path(program: &Path, path: Option<&OsStr>) -> io::Result<PathBuf> {
    // A bare compiler cannot be resolved without Cargo's inherited executable path.
    let path = path.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "PATH is not set"))?;

    // Preserve PATH order so resolution matches the command lookup Cargo would inherit.
    for directory in env::split_paths(&path) {
        let candidate = directory.join(program);

        // Accept only a regular file as a compiler executable candidate.
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    // Report the unresolved program without exposing the full inherited PATH value.
    let program = program.display();
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("compiler executable was not found on PATH: {program}"),
    ))
}
