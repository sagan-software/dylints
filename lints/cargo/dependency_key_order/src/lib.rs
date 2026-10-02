#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    reason = "the lint intentionally ignores diagnostic builders and slices validated ASCII Cargo table syntax"
)]

//! A lint to check for alphabetically sorted Cargo dependency entries.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub DEPENDENCY_KEY_ORDER,
    Warn,
    "Cargo dependency entries in the same block should be sorted alphabetically",
    DependencyKeyOrder
}

impl EarlyLintPass for DependencyKeyOrder {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let crate_root_span = krate.spans.inner_span;

        // Anchor the manifest scan to the Rust source that Cargo asked rustc to compile.
        let Some(crate_root) = crate_root_path(cx, crate_root_span) else {
            return;
        };

        let Some(manifest_path) = nearest_manifest(&crate_root) else {
            return;
        };

        // Generated or virtual source trees may not have readable manifests, so skip them.
        let Some(manifest) = read_manifest(&manifest_path) else {
            return;
        };

        for issue in manifest_info(&manifest).out_of_order_dependencies {
            emit_dependency_lint(cx, crate_root_span, &issue);
        }
    }
}

/// Read manifest for source-based analysis.
fn read_manifest(path: &Path) -> Option<String> {
    let mut manifest = String::new();

    // Lint passes run inside rustc, so keep manifest reading local and dependency-free.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut manifest).ok()?;

    Some(manifest)
}

/// State used by the manifest info analysis.
#[derive(Default)]
struct ManifestInfo {
    /// out of order dependencies stored for this lint's analysis.
    out_of_order_dependencies: Vec<DependencyOrderIssue>,
}

/// State used by the dependency order issue analysis.
#[derive(Debug, PartialEq, Eq)]
struct DependencyOrderIssue {
    /// table stored for this lint's analysis.
    table: String,
    /// dependency stored for this lint's analysis.
    dependency: String,
    /// previous dependency stored for this lint's analysis.
    previous_dependency: String,
}

/// Helper for manifest info analysis.
fn manifest_info(manifest: &str) -> ManifestInfo {
    // Track one dependency table, one contiguous block, and multiline nesting.
    let mut info = ManifestInfo::default();
    let mut active_dependency_table = None;
    let mut multiline_depth = 0;

    for line in manifest.lines() {
        if multiline_depth > 0 {
            // Multiline inline tables belong to the dependency that opened them, not to the
            // surrounding dependency table's ordering stream.
            multiline_depth = update_multiline_depth(multiline_depth, line);
            continue;
        }

        if starts_table(line) {
            // Flush the previous table before activating the next supported header.
            finish_dependency_table(&mut active_dependency_table, &mut info);
            active_dependency_table = table_header(line).and_then(dependency_table);
            continue;
        }

        let Some(table) = active_dependency_table.as_mut() else {
            continue;
        };

        if is_block_separator(line) {
            // Blank lines and comments begin a separately sorted dependency block.
            table.finish_block();
            continue;
        }

        let Some((key, value)) = key_value(line) else {
            table.finish_block();
            continue;
        };

        let Some(dependency) = dependency_entry_key(key) else {
            table.finish_block();
            continue;
        };

        // Record this key and suspend ordering while its multiline value continues.
        table.record_dependency(dependency);
        multiline_depth = initial_multiline_depth(value);
    }

    finish_dependency_table(&mut active_dependency_table, &mut info);

    info
}

/// Helper for finish dependency table analysis.
fn finish_dependency_table(table: &mut Option<DependencyTable>, info: &mut ManifestInfo) {
    let Some(table) = table.take() else {
        return;
    };

    // Diagnostics preserve manifest order so the first unsorted pair is the first item to fix.
    info.out_of_order_dependencies.extend(table.issues);
}

/// State used by the dependency table analysis.
struct DependencyTable {
    /// name stored for this lint's analysis.
    name: String,
    /// previous dependency stored for this lint's analysis.
    previous_dependency: Option<DependencyKey>,
    /// issues stored for this lint's analysis.
    issues: Vec<DependencyOrderIssue>,
}

