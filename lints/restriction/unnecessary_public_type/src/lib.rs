#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc items while parsing validated ASCII Cargo syntax"
)]

//! A lint to check for unused public types in private crates.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use rustc_ast::{AttrVec, Attribute, Crate, Item, ItemKind, visit::Visitor, visit::walk_item};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{
    SourceFile, Span,
    symbol::{Symbol, sym},
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub UNNECESSARY_PUBLIC_TYPE,
    Warn,
    "unused public type in a private crate",
    UnnecessaryPublicType
}

impl EarlyLintPass for UnnecessaryPublicType {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let crate_root_span = krate.spans.inner_span;

        // Anchor manifest and source discovery to the crate root that rustc is compiling.
        let Some(crate_root) = crate_root_path(cx, crate_root_span) else {
            return;
        };
        let Some(manifest_path) = nearest_manifest(&crate_root) else {
            return;
        };
        if !private_package_manifest(&manifest_path) {
            return;
        }

        // Count identifiers across every readable Rust file in the local crate.
        let Some(identifier_counts) = identifier_counts(cx, &crate_root) else {
            return;
        };

        // Report public type names whose declaration is their only local occurrence.
        for candidate in public_type_candidates(krate) {
            if identifier_counts
                .get(&candidate.name)
                .copied()
                .unwrap_or_default()
                > 1
            {
                continue;
            }
            emit_unused_public_type_lint(cx, &candidate);
        }
    }
}

/// State used by the public type candidate analysis.
struct PublicTypeCandidate {
    /// name stored for this lint's analysis.
    name: String,
    /// kind stored for this lint's analysis.
    kind: &'static str,
    /// span stored for this lint's analysis.
    span: Span,
}

/// State used by the public type collector analysis.
struct PublicTypeCollector {
    /// candidates stored for this lint's analysis.
    candidates: Vec<PublicTypeCandidate>,
}

impl<'ast> Visitor<'ast> for PublicTypeCollector {
    /// Helper for visit item analysis.
    fn visit_item(&mut self, item: &'ast Item) {
        if let Some(candidate) = public_type_candidate(item) {
            self.candidates.push(candidate);
        }

        // Continue into modules so nested `pub` types in private crates are checked too.
        walk_item(self, item);
    }
}

/// Helper for public type candidates analysis.
fn public_type_candidates(krate: &Crate) -> Vec<PublicTypeCandidate> {
    // Walk the complete crate and retain candidates in source traversal order.
    let mut collector = PublicTypeCollector {
        candidates: Vec::new(),
    };

    for item in &krate.items {
        collector.visit_item(item);
    }

    collector.candidates
}

/// Helper for public type candidate analysis.
fn public_type_candidate(item: &Item) -> Option<PublicTypeCandidate> {
    // Exclude private items and exports with explicit external-surface markers.
    if !item.vis.kind.is_pub() || has_external_surface_marker(&item.attrs) {
        return None;
    }

    let (name, kind, span) = match &item.kind {
        ItemKind::Struct(ident, _, _) => (ident.name.to_ident_string(), "struct", ident.span),
        ItemKind::Enum(ident, _, _) => (ident.name.to_ident_string(), "enum", ident.span),
        ItemKind::TyAlias(alias) => (
            alias.ident.name.to_ident_string(),
            "type alias",
            alias.ident.span,
        ),
        _ => return None,
    };

    Some(PublicTypeCandidate { name, kind, span })
}

/// Return whether external surface marker is present.
fn has_external_surface_marker(attrs: &AttrVec) -> bool {
    attrs.iter().any(marker_attr)
}

/// Helper for marker attr analysis.
fn marker_attr(attr: &Attribute) -> bool {
    // Documentation and explicit lint controls are treated as signals that a human chose the
    // exported surface, while cfg/test/FFI markers are common non-local reachability hints.
    attr.is_doc_comment()
        || [
            sym::doc,
            sym::allow,
            sym::expect,
            sym::cfg,
            sym::cfg_attr,
            sym::test,
            sym::repr,
            sym::no_mangle,
            sym::export_name,
            sym::used,
            sym::link_name,
            Symbol::intern("wasm_bindgen"),
        ]
        .iter()
        .any(|name| attr.has_name(*name))
}

