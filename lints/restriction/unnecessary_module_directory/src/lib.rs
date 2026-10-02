#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builders are configured through side effects"
)]
#![allow(
    clippy::disallowed_methods,
    reason = "synchronous directory traversal is the behavior this filesystem lint inspects"
)]
//! A lint to check for module directories that contain only `mod.rs`.
//!
//! It maps rustc-loaded local source files to package directories, detects a
//! directory tree whose only file is `mod.rs`, and chooses flattening or
//! splitting from the shared Rust file-size policy. Filesystem traversal stays
//! synchronous because this early lint runs during compiler source analysis.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::{
    collections::BTreeSet,
    fs::{File, read_dir},
    io::Read,
    path::{Path, PathBuf},
};

use dylint_support::rust_file_size_violation;
use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub UNNECESSARY_MODULE_DIRECTORY,
    Warn,
    "module directory contains only mod.rs",
    UnnecessaryModuleDirectory
}

impl EarlyLintPass for UnnecessaryModuleDirectory {
    /// Check every loaded module file after rustc has resolved module paths.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _krate: &Crate) {
        for directory in unnecessary_module_directories(cx) {
            emit_unnecessary_module_directory(cx, &directory);
        }
    }
}

/// One module directory that contains only its loaded `mod.rs` file.
#[derive(Debug)]
struct ModuleDirectory {
    /// First-line span used to identify the complete module file.
    span: Span,
    /// Size-aware restructuring action shown to the user.
    recommendation: ModuleDirectoryRecommendation,
}

/// Restructuring action selected from the shared Rust file-size policy.
#[derive(Debug, Eq, PartialEq)]
enum ModuleDirectoryRecommendation {
    /// Replace the directory with a conventional sibling module file.
    Flatten {
        /// File name for the flattened module.
        module_file_name: PathBuf,
    },
    /// Keep the directory and split the oversized module into child modules.
    Split,
}

/// One local source-map file that may represent a folder module.
#[derive(Debug)]
struct SourceCandidate {
    /// Canonical local source path.
    path: PathBuf,
    /// Byte position of the file's first source character.
    start_pos: BytePos,
}

/// Return loaded folder modules whose directory tree has no second file.
fn unnecessary_module_directories(cx: &EarlyContext<'_>) -> Vec<ModuleDirectory> {
    loaded_mod_rs_files(cx)
        .iter()
        .filter_map(module_directory)
        .collect()
}

/// Return loaded local `mod.rs` files inside the current Cargo package.
fn loaded_mod_rs_files(cx: &EarlyContext<'_>) -> Vec<SourceCandidate> {
    // Anchor containment to the compiled crate root, then to its nearest package manifest.
    let Some(crate_source_path) = crate_source_path(cx) else {
        return Vec::new();
    };
    let Some(package_root) = nearest_package_root(&crate_source_path) else {
        return Vec::new();
    };
    let source_map = cx.sess().source_map();
    let files = source_map.files();
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    // Inspect each source-map entry in compiler order while deduplicating paths.
    for source_file in files.iter() {
        // Resolve each candidate before applying package and file-name filters.
        // Skip virtual, external, duplicate, crate-root, and ordinary module files.
        let Some(path) = local_source_file_path(source_file) else {
            continue;
        };
        let is_crate_root = path == crate_source_path;
        let is_outside_package = !path.starts_with(&package_root);
        let is_mod_rs = path.file_name().is_some_and(|name| name == "mod.rs");
        let is_new_path = seen.insert(path.clone());
        if is_crate_root || is_outside_package || !is_mod_rs || !is_new_path {
            continue;
        }

        candidates.push(SourceCandidate {
            path,
            start_pos: source_file.start_pos,
        });
    }

    // Release the source-map guard before callers inspect the file system.
    drop(files);
    candidates
}

/// Resolve the source file rustc compiled as this crate's root.
fn crate_source_path(cx: &EarlyContext<'_>) -> Option<PathBuf> {
    let path = cx.sess().local_crate_source_file()?.into_local_path()?;
    absolute_canonical_path(path)
}

/// Find the nearest Cargo package root above one compiled source file.
fn nearest_package_root(source_path: &Path) -> Option<PathBuf> {
    source_path
        .parent()?
        .ancestors()
        .find(|directory| directory.join("Cargo.toml").is_file())
        .map(Path::to_path_buf)
}

/// Resolve one source-map file to a canonical local path.
fn local_source_file_path(source_file: &SourceFile) -> Option<PathBuf> {
    let path = source_file.name.clone().into_local_path()?;
    absolute_canonical_path(path)
}

/// Resolve one local path against the compiler process and canonicalize it.
fn absolute_canonical_path(path: PathBuf) -> Option<PathBuf> {
    // Resolve relative source-map paths before canonicalizing package containment.
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    path.canonicalize().ok()
}

/// Build a lint candidate when `mod.rs` is its directory tree's sole file.
fn module_directory(candidate: &SourceCandidate) -> Option<ModuleDirectory> {
    // Require a directory and confirm that its tree contains no second file.
    let directory = candidate.path.parent()?;
    is_only_file(&candidate.path, directory).then_some(())?;

    // Read the exact local source used by the sibling file-size lint policy.
    let source = module_source(&candidate.path)?;
    // Select the safe restructuring action before building the diagnostic.
    let recommendation = recommendation_for_source(&candidate.path, &source)?;

    Some(ModuleDirectory {
        span: first_line_span(candidate.start_pos, first_line_len(&source)),
        recommendation,
    })
}