impl DependencyTable {
    /// Helper for record dependency analysis.
    fn record_dependency(&mut self, dependency: &str) {
        let dependency = DependencyKey::new(dependency);
        if dependency.normalized.is_empty() {
            return;
        }

        if let Some(previous) = self.previous_dependency.as_ref()
            && previous.normalized != dependency.normalized
            && dependency.normalized < previous.normalized
        {
            self.issues.push(DependencyOrderIssue {
                table: self.name.clone(),
                dependency: dependency.original.clone(),
                previous_dependency: previous.original.clone(),
            });
        }

        // Repeated dotted fields for the same dependency should keep acting as one entry.
        // Advance ordering state only when the normalized dependency changes.
        if self
            .previous_dependency
            .as_ref()
            .is_none_or(|previous| previous.normalized != dependency.normalized)
        {
            self.previous_dependency = Some(dependency);
        }
    }

    /// Helper for finish block analysis.
    fn finish_block(&mut self) {
        // Blank lines and comments intentionally let manifests maintain sorted subgroups.
        self.previous_dependency = None;
    }
}

/// State used by the dependency key analysis.
struct DependencyKey {
    /// original stored for this lint's analysis.
    original: String,
    /// normalized stored for this lint's analysis.
    normalized: String,
}

impl DependencyKey {
    /// Helper for new analysis.
    fn new(key: &str) -> Self {
        let original = key.trim().to_owned();

        // Cargo package names are ASCII in practice; lowercase comparison avoids case-only churn.
        let normalized = original.to_ascii_lowercase();

        Self {
            original,
            normalized,
        }
    }
}

/// Helper for dependency table analysis.
fn dependency_table(header: &str) -> Option<DependencyTable> {
    is_dependency_section(header).then(|| DependencyTable {
        name: format!("[{header}]"),
        previous_dependency: None,
        issues: Vec::new(),
    })
}

/// Return whether dependency section.
fn is_dependency_section(table: &str) -> bool {
    matches!(
        table,
        "dependencies" | "dev-dependencies" | "build-dependencies" | "workspace.dependencies"
    ) || table.starts_with("target.")
        && (table.ends_with(".dependencies")
            || table.ends_with(".dev-dependencies")
            || table.ends_with(".build-dependencies"))
}

/// Helper for table header analysis.
fn table_header(line: &str) -> Option<&str> {
    // Reject comments, arrays of tables, and non-header lines.
    let line = line.trim_start();
    if line.starts_with('#') || line.starts_with("[[") || !line.starts_with('[') {
        return None;
    }

    // Accept inline comments after the closing bracket and reject empty names.
    let close = line.find(']')?;
    let table = line[1..close].trim();
    (!table.is_empty()).then_some(table)
}

/// Helper for starts table analysis.
fn starts_table(line: &str) -> bool {
    let line = line.trim_start();
    !line.starts_with('#') && line.starts_with('[')
}

/// Return whether block separator.
fn is_block_separator(line: &str) -> bool {
    let line = line.trim_start();
    line.is_empty() || line.starts_with('#')
}

/// Helper for key value analysis.
fn key_value(line: &str) -> Option<(&str, &str)> {
    // Ignore empty lines and full-line comments before splitting the assignment.
    let line = line.trim_start();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let (key, value) = line.split_once('=')?;
    Some((key.trim(), strip_inline_comment(value).trim()))
}

/// Helper for dependency entry key analysis.
fn dependency_entry_key(key: &str) -> Option<&str> {
    // Collapse dotted dependency fields to their package key and remove quotes.
    let dependency = key
        .split_once('.')
        .map_or(key, |(dependency, _)| dependency);
    let dependency = unquote_key(dependency.trim());

    (!dependency.is_empty()).then_some(dependency)
}

/// Helper for unquote key analysis.
fn unquote_key(key: &str) -> &str {
    // Remove matching TOML key quotes while leaving bare or malformed keys unchanged.
    let Some(quote) = key.as_bytes().first().copied() else {
        return key;
    };

    if quote != b'"' && quote != b'\'' {
        return key;
    }

    key.get(1..key.len().saturating_sub(1))
        .filter(|inner| key.as_bytes().last().copied() == Some(quote) && !inner.is_empty())
        .unwrap_or(key)
}

/// Helper for strip inline comment analysis.
fn strip_inline_comment(value: &str) -> &str {
    value.split_once('#').map_or(value, |(before, _)| before)
}

/// Helper for initial multiline depth analysis.
fn initial_multiline_depth(value: &str) -> usize {
    nesting_delta(value).max(0).unsigned_abs()
}