/// Helper for identifier counts analysis.
fn identifier_counts(cx: &EarlyContext<'_>, crate_root: &Path) -> Option<BTreeMap<String, usize>> {
    // Aggregate lexical identifier counts across all local source files.
    let mut counts = BTreeMap::new();

    for source_file in local_crate_rust_files(cx, crate_root) {
        let mut source = String::new();

        // Skip unreadable generated or remapped paths instead of guessing about their contents.
        // Count identifiers only after the complete file has been read.
        let mut file = File::open(source_file).ok()?;
        let _ = file.read_to_string(&mut source).ok()?;
        count_identifiers(&source, &mut counts);
    }

    Some(counts)
}

/// Helper for local crate rust files analysis.
fn local_crate_rust_files(cx: &EarlyContext<'_>, crate_root: &Path) -> Vec<PathBuf> {
    // Resolve the crate directory and deduplicate source-map file paths.
    let crate_dir = crate_root.parent().unwrap_or_else(|| Path::new("."));
    let source_map = cx.sess().source_map();
    let files = source_map.files();
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for source_file in files.iter() {
        // Retain readable-looking Rust paths inside this crate directory.
        let Some(path) = source_file_path(source_file) else {
            continue;
        };

        if !path.starts_with(crate_dir) || !is_rust_file(&path) || !seen.insert(path.clone()) {
            continue;
        }

        candidates.push(path);
    }

    // Release the source-map guard before callers perform file I/O.
    drop(files);

    candidates
}

/// Count identifiers used by the lint.
fn count_identifiers(source: &str, counts: &mut BTreeMap<String, usize>) {
    // Build ASCII identifiers incrementally and flush them at delimiters.
    let mut identifier = String::new();

    for character in source.chars() {
        if is_identifier_continue(character) {
            identifier.push(character);
            continue;
        }

        record_identifier(&mut identifier, counts);
    }

    // Flush an identifier that reaches the end of the source without a delimiter.
    record_identifier(&mut identifier, counts);
}

/// Helper for record identifier analysis.
fn record_identifier(identifier: &mut String, counts: &mut BTreeMap<String, usize>) {
    // Count only nonempty tokens with a valid Rust identifier start.
    if identifier.is_empty() {
        return;
    }

    if identifier.chars().next().is_some_and(is_identifier_start) {
        *counts.entry(identifier.clone()).or_default() += 1;
    }

    identifier.clear();
}

/// Return whether identifier start.
const fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

/// Return whether identifier continue.
const fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

/// Helper for private package manifest analysis.
fn private_package_manifest(manifest_path: &Path) -> bool {
    // Parse the package and workspace publication policy from this manifest.
    let Some(manifest) = read_file(manifest_path) else {
        return false;
    };

    let info = manifest_publish_info(&manifest);
    if !info.package.seen {
        return false;
    }

    // Accept direct, workspace-local, or inherited publish=false policy.
    info.package.publish_is_false
        || info.workspace.publish_is_false
        || info.package.publish_inherits_workspace
            && ancestor_workspace_publish_false(manifest_path)
}

/// State used by the manifest publish info analysis.
#[derive(Default)]
struct ManifestPublishInfo {
    /// package stored for this lint's analysis.
    package: PackagePublishInfo,
    /// workspace stored for this lint's analysis.
    workspace: WorkspacePublishInfo,
}

/// State used by the package publish info analysis.
#[derive(Default)]
struct PackagePublishInfo {
    /// seen stored for this lint's analysis.
    seen: bool,
    /// publish is false stored for this lint's analysis.
    publish_is_false: bool,
    /// publish inherits workspace stored for this lint's analysis.
    publish_inherits_workspace: bool,
}

