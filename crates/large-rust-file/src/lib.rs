#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builders are configured through side effects"
)]

//! A lint to check Rust source file size with separate production and total limits.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use dylint_support::{RustFileSizeViolation, rust_file_size_violation};
use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext, def_id::LOCAL_CRATE};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub LARGE_RUST_FILE,
    Warn,
    "Rust source files should stay under 1500 non-test lines and 2000 total lines",
    LargeRustFile
}

impl EarlyLintPass for LargeRustFile {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _krate: &Crate) {
        // Read each loaded local source file and retain only files over a configured limit.
        for file in loaded_rust_source_files(cx)
            .into_iter()
            .filter_map(|candidate| {
                let (first_line_len, violation) = file_violation(&candidate.path)?;
                Some(LargeFile {
                    span: first_line_span(candidate.start_pos, first_line_len),
                    violation,
                })
            })
        {
            emit_large_file_lint(cx, file.span, file.violation);
        }
    }
}

/// State used by the large file analysis.
struct LargeFile {
    /// span stored for this lint's analysis.
    span: Span,
    /// File-size violation selected for the diagnostic.
    violation: RustFileSizeViolation,
}

/// State used by the source candidate analysis.
struct SourceCandidate {
    /// path stored for this lint's analysis.
    path: PathBuf,
    /// start pos stored for this lint's analysis.
    start_pos: BytePos,
}

/// Helper for loaded rust source files analysis.
fn loaded_rust_source_files(cx: &EarlyContext<'_>) -> Vec<SourceCandidate> {
    // Resolve source-map paths against the target crate's current directory.
    let source_map = cx.sess().source_map();
    let files = source_map.files();
    let Some(crate_root) = std::env::current_dir().ok() else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for source_file in files.iter() {
        // Files imported from dependency metadata, such as macro definitions and the standard
        // library, belong to other crates even when their path is under this directory.
        if source_file.cnum != LOCAL_CRATE {
            continue;
        }

        // Retain unique local Rust files under the compiler's current directory.
        let Some(path) = local_source_file_path(source_file, &crate_root) else {
            continue;
        };
        if !seen.insert(path.clone()) || !is_rust_file(&path) {
            continue;
        }

        candidates.push(SourceCandidate {
            path,
            start_pos: source_file.start_pos,
        });
    }

    // Release the source-map guard before callers begin file I/O.
    drop(files);

    candidates
}

/// Helper for source file path analysis.
fn local_source_file_path(source_file: &SourceFile, crate_root: &Path) -> Option<PathBuf> {
    // Virtual, remapped, and imported paths may not be readable on the local host.
    let path = source_file.name.clone().into_local_path()?;
    crate_local_path(path, crate_root)
}

/// Resolve one path and retain it only when it belongs to the target crate tree.
fn crate_local_path(path: PathBuf, crate_root: &Path) -> Option<PathBuf> {
    // Resolve relative source-map paths before applying the package boundary.
    let path = if path.is_absolute() {
        path
    } else {
        crate_root.join(path)
    };
    path.starts_with(crate_root).then_some(path)
}

/// Read one source file and return its first-line length and size violation.
fn file_violation(path: &Path) -> Option<(u32, RustFileSizeViolation)> {
    let mut source = String::new();

    // Match the existing file-system lints by avoiding async/runtime dependencies in rustc.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut source).ok()?;

    let violation = rust_file_size_violation(&source)?;
    Some((first_line_len(&source), violation))
}

/// Return the first line len.
fn first_line_len(source: &str) -> u32 {
    source
        .lines()
        .next()
        .map_or(1, |line| u32::try_from(line.len()).unwrap_or(u32::MAX))
        .max(1)
}

/// Return the first line span.
fn first_line_span(start_pos: BytePos, first_line_len: u32) -> Span {
    let lo = start_pos;
    let hi = BytePos(lo.0.saturating_add(first_line_len));

    Span::new(lo, hi, SyntaxContext::root(), None)
}

/// Return whether rust file.
fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
}

/// Emit the large file lint diagnostic.
fn emit_large_file_lint(cx: &EarlyContext<'_>, span: Span, violation: RustFileSizeViolation) {
    // Render the selected total-line or non-test-line threshold violation.
    let message = match violation {
        RustFileSizeViolation::TotalLines(line_count) => {
            format!("Rust file has {line_count} total lines")
        }
        RustFileSizeViolation::NonTestLines(line_count) => {
            format!("Rust file has {line_count} non-test lines")
        }
    };

    // Keep the diagnostic text stable and short because compiletest normalizes fixture paths.
    // Point at the first source line because the violation applies to the whole file.
    cx.emit_span_lint(
        LARGE_RUST_FILE,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help("split large files by responsibility or module");
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::disallowed_methods,
        reason = "synchronous filesystem fixtures exercise this early lint's file parser"
    )]

    use super::{
        RustFileSizeViolation, file_violation, first_line_len, first_line_span, is_rust_file,
    };
    use rustc_span::BytePos;
    use std::path::Path;

    /// First-line measurement uses one byte for empty and blank input.
    #[test]
    fn measures_first_line_length() {
        assert_eq!(first_line_len(""), 1);
    }

    /// First-line measurement counts source bytes before the first newline.
    #[test]
    fn measures_nonempty_first_line() {
        assert_eq!(first_line_len("é\nrest"), 2);
    }

    /// Span construction saturates at the representable byte position.
    #[test]
    fn builds_first_line_span() {
        let span = first_line_span(BytePos(u32::MAX), 10);

        assert_eq!(span.lo(), BytePos(u32::MAX));
    }

    /// File suffix filtering accepts only Rust source files.
    #[test]
    fn identifies_rust_sources() {
        assert!(is_rust_file(Path::new("module.rs")) && !is_rust_file(Path::new("module.txt")));
    }

    /// Missing files are skipped without producing a size violation.
    #[test]
    fn skips_missing_files() {
        assert!(file_violation(Path::new("/missing/large-rust-file.rs")).is_none());
    }

    /// Files over the production threshold report non-test line violations.
    #[test]
    fn reports_non_test_file_violation() {
        // Generate a source file that exceeds the production line threshold.
        let directory =
            std::env::temp_dir().join(format!("large-rust-file-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("large.rs");
        let source = std::iter::repeat_n("fn value() {}\n", 1501).collect::<String>();
        std::fs::write(&path, source).unwrap();

        // Remove the fixture after classification so the test leaves no files behind.
        let violation = file_violation(&path);
        std::fs::remove_dir_all(directory).unwrap();

        assert!(matches!(
            violation,
            Some((_, RustFileSizeViolation::NonTestLines(1501)))
        ));
    }
}

/// Paths outside the compiler's current directory do not count toward the limit.
#[test]
fn keeps_only_paths_under_current_directory() {
    let crate_root = Path::new("/work/target-crate");
    let sysroot = PathBuf::from("/cache/rustup/toolchains/nightly/lib/rustlib/src/lib.rs");

    // Cover an absolute external path and one accepted relative source path.
    assert!(crate_local_path(sysroot, crate_root).is_none());
    assert_eq!(
        crate_local_path(PathBuf::from("src/lib.rs"), crate_root),
        Some(crate_root.join("src/lib.rs"))
    );
}