/// Reads one local module source without changing its original text.
fn module_source(path: &Path) -> Option<String> {
    let mut source = String::new();
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut source).ok()?;
    Some(source)
}

/// Return whether `mod.rs` is the directory tree's only readable file.
fn is_only_file(mod_rs_path: &Path, directory: &Path) -> bool {
    let mut pending_directories = vec![directory.to_path_buf()];
    let mut saw_mod_rs = false;

    // Walk nested directories until every reachable entry has been classified.
    while let Some(current_directory) = pending_directories.pop() {
        let Ok(entries) = read_dir(current_directory) else {
            return false;
        };

        // Reject unreadable entries before they can be mistaken for module source.
        for entry in entries {
            let Ok(entry) = entry else {
                return false;
            };
            let Ok(file_type) = entry.file_type() else {
                return false;
            };
            let path = entry.path();

            // Empty nested directories add no module source file.
            if file_type.is_dir() {
                pending_directories.push(path);
            } else if file_type.is_file() && path == mod_rs_path {
                saw_mod_rs = true;
            } else {
                // A second file or an uncertain special entry preserves the directory.
                return false;
            }
        }
    }

    saw_mod_rs
}

/// Select flattening or splitting from the shared file-size policy.
fn recommendation_for_source(
    mod_rs_path: &Path,
    source: &str,
) -> Option<ModuleDirectoryRecommendation> {
    // Oversized modules retain the directory so splitting stays possible.
    if rust_file_size_violation(source).is_some() {
        return Some(ModuleDirectoryRecommendation::Split);
    }

    // Flatten only modules that remain below the shared source-size threshold.
    let module_name = mod_rs_path.parent()?.file_name()?.to_string_lossy();
    Some(ModuleDirectoryRecommendation::Flatten {
        module_file_name: PathBuf::from(format!("{module_name}.rs")),
    })
}

impl ModuleDirectoryRecommendation {
    /// Render the exact size-aware restructuring guidance.
    fn help(&self) -> String {
        // Keep each closed recommendation variant tied to one stable diagnostic action.
        match self {
            Self::Flatten { module_file_name } => {
                format!(
                    "move this module to `{}` and remove the directory",
                    module_file_name.display()
                )
            }
            Self::Split => {
                // Preserve the directory when the shared file-size policy requires splitting.
                "`mod.rs` exceeds the `large_rust_file` thresholds; split it into child modules instead of flattening the directory".to_owned()
            }
        }
    }
}

/// Return the first physical source-line length.
fn first_line_len(source: &str) -> u32 {
    source
        .lines()
        .next()
        .map_or(1, |line| u32::try_from(line.len()).unwrap_or(u32::MAX))
        .max(1)
}

/// Build a span covering the first physical source line.
fn first_line_span(start_pos: BytePos, first_line_len: u32) -> Span {
    let hi = BytePos(start_pos.0.saturating_add(first_line_len));
    Span::new(start_pos, hi, SyntaxContext::root(), None)
}

/// Emit one module-directory diagnostic with the selected restructuring action.
fn emit_unnecessary_module_directory(cx: &EarlyContext<'_>, directory: &ModuleDirectory) {
    let help = directory.recommendation.help();

    // Point at the complete first line because the finding applies to the source file layout.
    cx.emit_span_lint(
        UNNECESSARY_MODULE_DIRECTORY,
        directory.span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message("`mod.rs` is the only file in this module directory");
            let _ = diag.help(help);
        }),
    );
}

/// Run the compiletest UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fs, path::Path};

    use super::{ModuleDirectoryRecommendation, is_only_file, recommendation_for_source};

    /// Empty nested directories do not satisfy the two-file requirement.
    #[test]
    fn ignores_empty_nested_directories() -> Result<(), Box<dyn Error>> {
        // Create one source file plus an empty descendant directory to isolate file counting.
        let directory = tempfile::tempdir()?;
        let mod_rs_path = directory.path().join("mod.rs");
        fs::write(&mod_rs_path, "pub fn value() {}")?;
        fs::create_dir(directory.path().join("empty"))?;

        assert!(is_only_file(&mod_rs_path, directory.path()));
        Ok(())
    }

    /// The shared file-size threshold changes the recommended restructuring action.
    #[test]
    fn recommends_splitting_an_oversized_single_file_module() {
        let path = Path::new("/work/large_module/mod.rs");
        let small_source = "pub fn run() {}";
        let large_source = std::iter::repeat_n("// production", 1500)
            .collect::<Vec<_>>()
            .join("\n");

        // Cover both actions at the exact production-line threshold used by `large_rust_file`.
        assert_eq!(
            recommendation_for_source(path, small_source),
            Some(ModuleDirectoryRecommendation::Flatten {
                module_file_name: Path::new("large_module.rs").to_owned(),
            })
        );
        assert_eq!(
            recommendation_for_source(path, &large_source),
            Some(ModuleDirectoryRecommendation::Split)
        );
    }

    /// The emitted help explains why an oversized module should retain its directory.
    #[test]
    fn renders_size_aware_split_guidance() {
        assert_eq!(
            ModuleDirectoryRecommendation::Split.help(),
            "`mod.rs` exceeds the `large_rust_file` thresholds; split it into child modules instead of flattening the directory"
        );
    }
}
