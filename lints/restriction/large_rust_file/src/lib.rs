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
    path::{Component, Path, PathBuf},
};

use dylint_support::{RustFileSizeViolation, rust_file_size_violation};
use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext};

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
        for file in large_rust_files(cx) {
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

/// Return every loaded local Rust file that exceeds either size limit.
fn large_rust_files(cx: &EarlyContext<'_>) -> impl Iterator<Item = LargeFile> {
    loaded_rust_source_files(cx)
        .into_iter()
        .filter_map(|candidate| {
            // Read the local file instead of trusting in-memory text so the lint follows the
            // file-system contract and skips unreadable paths.
            let (first_line_len, violation) = file_violation(&candidate.path)?;

            Some(LargeFile {
                span: first_line_span(candidate.start_pos, first_line_len),
                violation,
            })
        })
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
        // Retain unique local Rust files outside support and toolchain trees.
        let Some(path) = local_source_file_path(source_file, &crate_root) else {
            continue;
        };

        if !seen.insert(path.clone())
            || !is_rust_file(&path)
            || is_support_crate_root(&path)
            || is_toolchain_source(&path)
        {
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

/// Return whether the path is a shared support crate root module.
fn is_support_crate_root(path: &Path) -> bool {
    path.components()
        .collect::<Vec<_>>()
        .windows(3)
        .any(|components| {
            matches!(
                components,
                [
                    Component::Normal(support),
                    Component::Normal(src),
                    Component::Normal(lib),
            ] if *support == "support" && *src == "src" && *lib == "lib.rs"
            )
        })
        || path
            .components()
            .any(|component| matches!(component, Component::Normal(name) if name == "support"))
}

/// Return whether the path belongs to the workspace-local Dylint toolchain.
fn is_toolchain_source(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if name == ".rustup-dylint" || name == "toolchains" || name == "rustlib"
        )
    })
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

/// External toolchain source must not count toward the target crate's file limit.
#[test]
fn excludes_external_source_files() {
    let crate_root = Path::new("/work/target-crate");
    let sysroot = PathBuf::from("/cache/rustup/toolchains/sagan-lints/lib/rustlib/src/lib.rs");
    let local_toolchain =
        crate_root.join(".rustup-dylint/toolchains/nightly/lib/rustlib/src/lib.rs");
    let cached_toolchain =
        crate_root.join(".cache/nix-ui/rustup/toolchains/nightly/lib/rustlib/src/lib.rs");
    let packaged_toolchain = PathBuf::from("/nix/store/toolchain/lib/rustlib/src/lib.rs");

    // Cover absolute external paths, nested toolchains, and one accepted relative source path.
    assert!(crate_local_path(sysroot, crate_root).is_none());
    assert!(is_toolchain_source(&local_toolchain));
    assert!(is_toolchain_source(&cached_toolchain));
    assert!(is_toolchain_source(&packaged_toolchain));
    assert_eq!(
        crate_local_path(PathBuf::from("src/lib.rs"), crate_root),
        Some(crate_root.join("src/lib.rs"))
    );
}
