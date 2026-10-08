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
    Span,
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
        // Without candidate types there is no reason to inspect manifests or workspace files.
        let candidates = public_type_candidates(krate);
        if candidates.is_empty() {
            return;
        }
        let crate_root_span = krate.spans.inner_span;

        // Anchor manifest and source discovery to the crate root that rustc is compiling.
        // Virtual or path-remapped inputs may not have a readable local path, so skip them.
        let Some(crate_root) = cx
            .sess()
            .source_map()
            .span_to_filename(crate_root_span)
            .into_local_path()
        else {
            return;
        };
        let Some(manifest_path) = nearest_manifest(&crate_root) else {
            return;
        };
        if !private_package_manifest(&manifest_path) {
            return;
        }

        // Count identifiers across the crate and every Rust file in its workspace, because a
        // sibling crate or another target of this package can use a `pub` type.
        let search_root = workspace_root(&manifest_path);
        let identifier_counts = identifier_counts(cx, &crate_root, &search_root, &candidates);

        // Report public type names whose declaration is their only local occurrence.
        for candidate in candidates {
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

/// Count identifiers in the crate's loaded files and in every Rust file under `search_root`.
fn identifier_counts(
    cx: &EarlyContext<'_>,
    crate_root: &Path,
    search_root: &Path,
    candidates: &[PublicTypeCandidate],
) -> BTreeMap<String, usize> {
    // Deduplicate files that the crate loads from inside the search root.
    let mut files = local_crate_rust_files(cx, crate_root);
    files.extend(workspace_rust_files(search_root));

    // Aggregate lexical identifier counts, skipping files that cannot be read as UTF-8 text.
    let mut counts = candidates
        .iter()
        .map(|candidate| (candidate.name.clone(), 0))
        .collect();
    for source in files.iter().filter_map(|path| read_file(path)) {
        count_identifiers(&source, &mut counts);
        // Further files cannot change the result once every candidate has another occurrence.
        if counts.values().all(|count| *count > 1) {
            break;
        }
    }
    counts
}

/// Return the crate's loaded Rust files under its crate-root directory.
fn local_crate_rust_files(cx: &EarlyContext<'_>, crate_root: &Path) -> BTreeSet<PathBuf> {
    // Resolve the crate directory and deduplicate source-map file paths.
    let crate_dir = crate_root.parent().unwrap_or_else(|| Path::new("."));
    let source_map = cx.sess().source_map();
    let files = source_map.files();
    let candidates = files
        .iter()
        .filter_map(|source_file| {
            // Virtual, remapped, and imported paths may not be readable on the local host.
            source_file.name.clone().into_local_path()
        })
        .filter(|path| path.starts_with(crate_dir) && is_rust_file(path))
        .collect();

    // Release the source-map guard before callers perform file I/O.
    drop(files);
    candidates
}

/// Return Rust files under a workspace directory, skipping hidden entries
/// and build output.
#[expect(
    clippy::disallowed_methods,
    reason = "early lint execution is synchronous and has no async runtime"
)]
fn workspace_rust_files(search_root: &Path) -> BTreeSet<PathBuf> {
    // Keep discovered files and pending directories in deterministic traversal state.
    let mut files = BTreeSet::new();
    let mut directories = vec![search_root.to_path_buf()];

    // Walk iteratively without following symbolic links, so link cycles cannot recurse.
    while let Some(directory) = directories.pop() {
        // Ignore directories that disappear or cannot be read during the walk.
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // Skip hidden entries and Cargo build output before inspecting their type.
            let is_skipped = is_hidden_or_target(&entry);
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            // Queue directories and record only Rust source files.
            if is_skipped {
                continue;
            }
            if file_type.is_dir() {
                directories.push(path);
            } else if file_type.is_file() && is_rust_file(&path) {
                let _ = files.insert(path);
            }
        }
    }
    files
}

/// Return whether an entry is hidden or is Cargo's build-output directory.
fn is_hidden_or_target(entry: &std::fs::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_none_or(|name| name.starts_with('.') || name == "target")
}

/// Return the nearest directory, from the package upward, whose manifest declares `[workspace]`.
fn workspace_root(manifest_path: &Path) -> PathBuf {
    let package_dir = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Cargo uses the nearest enclosing workspace; a package without one is its own root.
    package_dir
        .ancestors()
        .find(|directory| {
            read_file(&directory.join("Cargo.toml")).is_some_and(|manifest| {
                manifest
                    .lines()
                    .any(|line| table_header(line) == Some("workspace"))
            })
        })
        .map_or_else(|| package_dir.clone(), Path::to_path_buf)
}

