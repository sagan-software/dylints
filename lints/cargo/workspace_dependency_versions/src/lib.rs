#![feature(rustc_private)]
#![expect(
    clippy::string_slice,
    clippy::let_underscore_must_use,
    reason = "the parser slices ASCII TOML delimiters and configures diagnostics in place"
)]

//! A lint to check for workspace packages with local dependency versions.
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
    pub WORKSPACE_DEPENDENCY_VERSIONS,
    Warn,
    "workspace package dependencies should inherit versions from workspace dependencies",
    WorkspaceDependencyVersions
}

impl EarlyLintPass for WorkspaceDependencyVersions {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let crate_root_span = krate.spans.inner_span;

        // Anchor the repository-structure check to the crate root that rustc is compiling.
        let Some(crate_root) = crate_root_path(cx, crate_root_span) else {
            return;
        };

        let Some(manifest_path) = nearest_manifest(&crate_root) else {
            return;
        };

        // Unreadable manifests can happen for generated or virtual sources; skip those instead of
        // guessing from the process working directory.
        let Some(manifest) = read_manifest(&manifest_path) else {
            return;
        };

        let manifest_info = manifest_info(&manifest);
        if !should_check_workspace_dependencies(&manifest_path, &manifest_info) {
            return;
        }

        for dependency in manifest_info.offending_dependencies {
            emit_dependency_lint(cx, crate_root_span, &dependency.name);
        }
    }
}

/// Helper for should check workspace dependencies analysis.
fn should_check_workspace_dependencies(manifest_path: &Path, info: &ManifestInfo) -> bool {
    if !info.has_package || info.offending_dependencies.is_empty() {
        return false;
    }

    info.participates_in_workspace || ancestor_manifest_has_workspace(manifest_path)
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
    /// has package stored for this lint's analysis.
    has_package: bool,
    /// has example target stored for this lint's analysis.
    has_example: bool,
    /// participates in workspace stored for this lint's analysis.
    participates_in_workspace: bool,
    /// offending dependencies stored for this lint's analysis.
    offending_dependencies: Vec<OffendingDependency>,
}

/// State used by the offending dependency analysis.
struct OffendingDependency {
    /// name stored for this lint's analysis.
    name: String,
    /// is dev dependency stored for this lint's analysis.
    is_dev_dependency: bool,
}

/// Helper for manifest info analysis.
fn manifest_info(manifest: &str) -> ManifestInfo {
    let mut info = ManifestInfo::default();
    let mut active_dependency_table = None;

    for line in manifest.lines() {
        if starts_table(line) {
            // Any TOML table boundary ends the previous dependency section. Array tables such as
            // `[[bin]]` are not parsed further, but they still prevent following keys from being
            // treated as dependencies.
            finish_dependency_table(&mut active_dependency_table, &mut info);

            if array_table_header(line) == Some("example") {
                info.has_example = true;
            }

            let Some(header) = table_header(line) else {
                active_dependency_table = None;
                continue;
            };

            info.has_package |= header == "package";
            info.participates_in_workspace |= is_workspace_table(header);
            active_dependency_table = dependency_table(header);
            continue;
        }

        let Some((key, value)) = key_value(line) else {
            continue;
        };

        // Workspace-inherited package fields are enough evidence that the package is a workspace
        // member even when this manifest is compiled outside `cargo metadata`.
        if workspace_inherited_key(key, value) {
            info.participates_in_workspace = true;
        }

        if let Some(table) = active_dependency_table.as_mut() {
            table.record(key, value);
        }
    }

    finish_dependency_table(&mut active_dependency_table, &mut info);

    if info.has_example {
        // Example crates often pin dev-dependencies to exercise version-specific upstream
        // behavior; those fixture pins should not force the whole workspace dependency table.
        info.offending_dependencies
            .retain(|dependency| !dependency.is_dev_dependency);
    }

    info
}

/// Helper for finish dependency table analysis.
fn finish_dependency_table(table: &mut Option<DependencyTable>, info: &mut ManifestInfo) {
    let Some(table) = table.take() else {
        return;
    };

    // Preserve manifest order well enough for stable diagnostics by letting each table sort its
    // dependency keys before appending.
    info.offending_dependencies
        .extend(table.offending_dependencies());
}

/// State used by the dependency table analysis.
struct DependencyTable {
    /// entries stored for this lint's analysis.
    entries: BTreeMap<String, DependencySpec>,
    /// is dev dependencies stored for this lint's analysis.
    is_dev_dependencies: bool,
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

        // A string dependency value is Cargo's shorthand for `version = "...";` inline tables need
        // their fields scanned separately.
        if value_is_string(value) {
            spec.has_explicit_version = true;
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
            .filter(|(_, spec)| spec.has_explicit_version && !spec.inherits_workspace)
            .map(|(name, _)| OffendingDependency {
                name,
                is_dev_dependency: self.is_dev_dependencies,
            })
            .collect()
    }
}

