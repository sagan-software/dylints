#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    reason = "the lint intentionally ignores diagnostic builders and slices ASCII Cargo version syntax"
)]

//! A lint to check for dependency versions without full semver.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::{
    collections::BTreeMap,
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
    pub DEPENDENCY_FULL_SEMVER_VERSIONS,
    Warn,
    "dependency version requirements should include a full semver patch component",
    DependencyFullSemverVersions
}

impl EarlyLintPass for DependencyFullSemverVersions {
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

        for dependency in manifest_info(&manifest).offending_dependencies {
            emit_dependency_lint(cx, crate_root_span, &dependency);
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
    /// offending dependencies stored for this lint's analysis.
    offending_dependencies: Vec<OffendingDependency>,
}

/// State used by the offending dependency analysis.
#[derive(Debug, PartialEq, Eq)]
struct OffendingDependency {
    /// name stored for this lint's analysis.
    name: String,
    /// version stored for this lint's analysis.
    version: String,
}

/// Helper for manifest info analysis.
fn manifest_info(manifest: &str) -> ManifestInfo {
    // Accumulate dependencies within one active dependency table at a time.
    let mut info = ManifestInfo::default();
    let mut active_dependency_table = None;

    for line in manifest.lines() {
        if starts_table(line) {
            // Any TOML table boundary ends the previous dependency section.
            finish_dependency_table(&mut active_dependency_table, &mut info);
            active_dependency_table = table_header(line).and_then(dependency_table);
            continue;
        }

        let Some((key, value)) = key_value(line) else {
            continue;
        };

        // Record assignments only while a dependency table remains active.
        if let Some(table) = active_dependency_table.as_mut() {
            table.record(key, value);
        }
    }

    finish_dependency_table(&mut active_dependency_table, &mut info);

    info
}

/// Helper for finish dependency table analysis.
fn finish_dependency_table(table: &mut Option<DependencyTable>, info: &mut ManifestInfo) {
    let Some(table) = table.take() else {
        return;
    };

    // Sort dependency keys within each table so diagnostics stay stable across map iteration.
    info.offending_dependencies
        .extend(table.offending_dependencies());
}

/// State used by the dependency table analysis.
struct DependencyTable {
    /// entries stored for this lint's analysis.
    entries: BTreeMap<String, DependencySpec>,
    /// subtable dependency stored for this lint's analysis.
    subtable_dependency: Option<String>,
}

impl DependencyTable {
    /// Helper for record analysis.
    fn record(&mut self, key: &str, value: &str) {
        // Subtables apply every assignment to their single dependency name.
        if let Some(dependency) = self.subtable_dependency.clone() {
            self.record_dependency_field(&dependency, key, value);
            return;
        }

        // Split dotted keys before choosing whole-spec or field parsing.
        let (dependency, field) = dependency_key(key);
        if dependency.is_empty() {
            return;
        }

        if let Some(field) = field {
            self.record_dependency_field(dependency, field, value);
        } else {
            self.record_dependency_value(dependency, value);
        }
    }

    /// Helper for record dependency value analysis.
    fn record_dependency_value(&mut self, dependency: &str, value: &str) {
        let spec = self.entries.entry(dependency.to_owned()).or_default();

        // Cargo's string shorthand means `version = "...";` inline tables need field parsing.
        if let Some(version) = quoted_string(value) {
            spec.record_version(version);
        } else if let Some(body) = inline_table_body(value) {
            spec.record_inline_table(body);
        }
    }

    /// Helper for record dependency field analysis.
    fn record_dependency_field(&mut self, dependency: &str, field: &str, value: &str) {
        let spec = self.entries.entry(dependency.to_owned()).or_default();
        spec.record_field(field, value);
    }

    /// Helper for offending dependencies analysis.
    fn offending_dependencies(self) -> Vec<OffendingDependency> {
        self.entries
            .into_iter()
            .filter_map(|(name, spec)| {
                // Workspace-only dependencies inherit the version from elsewhere; do not warn
                // even if a malformed inline table contains both fields.
                if spec.inherits_workspace {
                    return None;
                }

                spec.incomplete_version
                    .map(|version| OffendingDependency { name, version })
            })
            .collect()
    }
}

/// State used by the dependency spec analysis.
#[derive(Default)]
struct DependencySpec {
    /// incomplete version stored for this lint's analysis.
    incomplete_version: Option<String>,
    /// inherits workspace stored for this lint's analysis.
    inherits_workspace: bool,
}

impl DependencySpec {
    /// Helper for record inline table analysis.
    fn record_inline_table(&mut self, body: &str) {
        for field in body.split(',') {
            // Splitting on commas is deliberately small; fields we do not understand are ignored.
            let Some((key, value)) = key_value(field) else {
                continue;
            };

            self.record_field(key, value);
        }
    }

    /// Helper for record field analysis.
    fn record_field(&mut self, field: &str, value: &str) {
        // Record workspace inheritance or a literal version field.
        let field = field.trim();

        if field == "workspace" && value_is_true(value) {
            self.inherits_workspace = true;
        } else if (field == "version" || field.ends_with(".version"))
            && let Some(version) = quoted_string(value)
        {
            self.record_version(version);
        } else if field.ends_with(".workspace") && value_is_true(value) {
            self.inherits_workspace = true;
        }
    }