/// Helper for update multiline depth analysis.
fn update_multiline_depth(depth: usize, line: &str) -> usize {
    let delta = nesting_delta(line);

    if delta.is_negative() {
        depth.saturating_sub(delta.unsigned_abs())
    } else {
        depth.saturating_add(delta.unsigned_abs())
    }
}

/// Helper for nesting delta analysis.
fn nesting_delta(line: &str) -> isize {
    let line = strip_inline_comment(line);

    // This intentionally ignores strings; it only keeps obvious multiline inline tables from
    // turning their inner fields into fake dependency entries.
    line.chars().fold(0, |depth, ch| match ch {
        '{' | '[' => depth + 1,
        '}' | ']' => depth - 1,
        _ => depth,
    })
}

/// Emit the dependency lint diagnostic.
fn emit_dependency_lint(cx: &EarlyContext<'_>, span: Span, issue: &DependencyOrderIssue) {
    let dependency = issue.dependency.clone();
    let previous_dependency = issue.previous_dependency.clone();

    // The only available span is in Rust source, so include the manifest context in the message.
    cx.emit_span_lint(
        DEPENDENCY_KEY_ORDER,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(format!(
                "dependency `{dependency}` appears after `{previous_dependency}` but should sort before it"
            ));
            let _ = diag.help(
                "sort contiguous dependency entries alphabetically within each Cargo dependency table",
            );
        }),
    );
}

/// Helper for crate root path analysis.
fn crate_root_path(cx: &EarlyContext<'_>, span: Span) -> Option<PathBuf> {
    // Virtual or path-remapped inputs may not have a readable local path, so skip those rather
    // than guessing from the process working directory.
    cx.sess()
        .source_map()
        .span_to_filename(span)
        .into_local_path()
}

/// Helper for nearest manifest analysis.
fn nearest_manifest(crate_root: &Path) -> Option<PathBuf> {
    let mut directory = crate_root.parent();

    while let Some(path) = directory {
        // Cargo uses the nearest manifest for this crate root; stop at the first ancestor match.
        let candidate = path.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }

        // Continue toward the filesystem root until a package boundary appears.
        directory = path.parent();
    }

    None
}

/// Helper for flags unsorted dependency blocks in supported tables analysis.
#[test]
fn flags_unsorted_dependency_blocks_in_supported_tables() {
    let manifest = r#"
        [workspace.dependencies]
        zeta = "1.0.0"
        alpha = "1.0.0"

        [dependencies]
        serde.version = "1.0.0"
        anyhow = "1.0.0"

        [target.'cfg(unix)'.dev-dependencies]
        unix_zeta = "1.0.0"
        unix_alpha = "1.0.0"
    "#;

    let offenders = manifest_info(manifest).out_of_order_dependencies;

    assert_eq!(
        offenders,
        vec![
            DependencyOrderIssue {
                table: "[workspace.dependencies]".to_owned(),
                dependency: "alpha".to_owned(),
                previous_dependency: "zeta".to_owned(),
            },
            DependencyOrderIssue {
                table: "[dependencies]".to_owned(),
                dependency: "anyhow".to_owned(),
                previous_dependency: "serde".to_owned(),
            },
            DependencyOrderIssue {
                table: "[target.'cfg(unix)'.dev-dependencies]".to_owned(),
                dependency: "unix_alpha".to_owned(),
                previous_dependency: "unix_zeta".to_owned(),
            },
        ]
    );
}

/// Helper for skips sorted blocks separators subtables and multiline fields analysis.
#[test]
fn skips_sorted_blocks_separators_subtables_and_multiline_fields() {
    let manifest = r#"
        [dependencies]
        alpha = "1.0.0"
        beta = "1.0.0"

        zeta = "1.0.0"
        # comments split independent dependency groups
        gamma = "1.0.0"

        serde = {
            version = "1.0.0",
            features = [
                "derive",
            ],
        }
        anyhow = "1.0.0"

        [dependencies.tokio]
        version = "1.0.0"
        features = ["rt"]

        [target.'cfg(unix)'.build-dependencies]
        build_alpha = "1.0.0"
        build_zeta = "1.0.0"
    "#;

    let offenders = manifest_info(manifest).out_of_order_dependencies;

    assert_eq!(
        offenders,
        vec![DependencyOrderIssue {
            table: "[dependencies]".to_owned(),
            dependency: "anyhow".to_owned(),
            previous_dependency: "serde".to_owned(),
        }]
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