/// Count only candidate identifiers, retaining at most the two occurrences needed.
fn count_identifiers(source: &str, counts: &mut BTreeMap<String, usize>) {
    // Borrow token slices instead of allocating and cloning every workspace identifier.
    for identifier in source.split(|character| !is_identifier_continue(character)) {
        record_identifier(identifier, counts);
    }
}

/// Record a valid candidate token without growing the set of tracked names.
fn record_identifier(identifier: &str, counts: &mut BTreeMap<String, usize>) {
    if identifier.chars().next().is_some_and(is_identifier_start)
        && let Some(count) = counts.get_mut(identifier)
    {
        *count = (*count + 1).min(2);
    }
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
        if is_workspace_publish_false(&candidate) {
            return true;
        }

        directory = path.parent();
    }

    false
}

/// Return whether one manifest is a workspace publication boundary with
/// `publish = false`.
fn is_workspace_publish_false(candidate: &Path) -> bool {
    candidate.is_file()
        && read_file(candidate)
            .is_some_and(|manifest| manifest_publish_info(&manifest).workspace.publish_is_false)
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

/// Return whether rust file.
fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
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

#[cfg(test)]
mod tests {
    #![expect(
        clippy::disallowed_methods,
        reason = "synchronous filesystem fixtures exercise this early lint's file parser"
    )]

    use super::{
        count_identifiers, is_identifier_continue, is_identifier_start, is_rust_file, key_value,
        manifest_publish_info, nearest_manifest, private_package_manifest, read_file,
        record_identifier, table_header, value_is_false, value_is_true, workspace_root,
        workspace_rust_files,
    };
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
    };

    /// Create an isolated temporary directory for manifest parser tests.
    fn temporary_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "unnecessary-public-type-{name}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    /// Identifier scanning counts ASCII tokens and flushes at end of input.
    #[test]
    fn counts_identifiers() {
        let mut counts = BTreeMap::from([
            ("alpha".to_owned(), 0),
            ("_beta".to_owned(), 0),
            ("gamma".to_owned(), 0),
        ]);
        count_identifiers("alpha _beta 123gamma alpha unrelated alpha", &mut counts);
        assert_eq!(
            counts,
            BTreeMap::from([
                ("alpha".to_owned(), 2),
                ("_beta".to_owned(), 1),
                ("gamma".to_owned(), 0),
            ])
        );
    }

    /// Identifier recording ignores empty and invalid-start tokens.
    #[test]
    fn records_only_valid_identifier_starts() {
        let mut counts = BTreeMap::new();
        record_identifier("", &mut counts);
        record_identifier("123", &mut counts);

        assert!(counts.is_empty());
    }

    /// Identifier character predicates accept the documented ASCII forms.
    #[test]
    fn classifies_identifier_characters() {
        assert!(is_identifier_start('_') && is_identifier_continue('9'));
    }

    /// Manifest parsing tracks package and workspace publication policy.
    #[test]
    fn parses_publication_policy() {
        let info = manifest_publish_info(
            "[package]\npublish.workspace = true\n[workspace.package]\npublish = false\n",
        );

        assert!(info.package.seen && info.package.publish_inherits_workspace);
    }

    /// Direct false publication values are recognized in both supported forms.
    #[test]
    fn recognizes_false_publication_values() {
        assert!(value_is_false(" [] ") && value_is_false("false"));
    }

    /// The inherited publication flag accepts only the literal true value.
    #[test]
    fn recognizes_true_publication_values() {
        assert!(value_is_true(" true ") && !value_is_true("1"));
    }

    /// Manifest table and assignment parsing ignores comments and malformed lines.
    #[test]
    fn parses_manifest_lines() {
        assert!(table_header("  [package] # comment").is_some());
        assert!(key_value("publish = false # comment").is_some());
    }

    /// A missing package manifest is not treated as private.
    #[test]
    fn rejects_missing_private_manifest() {
        assert!(!private_package_manifest(Path::new("/missing/Cargo.toml")));
    }

    /// A manifest without a package table is not treated as private.
    #[test]
    fn rejects_manifest_without_package_table() {
        // Write a workspace-only manifest to exercise the missing package branch.
        let directory = temporary_directory("no-package");
        let manifest = directory.join("Cargo.toml");
        std::fs::write(&manifest, "[workspace]\n").unwrap();

        // Remove the fixture after parsing so parallel test runs do not retain files.
        let is_private = private_package_manifest(&manifest);
        std::fs::remove_dir_all(directory).unwrap();

        assert!(!is_private);
    }

    /// A package manifest with publish false is treated as private.
    #[test]
    fn accepts_private_manifest() {
        // Create a direct package publication policy for the positive branch.
        let directory = temporary_directory("package");
        let manifest = directory.join("Cargo.toml");
        std::fs::write(&manifest, "[package]\npublish = false\n").unwrap();

        // Remove the fixture after parsing to keep the temporary tree isolated.
        let is_private = private_package_manifest(&manifest);
        std::fs::remove_dir_all(directory).unwrap();

        assert!(is_private);
    }

    /// A workspace package table with publish false is treated as private.
    #[test]
    fn accepts_workspace_package_policy() {
        // Set publication policy in the workspace package table.
        let directory = temporary_directory("workspace-package");
        let manifest = directory.join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[package]\n[workspace.package]\npublish = false\n",
        )
        .unwrap();

        // Remove the fixture after parsing to keep the temporary tree isolated.
        let is_private = private_package_manifest(&manifest);
        std::fs::remove_dir_all(directory).unwrap();

        assert!(is_private);
    }

    /// An inherited workspace publish policy is resolved from the ancestor manifest.
    #[test]
    fn accepts_inherited_workspace_policy() {
        // Place the package below a workspace that disables publication.
        let root = temporary_directory("inherited");
        let package = root.join("package");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\n[workspace.package]\npublish = false\n",
        )
        .unwrap();
        let manifest = package.join("Cargo.toml");
        std::fs::write(&manifest, "[package]\npublish.workspace = true\n").unwrap();

        // Remove the fixture after parsing to keep the temporary tree isolated.
        let is_private = private_package_manifest(&manifest);
        std::fs::remove_dir_all(root).unwrap();

        assert!(is_private);
    }

    /// Inherited publication falls back to public when no ancestor disables publishing.
    #[test]
    fn rejects_unrestricted_inherited_policy() {
        // Enable workspace inheritance without providing a private ancestor.
        let directory = temporary_directory("inherited-public");
        let manifest = directory.join("Cargo.toml");
        std::fs::write(&manifest, "[package]\npublish.workspace = true\n").unwrap();

        // Remove the fixture after parsing to keep the temporary tree isolated.
        let is_private = private_package_manifest(&manifest);
        std::fs::remove_dir_all(directory).unwrap();

        assert!(!is_private);
    }

    /// Workspace discovery falls back to the package directory without a workspace table.
    #[test]
    fn uses_package_directory_without_workspace() {
        assert_eq!(
            workspace_root(Path::new("/missing/package/Cargo.toml")),
            Path::new("/missing/package")
        );
    }

    /// Workspace discovery returns the nearest manifest containing a workspace table.
    #[test]
    fn finds_workspace_directory() {
        // Create a package nested beneath a manifest with a workspace table.
        let root = temporary_directory("workspace-root");
        let package = root.join("package");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        let manifest = package.join("Cargo.toml");
        std::fs::write(&manifest, "[package]\n").unwrap();

        // Remove the fixture only after workspace discovery reads both manifests.
        let workspace_root_path = workspace_root(&manifest);
        std::fs::remove_dir_all(root.clone()).unwrap();

        assert_eq!(workspace_root_path, root);
    }

    /// File helpers distinguish Rust sources from other paths and unreadable files.
    #[test]
    fn classifies_source_files() {
        assert!(is_rust_file(Path::new("src/lib.rs")));
        assert!(!is_rust_file(Path::new("README.md")));
        assert!(read_file(Path::new("/missing/source.rs")).is_none());
    }

    /// Missing directories are skipped during workspace traversal.
    #[test]
    fn skips_missing_workspace_directory() {
        assert!(workspace_rust_files(Path::new("/missing/source-tree")).is_empty());
    }

    /// The nearest manifest search stops when no ancestor exists.
    #[test]
    fn finds_no_manifest_for_missing_tree() {
        assert!(nearest_manifest(Path::new("/missing/src/lib.rs")).is_none());
    }

    /// Workspace traversal returns Rust files under the selected source tree.
    #[test]
    fn discovers_workspace_rust_files() {
        assert!(!workspace_rust_files(Path::new(env!("CARGO_MANIFEST_DIR"))).is_empty());
    }

    /// Hidden and Cargo target directories are excluded before file classification.
    #[test]
    fn skips_hidden_and_target_directories() {
        // Populate ignored directories and one source file under the traversal root.
        let root = temporary_directory("skipped");
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join(".hidden/ignored.rs"), "").unwrap();
        std::fs::write(root.join("target/ignored.rs"), "").unwrap();
        std::fs::write(root.join("kept.rs"), "").unwrap();

        // Remove the fixture after traversal so only the visible file can be returned.
        let files = workspace_rust_files(&root);
        std::fs::remove_dir_all(root.clone()).unwrap();

        assert_eq!(files, std::iter::once(root.join("kept.rs")).collect());
    }
}
