//! Compiler workloads shared by the Criterion harness and its integration
//! tests.

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

/// One independently loadable lint and its benchmark identity.
#[derive(Debug)]
pub(super) struct Lint {
    /// Cargo package name, also used in the Dylint library filename.
    pub(super) name: String,
    /// Shared library built by the pinned development toolchain.
    pub(super) library: PathBuf,
}

/// Discover leaf lint packages from Cargo metadata rather than a second lint
/// list.
///
/// # Panics
///
/// Panics when Cargo metadata omits artifact directories, packages, target fields,
/// or an eligible package name.
pub(super) fn lints(metadata: &serde_json::Value, toolchain: &str) -> Vec<Lint> {
    // Prefer Cargo's separate build directory when dylint-link stores libraries there.
    let artifacts = metadata
        .get("build_directory")
        .and_then(serde_json::Value::as_str)
        .or_else(|| metadata["target_directory"].as_str())
        .expect("Cargo artifact directory");
    // Retain the inventory because registration and aggregate loading both use it.
    let mut lints: Vec<_> = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .filter_map(|package| leaf_lint(package, metadata, toolchain, Path::new(artifacts)))
        .collect();
    // Keep benchmark identities deterministic across Cargo metadata ordering.
    lints.sort_by(|left, right| left.name.cmp(&right.name));
    lints
}

/// Resolve one documented leaf package to its independently loadable library.
fn leaf_lint(
    package: &serde_json::Value,
    metadata: &serde_json::Value,
    toolchain: &str,
    target: &Path,
) -> Option<Lint> {
    let directory = package_directory(package)?;
    // Categories can have UI tests but cannot serve as independent leaf libraries.
    if is_category(metadata, directory) {
        return None;
    }
    // Documentation and a workload distinguish leaf lints from support packages.
    let has_documentation = directory.join("README.md").is_file();
    let has_workload = directory.join("ui").is_dir() || directory.join("examples").is_dir();
    if !has_documentation || !has_workload || !is_dynamic(package) {
        return None;
    }
    Some(library(package, toolchain, target))
}

/// Borrow a package directory only when Cargo supplies a valid manifest path.
fn package_directory(package: &serde_json::Value) -> Option<&Path> {
    // Missing or non-text paths cannot identify an eligible leaf package.
    let manifest = Path::new(package["manifest_path"].as_str()?);
    manifest.parent()
}

/// Check whether Cargo builds this package as a dynamic library.
fn is_dynamic(package: &serde_json::Value) -> bool {
    package["targets"]
        .as_array()
        .expect("targets")
        .iter()
        .any(|target| {
            target["crate_types"]
                .as_array()
                .expect("crate types")
                .iter()
                .any(|kind| kind == "cdylib")
        })
}

/// Match dylint-link's platform prefix, normalized name, toolchain, and suffix.
fn library(package: &serde_json::Value, toolchain: &str, target: &Path) -> Lint {
    let name = package["name"].as_str().expect("package name").to_owned();
    // Cargo normalizes hyphens in library names, while benchmark names retain package spelling.
    let library_name = name.replace('-', "_");
    let prefix = std::env::consts::DLL_PREFIX;
    let suffix = std::env::consts::DLL_SUFFIX;
    // Compose the linker filename only after Cargo name normalization.
    let filename = format!("{prefix}{library_name}@{toolchain}{suffix}");
    Lint {
        name,
        library: target.join("debug").join(filename),
    }
}

/// Identify categories from the workspace's existing Dylint library
/// declarations.
fn is_category(metadata: &serde_json::Value, directory: &Path) -> bool {
    metadata
        .pointer("/metadata/dylint/libraries")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .any(|library| {
            library["path"].as_str().is_some_and(|path| {
                Path::new(metadata["workspace_root"].as_str().expect("workspace root")).join(path)
                    == directory
            })
        })
}

/// Create a standalone Cargo project whose source exercises compiler lint
/// callbacks.
///
/// Setup and file generation occur before any timed iteration.
///
/// # Errors
///
/// Returns an error when a workload file cannot be written.
pub(super) fn workload(
    directory: &Path,
    modules: usize,
    functions: usize,
) -> std::io::Result<PathBuf> {
    // Isolate configuration discovery from the surrounding lint workspace.
    initialize_project(directory)?;
    // Write module files before exposing the crate root to the compiler.
    let root = source_tree(directory, modules, functions)?;
    let path = directory.join("src/lib.rs");
    fs::write(&path, root)?;
    Ok(path)
}

/// Create the standalone manifest, pinned toolchain, and empty Clippy
/// configuration.
fn initialize_project(directory: &Path) -> std::io::Result<()> {
    // Create the source directory before writing any project or module files.
    fs::create_dir_all(directory.join("src"))?;
    fs::write(
        directory.join("Cargo.toml"),
        "[package]\nname = \"benchmark_workload\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n",
    )?;
    // Configuration discovery must use the same nightly and defaults as analysis benchmarks.
    fs::write(
        directory.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"nightly-2026-07-15\"\n",
    )?;
    fs::write(directory.join("clippy.toml"), "")
}

/// Write equal module workloads and return their ordered crate-root
/// declarations.
fn source_tree(directory: &Path, modules: usize, functions: usize) -> std::io::Result<String> {
    // Accumulate only the small crate-root declarations; module bodies go directly to files.
    let mut root = String::new();
    // Preserve module order and generate equal function bodies in each module.
    for module in 0..modules {
        let _written = writeln!(root, "mod part_{module};");
        fs::write(
            directory.join(format!("src/part_{module}.rs")),
            part_source(functions),
        )?;
    }
    Ok(root)
}

/// Generate functions with owned inputs, branches, loops, calls, and public
/// items.
fn part_source(functions: usize) -> String {
    // Each module owns its generated source until the setup write completes.
    let mut source = String::new();
    // Keep function bodies identical while retaining unique Rust identifiers.
    for function in 0..functions {
        let _written = write!(
            source,
            "/// Compute a filtered sum.\npub fn compute_{function}(values: Vec<u32>, limit: u32) -> u32 {{\n    let mut total = 0;\n    for value in values {{\n        if value > limit && value % 2 == 0 {{ total += value; }}\n    }}\n    total\n}}\n"
        );
    }
    source
}