/// State used by the dependency spec analysis.
#[derive(Default)]
struct DependencySpec {
    /// has explicit version stored for this lint's analysis.
    has_explicit_version: bool,
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
        // Record workspace inheritance or an explicit version field.
        let field = field.trim();

        if field == "workspace" && value_is_true(value) {
            self.inherits_workspace = true;
        } else if field == "version" || field.ends_with(".version") {
            self.has_explicit_version = true;
        } else if field.ends_with(".workspace") && value_is_true(value) {
            self.inherits_workspace = true;
        }
    }
}

/// Helper for dependency table analysis.
fn dependency_table(header: &str) -> Option<DependencyTable> {
    if is_dependency_section(header) {
        return Some(DependencyTable {
            entries: BTreeMap::new(),
            is_dev_dependencies: header.ends_with("dev-dependencies"),
            subtable_dependency: None,
        });
    }

    dependency_subtable(header).map(|dependency| DependencyTable {
        entries: BTreeMap::new(),
        is_dev_dependencies: header.starts_with("dev-dependencies.")
            || header.contains(".dev-dependencies."),
        subtable_dependency: Some(dependency.to_owned()),
    })
}

/// Return whether dependency section.
fn is_dependency_section(table: &str) -> bool {
    matches!(
        table,
        "dependencies" | "dev-dependencies" | "build-dependencies"
    ) || table.starts_with("target.")
        && (table.ends_with(".dependencies")
            || table.ends_with(".dev-dependencies")
            || table.ends_with(".build-dependencies"))
}

/// Helper for dependency subtable analysis.
fn dependency_subtable(table: &str) -> Option<&str> {
    // Test ordinary dependency-table prefixes before target-specific tables.
    for prefix in ["dependencies.", "dev-dependencies.", "build-dependencies."] {
        if let Some(dependency) = table.strip_prefix(prefix) {
            return nonempty_dependency_name(dependency);
        }
    }

    // Ignore non-target tables after ordinary dependency prefixes fail.
    if !table.starts_with("target.") {
        return None;
    }

    // Extract dependency names after supported target dependency markers.
    for marker in [
        ".dependencies.",
        ".dev-dependencies.",
        ".build-dependencies.",
    ] {
        if let Some((_, dependency)) = table.split_once(marker) {
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

/// Helper for ancestor manifest has workspace analysis.
fn ancestor_manifest_has_workspace(manifest_path: &Path) -> bool {
    let mut directory = manifest_path.parent().and_then(Path::parent);

    while let Some(path) = directory {
        let candidate = path.join("Cargo.toml");

        // Ancestor manifests provide workspace context only when they explicitly define workspace
        // tables; unreadable ancestors are ignored.
        if candidate.is_file()
            && read_manifest(&candidate)
                .is_some_and(|manifest| manifest_has_workspace_table(&manifest))
        {
            return true;
        }

        directory = path.parent();
    }

    false
}

/// Helper for manifest has workspace table analysis.
fn manifest_has_workspace_table(manifest: &str) -> bool {
    manifest
        .lines()
        .filter_map(table_header)
        .any(is_workspace_table)
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

/// Helper for array table header analysis.
fn array_table_header(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.starts_with('#') || !line.starts_with("[[") {
        return None;
    }

    // Array tables such as `[[example]]` are Cargo target declarations rather
    // than dependencies, but they can explain version-pinned dev fixtures.
    let close = line.find("]]")?;
    let table = line[2..close].trim();
    (!table.is_empty()).then_some(table)
}

/// Helper for starts table analysis.
fn starts_table(line: &str) -> bool {
    let line = line.trim_start();
    !line.starts_with('#') && line.starts_with('[')
}

/// Return whether workspace table.
fn is_workspace_table(table: &str) -> bool {
    table == "workspace" || table.starts_with("workspace.")
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

/// Helper for value is string analysis.
fn value_is_string(value: &str) -> bool {
    let value = value.trim_start();
    value.starts_with('"') || value.starts_with('\'')
}

/// Helper for inline table body analysis.
fn inline_table_body(value: &str) -> Option<&str> {
    value.trim().strip_prefix('{')?.trim_end().strip_suffix('}')
}

/// Helper for workspace inherited key analysis.
fn workspace_inherited_key(key: &str, value: &str) -> bool {
    key.ends_with(".workspace") && value_is_true(value)
}

/// Emit the dependency lint diagnostic.
fn emit_dependency_lint(cx: &EarlyContext<'_>, span: Span, dependency_name: &str) {
    let dependency_name = dependency_name.to_owned();

    // The only available span is in Rust source, so include the dependency name in the message.
    cx.emit_span_lint(
        WORKSPACE_DEPENDENCY_VERSIONS,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(format!(
                "dependency `{dependency_name}` specifies an explicit version instead of inheriting from `[workspace.dependencies]`"
            ));
            let _ = diag.help(format!(
                "move the version for `{dependency_name}` into `[workspace.dependencies]` and use `{dependency_name}.workspace = true` here"
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

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
