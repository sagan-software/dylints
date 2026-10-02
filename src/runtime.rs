//! Compiler runtime selected for the unified lint executable.

use std::{env, ffi::OsString, path::PathBuf};

use crate::error::RunnerError;

/// Exact compiler assets used by Clippy and the embedded rustc driver.
#[derive(Clone, Debug)]
pub(super) struct Runtime {
    /// Cargo from the compiler sysroot that built this executable.
    pub(super) cargo: PathBuf,
    /// rustc from the compiler sysroot that built this executable.
    pub(super) rustc: PathBuf,
    /// Compiler sysroot containing `rustc_driver` and its matching standard library.
    pub(super) sysroot: PathBuf,
    /// Pinned toolchain identity shown in diagnostics and dry runs.
    pub(super) toolchain_name: &'static str,
}

/// Destination-host sysroot supplied by the skill launcher.
const SYSROOT_ENV: &str = "SAGAN_LINTS_SYSROOT";
/// Exact nightly used to build every native bundle executable.
const TOOLCHAIN_NAME: &str = env!("SAGAN_LINTS_TOOLCHAIN");
/// Build-host fallback retained for direct `cargo install` users.
const BUILD_SYSROOT: &str = env!("SAGAN_LINTS_BUILD_SYSROOT");

impl Runtime {
    /// Load one matching destination-host toolchain and reject missing assets.
    pub(super) fn load() -> Result<Self, RunnerError> {
        // The launcher overrides the build path after resolving the pinned local nightly.
        let sysroot = sysroot()?;
        // Both tools must come from the same sysroot as the linked rustc driver.
        let executable_suffix = env::consts::EXE_SUFFIX;
        let cargo = required_file(
            sysroot
                .join("bin")
                .join(format!("cargo{executable_suffix}")),
        )?;
        let rustc = required_file(
            sysroot
                .join("bin")
                .join(format!("rustc{executable_suffix}")),
        )?;

        // Keep the resolved paths owned so child phases cannot observe a later environment change.
        Ok(Self {
            cargo,
            rustc,
            sysroot,
            toolchain_name: TOOLCHAIN_NAME,
        })
    }

    /// Return the directory prepended for Cargo, rustc, Clippy, and rustdoc discovery.
    pub(super) fn toolchain_bin(&self) -> PathBuf {
        self.sysroot.join("bin")
    }
}

/// Return the compiler sysroot to internal compiler-wrapper processes.
#[expect(
    runtime_env_read,
    reason = "the compiler wrapper reads its launcher boundary"
)]
pub(super) fn sysroot() -> Result<PathBuf, RunnerError> {
    // Keep one resolution rule for the parent runner and every re-entered compiler process.
    let path = selected_sysroot(env::var_os(SYSROOT_ENV));
    if path.is_dir() {
        Ok(path)
    } else {
        Err(RunnerError::MissingAsset { path })
    }
}

/// Prefer the destination-host path while retaining direct Cargo installation behavior.
fn selected_sysroot(runtime_sysroot: Option<OsString>) -> PathBuf {
    runtime_sysroot.map_or_else(|| PathBuf::from(BUILD_SYSROOT), PathBuf::from)
}

/// Return a required toolchain executable after validating it.
fn required_file(path: PathBuf) -> Result<PathBuf, RunnerError> {
    if path.is_file() {
        Ok(path)
    } else {
        Err(RunnerError::MissingAsset { path })
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::{BUILD_SYSROOT, selected_sysroot};

    /// Prove a relocated bundle never retains the build host's absolute sysroot.
    #[test]
    fn destination_sysroot_overrides_build_sysroot() {
        let destination = OsString::from("/destination/nightly");

        // Preserve direct installs while ensuring the launcher can relocate a bundled executable.
        assert_eq!(
            selected_sysroot(Some(destination)),
            PathBuf::from("/destination/nightly")
        );
        assert_eq!(selected_sysroot(None), PathBuf::from(BUILD_SYSROOT));
    }
}