    /// Helper for record version analysis.
    fn record_version(&mut self, version: &str) {
        if is_incomplete_bare_semver(version) {
            let _recorded_version = self
                .incomplete_version
                .get_or_insert_with(|| version.to_owned());
        }
    }
}

/// Helper for dependency table analysis.
fn dependency_table(header: &str) -> Option<DependencyTable> {
    if is_dependency_section(header) {
        return Some(DependencyTable {
            entries: BTreeMap::new(),
            subtable_dependency: None,
        });
    }

    dependency_subtable(header).map(|dependency| DependencyTable {
        entries: BTreeMap::new(),
        subtable_dependency: Some(dependency.to_owned()),
    })
}

/// Return whether dependency section.
fn is_dependency_section(table: &str) -> bool {
    matches!(
        table,
        "dependencies" | "dev-dependencies" | "build-dependencies" | "workspace.dependencies"
    )
}

/// Helper for dependency subtable analysis.
fn dependency_subtable(table: &str) -> Option<&str> {
    // Test each supported dependency-table prefix in stable order.
    for prefix in [
        "dependencies.",
        "dev-dependencies.",
        "build-dependencies.",
        "workspace.dependencies.",
    ] {
        if let Some(dependency) = table.strip_prefix(prefix) {
            return nonempty_dependency_name(dependency);
        }
    }

    None
}

/// Return the nonempty dependency name.
fn nonempty_dependency_name(name: &str) -> Option<&str> {
    let name = name.trim();
    (!name.is_empty()).then_some(name)
}

/// Helper for dependency key analysis.
fn dependency_key(key: &str) -> (&str, Option<&str>) {
    key.split_once('.').map_or_else(
        || (key.trim(), None),
        |(dependency, field)| (dependency.trim(), Some(field.trim())),
    )
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

/// Helper for strip inline comment analysis.
fn strip_inline_comment(value: &str) -> &str {
    value.split_once('#').map_or(value, |(before, _)| before)
}

/// Helper for value is true analysis.
fn value_is_true(value: &str) -> bool {
    value.trim() == "true"
}

/// Helper for quoted string analysis.
fn quoted_string(value: &str) -> Option<&str> {
    // Require a leading single or double quote after whitespace.
    let value = value.trim();
    let quote = value.as_bytes().first().copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }

    // Keep string handling conservative; escaped quotes and trailing tokens remain unsupported.
    let end = value[1..].find(char::from(quote))?;
    Some(&value[1..=end])
}

/// Helper for inline table body analysis.
fn inline_table_body(value: &str) -> Option<&str> {
    value.trim().strip_prefix('{')?.trim_end().strip_suffix('}')
}

/// Return whether incomplete bare semver.
fn is_incomplete_bare_semver(version: &str) -> bool {
    if version.is_empty() || !version.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return false;
    }

    let components: Vec<_> = version.split('.').collect();

    // A narrow bare-version match keeps ranges, wildcards, and pre-release requirements quiet.
    components.iter().all(|component| !component.is_empty()) && matches!(components.len(), 1 | 2)
}

/// Helper for suggested full version analysis.
fn suggested_full_version(version: &str) -> String {
    // Count numeric components before appending only the missing suffix.
    match version.split('.').count() {
        // A major-only requirement needs both minor and patch components.
        1 => format!("{version}.0.0"),
        // A major-minor requirement needs only the patch component.
        2 => format!("{version}.0"),
        _ => version.to_owned(),
    }
}

/// Emit the dependency lint diagnostic.
fn emit_dependency_lint(cx: &EarlyContext<'_>, span: Span, dependency: &OffendingDependency) {
    let dependency_name = dependency.name.clone();
    let version = dependency.version.clone();
    let suggestion = suggested_full_version(&version);

    // The only available span is in Rust source, so include the dependency name and version.
    cx.emit_span_lint(
        DEPENDENCY_FULL_SEMVER_VERSIONS,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(format!(
                "dependency `{dependency_name}` has incomplete semver version `{version}`"
            ));
            let _ = diag.help(format!(
                "write a full version such as `{suggestion}` for `{dependency_name}`"
            ));
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

/// Helper for flags bare semver strings and version fields analysis.
#[test]
fn flags_bare_semver_strings_and_version_fields() {
    let manifest = r#"
        [workspace.dependencies]
        workspace_bad = "1"

        [dependencies]
        shorthand = "1"
        inline = { version = "0.1" }

        [dev-dependencies.subtable]
        version = "2"
    "#;

    let offenders = manifest_info(manifest).offending_dependencies;

    assert_eq!(
        offenders,
        vec![
            OffendingDependency {
                name: "workspace_bad".to_owned(),
                version: "1".to_owned(),
            },
            OffendingDependency {
                name: "inline".to_owned(),
                version: "0.1".to_owned(),
            },
            OffendingDependency {
                name: "shorthand".to_owned(),
                version: "1".to_owned(),
            },
            OffendingDependency {
                name: "subtable".to_owned(),
                version: "2".to_owned(),
            },
        ]
    );
}

/// Helper for skips full ranges path only and workspace only entries analysis.
#[test]
fn skips_full_ranges_path_only_and_workspace_only_entries() {
    let manifest = r#"
        [workspace.dependencies]
        full = "1.2.3"
        range = ">=1.0.0"

        [dependencies]
        path_only = { path = "path_only" }
        git_only = { git = "https://example.invalid/repo.git" }
        workspace_only.workspace = true
        full_inline = { version = "0.1.0" }
        operator_inline = { version = ">=1.0.0" }
        pre_release = { version = "1.2.3-alpha.1" }
    "#;

    let offenders = manifest_info(manifest).offending_dependencies;

    assert_eq!(offenders, Vec::new());
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
