//! Exposes the selected compiler's private-library directory to dependent lint crates.
//!
//! Cargo invokes this script while compiling the private rustc-facing support
//! library. It resolves the compiler selected by Cargo, validates the expected
//! toolchain layout, and writes the native search path through Cargo's build
//! script protocol. The emitted metadata lets dependent lint crates reuse the
//! same compiler libraries without guessing from the invoking shell.

#[path = "../compiler_sysroot.rs"]
mod compiler_sysroot;

use std::{env, error::Error, io, io::Write as _};

/// Report the selected compiler library path through Cargo's native-link contract.
fn main() -> Result<(), Box<dyn Error>> {
    // Read process configuration only at this Cargo-script bootstrap boundary.
    let rustc = env::var_os("RUSTC");
    let path = env::var_os("PATH");

    // Resolve the sysroot that belongs to Cargo's selected compiler.
    let sysroot = compiler_sysroot::from_cargo_environment(rustc, path.as_deref())?;

    // rustc_driver links against the matching LLVM library beside the sysroot tools.
    let compiler_libraries = sysroot.join("lib");

    // Reject an incomplete toolchain before dependent lint crates start linking.
    if !compiler_libraries.is_dir() {
        let compiler_libraries = compiler_libraries.display();
        return Err(io::Error::other(format!(
            "rustc compiler library directory does not exist: {compiler_libraries}"
        ))
        .into());
    }

    // Convert the validated path once before assembling Cargo's line-oriented protocol.
    let compiler_libraries = compiler_libraries.to_string_lossy();
    let directives = build_directives(&compiler_libraries);

    // Emit the complete protocol buffer through a fallible output operation.
    io::stdout().lock().write_all(directives.as_bytes())?;
    Ok(())
}

/// Build the link-search and metadata directives consumed by dependent crates.
fn build_directives(compiler_libraries: &str) -> String {
    // Tell rustc where the selected compiler's private libraries are installed.
    let mut directives = String::new();
    directives.push_str("cargo:rustc-link-search=native=");
    directives.push_str(compiler_libraries);
    directives.push('\n');

    // Preserve the validated path for dependent crates that consume build metadata.
    directives.push_str("cargo:metadata=compiler-libraries=");
    directives.push_str(compiler_libraries);
    directives.push('\n');
    directives.push_str("cargo:rerun-if-env-changed=RUSTC\n");
    directives
}
