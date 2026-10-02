//! Records the pinned compiler fallback used by direct Cargo installations.
//!
//! This build script resolves the compiler selected by Cargo, validates the
//! private rustc library layout, and exports the paths needed by the runner.
//! Portable bundles receive a launcher sentinel, while direct installations
//! retain the compiler library directory through an ELF runtime search path.
//! The generated values are build-time metadata and do not read target source
//! files or depend on the user's interactive shell configuration.

#[path = "support/compiler_sysroot.rs"]
mod compiler_sysroot;

use std::{env, error::Error, io, io::Write as _, path::Path};

/// Embed the selected toolchain paths and retain its compiler libraries at runtime.
fn main() -> Result<(), Box<dyn Error>> {
    // Read process configuration only at this Cargo-script bootstrap boundary.
    let rustc = env::var_os("RUSTC");
    let path = env::var_os("PATH");
    let target_is_unix = env::var_os("CARGO_CFG_UNIX").is_some();
    let is_portable_bundle = env::var_os("SAGAN_LINTS_PORTABLE_BUNDLE").is_some();

    // Resolve every executable from Cargo's selected compiler instead of host defaults.
    let sysroot = compiler_sysroot::from_cargo_environment(rustc, path.as_deref())?;

    let directives = build_directives(&sysroot, target_is_unix, is_portable_bundle)?;

    // Emit the complete protocol buffer through a fallible output operation.
    io::stdout().lock().write_all(directives.as_bytes())?;
    Ok(())
}

/// Build the environment and linker directives for the selected toolchain.
fn build_directives(
    sysroot: &Path,
    target_is_unix: bool,
    is_portable_bundle: bool,
) -> Result<String, Box<dyn Error>> {
    let compiler_libraries = validated_compiler_libraries(sysroot)?;
    let build_sysroot = build_sysroot(sysroot, is_portable_bundle);
    let compiler_libraries = compiler_libraries.to_string_lossy();

    // Assemble the environment values that the runner reads at startup.
    let mut directives = String::new();
    directives.push_str("cargo:rustc-env=SAGAN_LINTS_BUILD_SYSROOT=");
    directives.push_str(&build_sysroot);
    directives.push('\n');
    directives.push_str("cargo:rustc-env=SAGAN_LINTS_TOOLCHAIN=nightly-2026-07-15\n");

    // Add the compiler library directory to the native linker search path.
    directives.push_str("cargo:rustc-link-search=native=");
    directives.push_str(&compiler_libraries);
    directives.push('\n');

    // Keep direct Unix installations runnable without a wrapper executable.
    if target_is_unix && !is_portable_bundle {
        directives.push_str("cargo:rustc-link-arg=-Wl,-rpath,");
        directives.push_str(&compiler_libraries);
        directives.push('\n');
    }

    // Re-run this script when the selected compiler or packaging mode changes.
    directives.push_str("cargo:rerun-if-env-changed=RUSTC\n");
    directives.push_str("cargo:rerun-if-env-changed=SAGAN_LINTS_PORTABLE_BUNDLE\n");
    Ok(directives)
}

/// Validate the compiler directories needed by the generated linker metadata.
fn validated_compiler_libraries(sysroot: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    // Derive both required directories from the same Cargo-selected sysroot.
    let compiler_libraries = sysroot.join("lib");
    let compiler_bin = sysroot.join("bin");
    // Validate every directory before emitting linker metadata to dependents.
    for required in [&compiler_libraries, &compiler_bin] {
        if !required.exists() {
            let required = required.display();
            return Err(io::Error::other(format!(
                "required compiler asset does not exist: {required}"
            ))
            .into());
        }
    }
    // Return the library directory after the complete layout check succeeds.
    Ok(compiler_libraries)
}

/// Select the runtime sysroot value for direct and portable installations.
fn build_sysroot(sysroot: &Path, is_portable_bundle: bool) -> String {
    if is_portable_bundle {
        format!("/{}/launcher-required", env!("CARGO_PKG_NAME"))
    } else {
        sysroot.to_string_lossy().into_owned()
    }
}
