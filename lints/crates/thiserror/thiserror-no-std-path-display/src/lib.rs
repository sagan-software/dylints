#![feature(rustc_private)]

//! A lint to check for Path formatting with thiserror std disabled.
//!
//! This Dylint library resolves thiserror source files, reports path formatting
//! without the standard feature, and recommends an enabled feature or display.
//!
//! The README defines the supported source profile and replacement. UI fixtures
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
    pub THISERROR_NO_STD_PATH_DISPLAY,
    Warn,
    "`thiserror` formats a path field while its std feature is disabled",
    ThiserrorNoStdPathDisplay
}

impl EarlyLintPass for ThiserrorNoStdPathDisplay {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _krate: &Crate) {
        for candidate in loaded_rust_sources(cx) {
            check_source(cx, &candidate);
        }
    }
}

/// User source file plus its source-map offset for text-based diagnostics.
struct SourceCandidate {
    /// Filesystem path for manifest lookup.
    path: PathBuf,
    /// Full Rust source text for the candidate file.
    source: String,
    /// Byte position where this source file starts in rustc's source map.
    start_pos: BytePos,
}

/// Check one source file for thiserror path captures with `std` disabled.
fn check_source(cx: &EarlyContext<'_>, candidate: &SourceCandidate) {
    // Require both the no-std dependency profile and a thiserror derive.
    if !manifest_disables_thiserror_std(&candidate.path, &candidate.source) {
        return;
    }
    if !derives_thiserror_error(&candidate.source) {
        return;
    }

    for (start, end) in attr_ranges(&candidate.source, "error") {
        // Report only formats that directly capture a path-like field.
        let Some(attr) = candidate.source.get(start..end) else {
            continue;
        };
        if path_field_capture(&candidate.source, attr) {
            emit_span_lint_with_help(
                cx,
                THISERROR_NO_STD_PATH_DISPLAY,
                range_span(candidate.start_pos, start, end),
                "`thiserror` path capture needs its `std` feature",
                "enable thiserror's `std` feature, or format the field with `.display()`",
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

        // Preserve the path and source-map origin for manifest lookup and diagnostics.
        candidates.push(SourceCandidate {
            path,
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

/// Return whether the nearest manifest disables `thiserror` default features.
fn manifest_disables_thiserror_std(source_path: &Path, source: &str) -> bool {
    let Some(manifest_path) = nearest_manifest(source_path) else {
        return source_has_no_std_fixture_marker(source);
    };

    read_file(&manifest_path).is_some_and(|manifest| manifest_has_thiserror_no_std(&manifest))
}

/// Return whether copied UI fixture source carries the no-std marker comment.
fn source_has_no_std_fixture_marker(source: &str) -> bool {
    // Example UI tests are copied to a temp file without their Cargo manifest; this marker keeps
    // the rendering test focused while real lint runs use the package manifest path above.
    source.contains("thiserror-no-std-path-display: default-features = false")
}

/// Walk ancestors from a source path to find the nearest Cargo manifest.
fn nearest_manifest(crate_root: &Path) -> Option<PathBuf> {
    let mut directory = crate_root.parent();

    while let Some(path) = directory {
        // The nearest manifest is the compiled crate's package boundary.
        let candidate = path.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }

        // Continue toward the filesystem root until a package boundary appears.
        directory = path.parent();
    }

    None
}

/// Read a manifest or source file into a string.
fn read_file(path: &Path) -> Option<String> {
    let mut content = String::new();

    // Lint passes run inside rustc, so keep manifest reading local and dependency-free.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut content).ok()?;

    Some(content)
}

/// Return whether manifest text declares `thiserror` with default features disabled.
fn manifest_has_thiserror_no_std(manifest: &str) -> bool {
    // Track the active TOML table so workspace defaults do not hide package overrides.
    let mut section = "";
    // Prefer a package dependency because it is the setting rustc resolves for this source.
    let mut package_setting = None;
    // Retain the workspace setting for packages that inherit the dependency unchanged.
    let mut workspace_setting = None;
    for raw_line in manifest.lines() {
        // Strip comments before interpreting simple dependency assignments.
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            section = name.trim();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "thiserror" {
            continue;
        }
        // An explicit std feature wins over a workspace default-features=false declaration.
        let is_no_std = if value.contains("features") && value.contains("\"std\"") {
            false
        } else {
            value.contains("default-features") && value.contains("false")
        };
        if section == "workspace.dependencies" {
            workspace_setting = Some(is_no_std);
        } else if section.ends_with("dependencies") {
            package_setting = Some(is_no_std);
        }
    }
    package_setting.or(workspace_setting).unwrap_or(false)
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

/// Return whether an error format captures a field whose type looks path-like.
fn path_field_capture(source: &str, attr: &str) -> bool {
    let Some(format) = error_format_literal(attr) else {
        return false;
    };

    path_field_names(source)
        .into_iter()
        .any(|field| format.contains(&format!("{{{field}}}")))
}

/// Extract field names whose textual type is `Path` or `PathBuf`.
fn path_field_names(source: &str) -> Vec<String> {
    // Scan textual field declarations because cfg-disabled fields lack AST nodes.
    let mut fields = Vec::new();

    for line in source.lines() {
        // Retain only fields whose textual type mentions Path or PathBuf.
        let Some((name, ty)) = line.split_once(':') else {
            continue;
        };
        if !(ty.contains("PathBuf") || ty.contains("Path")) {
            continue;
        }

        // Remove preceding attributes and public visibility from the field name.
        let name = name.rsplit(']').next().unwrap_or(name).trim();
        let name = name.strip_prefix("pub ").unwrap_or(name).trim();
        if !name.is_empty() {
            fields.push(name.to_owned());
        }
    }

    fields
}

/// Parse the first string literal out of a raw source `#[error(...)]` attribute.
fn error_format_literal(attr: &str) -> Option<String> {
    // Locate the attribute argument list and require a leading string literal.
    let chars = error_literal_chars(attr)?;

    // Decode escaped characters while retaining the literal's logical contents.
    let mut literal = String::new();
    let mut escaped = false;
    for ch in chars {
        // Treat a backslash as escaping exactly the next source character.
        if escaped {
            literal.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(literal);
        } else {
            literal.push(ch);
        }
    }

    None
}

/// Return the characters after the opening quote of an error format literal.
fn error_literal_chars(attr: &str) -> Option<std::str::Chars<'_>> {
    let argument_start = attr.find('(')? + 1;
    let mut chars = attr.get(argument_start..)?.trim_start().chars();
    (chars.next() == Some('"')).then_some(chars)
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

/// Emit the path-display diagnostic with help text instead of a machine rewrite.
fn emit_span_lint_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // This spans a dependency feature choice or a format rewrite, so keep it help-only.
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

#[cfg(test)]
mod tests {
    use super::manifest_has_thiserror_no_std;

    #[test]
    fn package_feature_overrides_workspace_no_std_setting() {
        let manifest = r#"
[dependencies]
thiserror = { workspace = true, features = ["std"] }

[workspace.dependencies]
thiserror = { version = "2", default-features = false }
"#;

        assert!(!manifest_has_thiserror_no_std(manifest));
    }

    #[test]
    fn package_default_features_false_is_detected() {
        let manifest = r#"
[dependencies]
thiserror = { version = "2", default-features = false }
"#;

        assert!(manifest_has_thiserror_no_std(manifest));
    }
}