/// State used by the workspace publish info analysis.
#[derive(Default)]
struct WorkspacePublishInfo {
    /// publish is false stored for this lint's analysis.
    publish_is_false: bool,
}

/// Helper for manifest publish info analysis.
fn manifest_publish_info(manifest: &str) -> ManifestPublishInfo {
    // Track the active table while reading package publication assignments.
    let mut info = ManifestPublishInfo::default();
    let mut table = "";

    // Process table transitions and assignments in source order.
    for line in manifest.lines() {
        if let Some(header) = table_header(line) {
            table = header;
            info.package.seen |= table == "package";
        } else if let Some((key, value)) = key_value(line) {
            update_publish_info(&mut info, table, key, value);
        }
    }

    info
}

/// Apply one supported publication assignment to the manifest summary.
fn update_publish_info(info: &mut ManifestPublishInfo, table: &str, key: &str, value: &str) {
    // Keep package and workspace publication policy separate for the caller.
    if table == "package" {
        update_package_publish_info(&mut info.package, key, value);
    } else if table == "workspace.package" {
        update_workspace_publish_info(&mut info.workspace, key, value);
    }
}

/// Applies one package publication assignment.
fn update_package_publish_info(info: &mut PackagePublishInfo, key: &str, value: &str) {
    if key == "publish" && value_is_false(value) {
        info.publish_is_false = true;
    }
    if key == "publish.workspace" && value_is_true(value) {
        info.publish_inherits_workspace = true;
    }
}

/// Applies one workspace publication assignment.
fn update_workspace_publish_info(info: &mut WorkspacePublishInfo, key: &str, value: &str) {
    if key == "publish" && value_is_false(value) {
        info.publish_is_false = true;
    }
}

/// Helper for ancestor workspace publish false analysis.
fn ancestor_workspace_publish_false(manifest_path: &Path) -> bool {
    // Walk ancestor manifests beyond the package directory.
    let mut directory = manifest_path.parent().and_then(Path::parent);

    while let Some(path) = directory {
        // Stop at the first ancestor workspace that sets publish=false.
        let candidate = path.join("Cargo.toml");
        if candidate.is_file()
            && read_file(&candidate)
                .is_some_and(|manifest| manifest_publish_info(&manifest).workspace.publish_is_false)
        {
            return true;
        }

        directory = path.parent();
    }

    false
}

/// Read file for source-based analysis.
fn read_file(path: &Path) -> Option<String> {
    let mut contents = String::new();

    // Keep manifest and source scanning dependency-free because this runs inside rustc.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut contents).ok()?;

    Some(contents)
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

/// Helper for key value analysis.
fn key_value(line: &str) -> Option<(&str, &str)> {
    // Remove inline comments before parsing a manifest assignment.
    let line = line.split('#').next()?.trim();
    if line.is_empty() {
        return None;
    }

    // Trim both sides and reject an empty key.
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    let value = value.trim();

    (!key.is_empty()).then_some((key, value))
}

/// Helper for value is false analysis.
fn value_is_false(value: &str) -> bool {
    matches!(value.trim(), "false" | "[]")
}

/// Helper for value is true analysis.
fn value_is_true(value: &str) -> bool {
    value.trim() == "true"
}

/// Helper for source file path analysis.
fn source_file_path(source_file: &SourceFile) -> Option<PathBuf> {
    // Virtual, remapped, and imported paths may not be readable on the local host.
    source_file.name.clone().into_local_path()
}

/// Return whether rust file.
fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
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

/// Emit the unused public type lint diagnostic.
fn emit_unused_public_type_lint(cx: &EarlyContext<'_>, candidate: &PublicTypeCandidate) {
    let message = format!(
        "public {} `{}` is unused inside this private crate",
        candidate.kind, candidate.name
    );

    // Keep the help generic because either deletion or reduced visibility may be correct.
    // Point at the public type name whose local count established the finding.
    cx.emit_span_lint(
        UNNECESSARY_PUBLIC_TYPE,
        candidate.span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ =
                diag.help("remove the type or make it private unless an external boundary uses it");
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
