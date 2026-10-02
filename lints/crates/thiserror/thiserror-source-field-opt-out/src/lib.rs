#![feature(rustc_private)]

//! A lint to check for accidental thiserror source fields.
//!
//! This Dylint library resolves thiserror source fields, reports plain data
//! fields that would become error sources, and recommends an explicit opt-out.
//!
//! The README defines the supported source shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

#[cfg(test)]
use thiserror as _;

use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_SOURCE_FIELD_OPT_OUT,
    Warn,
    "`thiserror` treats a field named source as an error source",
    ThiserrorSourceFieldOptOut
}

impl EarlyLintPass for ThiserrorSourceFieldOptOut {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _krate: &Crate) {
        for candidate in loaded_rust_sources(cx) {
            check_source(cx, &candidate);
        }
    }
}

/// User source file plus its source-map offset for text-based diagnostics.
struct SourceCandidate {
    /// Full Rust source text for the candidate file.
    source: String,
    /// Byte position where this source file starts in rustc's source map.
    start_pos: BytePos,
}

/// Check one source file for plain data fields named `source`.
fn check_source(cx: &EarlyContext<'_>, candidate: &SourceCandidate) {
    // Skip files that cannot contain thiserror-owned fields.
    if !derives_thiserror_error(&candidate.source) {
        return;
    }

    // Inspect literal source fields and retain only likely plain data types.
    for (field_start, field_end) in source_field_ranges(&candidate.source) {
        let Some(field) = candidate.source.get(field_start..field_end) else {
            continue;
        };
        if likely_plain_data_source_field(field) {
            emit_span_lint_with_help(
                cx,
                THISERROR_SOURCE_FIELD_OPT_OUT,
                range_span(candidate.start_pos, field_start, field_end),
                "`thiserror` treats this `source` field as `Error::source()`",
                "rename ordinary data fields to `r#source` to opt out",
            );
        }
    }
}

/// Load user Rust source files from rustc's source map for fallback scanning.
fn loaded_rust_sources(cx: &EarlyContext<'_>) -> Vec<SourceCandidate> {
    // Deduplicate local Rust files exposed through rustc's source map.
    let files = cx.sess().source_map().files();
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for source_file in files.iter() {
        // Exclude repeated paths, non-Rust files, and dependency sources.
        let Some(path) = source_file_path(source_file) else {
            continue;
        };
        let is_already_seen = !seen.insert(path.clone());
        let not_rust_source = path.extension().is_none_or(|ext| ext != "rs");
        if is_already_seen || not_rust_source || is_dependency_source(&path) {
            continue;
        }

        let Some(source) = read_file(&path) else {
            continue;
        };

        // Preserve the source-map origin for diagnostic span reconstruction.
        candidates.push(SourceCandidate {
            source,
            start_pos: source_file.start_pos,
        });
    }

    // Release the source-map file guard before returning owned candidates.
    drop(files);

    candidates
}

/// Convert a rustc source-file name into a local filesystem path.
fn source_file_path(source_file: &SourceFile) -> Option<PathBuf> {
    source_file.name.clone().into_local_path()
}

/// Return whether a path belongs to dependency or compiler source that should be skipped.
fn is_dependency_source(path: &Path) -> bool {
    has_path_component(path, "rustc")
        || (has_path_component(path, ".cargo")
            && (has_path_component(path, "registry") || has_path_component(path, "git")))
}

/// Return whether a path contains one named normal component.
fn has_path_component(path: &Path, expected: &str) -> bool {
    path.components()
        .any(|component| matches!(component, Component::Normal(name) if name == expected))
}

/// Read a source file into a string for exact text fallback checks.
fn read_file(path: &Path) -> Option<String> {
    let mut source = String::new();

    // Source-file scanning keeps proc-macro-helper attributes visible before expansion.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut source).ok()?;

    Some(source)
}

/// Return whether raw source contains a derive attribute that names `Error`.
fn derives_thiserror_error(source: &str) -> bool {
    attr_ranges(source, "derive")
        .into_iter()
        .any(|(start, end)| {
            source.get(start..end).is_some_and(|attribute| {
                attribute
                    .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
                    .any(|segment| segment == "Error" || segment.ends_with("::Error"))
            })
        })
}

/// Return byte ranges for attributes whose body starts with the target name.
fn attr_ranges(source: &str, name: &str) -> Vec<(usize, usize)> {
    // Scan forward through complete outer attributes in source order.
    let mut ranges = Vec::new();
    let mut search_start = 0;

    while let Some(relative_start) = source.get(search_start..).and_then(|tail| tail.find("#[")) {
        // Convert the relative match and closing bracket into absolute offsets.
        let start = search_start + relative_start;
        let Some(end) = source
            .get(start..)
            .and_then(|attribute| attribute.find(']'))
            .map(|relative_end| start + relative_end + 1)
        else {
            break;
        };

        // Retain attributes whose body begins with the target name.
        let body = source
            .get(start + 2..end - 1)
            .map(str::trim_start)
            .unwrap_or_default();
        if body.starts_with(name) {
            ranges.push((start, end));
        }
        search_start = end;
    }

    ranges
}

/// Return source ranges for fields literally named `source`.
fn source_field_ranges(source: &str) -> Vec<(usize, usize)> {
    // Track absolute byte offsets while scanning source one line at a time.
    let mut ranges = Vec::new();
    let mut offset = 0;

    for line in source.lines() {
        // Match only fields whose literal identifier is `source`.
        let trimmed = line.trim_start();
        if trimmed.starts_with("source:") || trimmed.starts_with("pub source:") {
            let start = offset + line.find("source").unwrap_or(0);
            ranges.push((start, offset + line.len()));
        }
        offset += line.len() + 1;
    }

    ranges
}

/// Return whether a `source` field type looks like ordinary data rather than an error.
fn likely_plain_data_source_field(field: &str) -> bool {
    let Some((_, ty)) = field.split_once(':') else {
        return false;
    };
    let ty = ty.trim().trim_end_matches(',').trim();

    matches!(
        ty,
        "String"
            | "&str"
            | "str"
            | "char"
            | "bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "isize"
    )
}

/// Build a span for a byte range inside the source file currently being scanned.
fn range_span(start_pos: BytePos, start: usize, end: usize) -> Span {
    let lo = start_pos + byte_pos(start);
    let hi = start_pos + byte_pos(end);

    Span::new(lo, hi, SyntaxContext::root(), None)
}

/// Convert a source offset into rustc's bounded byte-position type.
fn byte_pos(offset: usize) -> BytePos {
    BytePos(u32::try_from(offset).expect("source file offset exceeds rustc BytePos range"))
}

/// Emit the source-field diagnostic with help text instead of a broad rewrite.
fn emit_span_lint_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Renaming a field can require constructor and pattern updates, so keep this help-only.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _configured_message = diag.primary_message(message);
            let _configured_help = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
