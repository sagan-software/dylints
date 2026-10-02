#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc items while parsing validated ASCII Cargo syntax"
)]

//! A lint to check for public serde DTOs missing a schema derive.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use schemars as _;
#[cfg(test)]
use serde as _;

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use rustc_errors::DiagDecorator;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, def_id::LocalDefId};

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub PUBLIC_SERDE_SCHEMA_DERIVE,
    Warn,
    "public serde DTO in a schema-generating crate should derive JsonSchema",
    PublicSerdeSchemaDerive,
    PublicSerdeSchemaDerive::default()
}

/// Stateful pass that caches serde/schema impl targets for the current crate.
#[derive(Default)]
struct PublicSerdeSchemaDerive {
    /// Local ADTs grouped by the traits already implemented for them.
    impls: ImplTargets,
    /// Cached result of the nearest-manifest `schemars` dependency check.
    manifest_has_schemars: Option<bool>,
}

/// Local type IDs that implement serde and schema traits.
#[derive(Default)]
struct ImplTargets {
    /// Local types with a serde serialize or deserialize implementation.
    serde: Vec<LocalDefId>,
    /// Local types with a schemars `JsonSchema` implementation.
    schema: Vec<LocalDefId>,
}

impl<'tcx> LateLintPass<'tcx> for PublicSerdeSchemaDerive {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.impls = local_impl_targets(cx);
        self.manifest_has_schemars = None;
    }

    /// Check item for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Evaluate public serde DTOs only in crates that use schema generation.
        if !self.schema_crate(cx, item.span) {
            return;
        }

        let Some(candidate) = serde_dto_candidate(cx, item, &self.impls) else {
            return;
        };

        emit_schema_derive_lint(cx, &candidate);
    }
}

impl PublicSerdeSchemaDerive {
    /// Helper for schema crate analysis.
    fn schema_crate(&mut self, cx: &LateContext<'_>, span: Span) -> bool {
        if self.impls.has_schema_impl() {
            return true;
        }

        // Cargo metadata is outside rustc's lint context, so read the nearest manifest only once.
        *self
            .manifest_has_schemars
            .get_or_insert_with(|| manifest_has_schemars_dependency(cx, span))
    }
}

impl ImplTargets {
    /// Return whether schema impl is present.
    const fn has_schema_impl(&self) -> bool {
        !self.schema.is_empty()
    }

    /// Helper for implements serde without schema analysis.
    fn implements_serde_without_schema(&self, local_def_id: LocalDefId) -> bool {
        self.serde.contains(&local_def_id) && !self.schema.contains(&local_def_id)
    }

    /// Helper for push serde analysis.
    fn push_serde(&mut self, local_def_id: LocalDefId) {
        push_unique(&mut self.serde, local_def_id);
    }

    /// Helper for push schema analysis.
    fn push_schema(&mut self, local_def_id: LocalDefId) {
        push_unique(&mut self.schema, local_def_id);
    }
}

/// Helper for push unique analysis.
fn push_unique(targets: &mut Vec<LocalDefId>, local_def_id: LocalDefId) {
    if !targets.contains(&local_def_id) {
        targets.push(local_def_id);
    }
}

/// Public serde DTO that is missing a schema implementation.
struct SerdeDtoCandidate {
    /// Display name used in the diagnostic message.
    name: String,
    /// Human-readable ADT kind, such as `struct` or `enum`.
    kind: &'static str,
    /// Item span that should receive the diagnostic.
    span: Span,
}

/// Helper for local impl targets analysis.
fn local_impl_targets(cx: &LateContext<'_>) -> ImplTargets {
    // Classify local impls by the resolved trait family they implement.
    let mut targets = ImplTargets::default();

    for (&trait_def_id, impl_def_ids) in cx.tcx.all_local_trait_impls(()) {
        let Some(kind) = trait_kind(cx, trait_def_id) else {
            continue;
        };

        // Retain only impls whose self type is a local ADT.
        for &impl_def_id in impl_def_ids {
            let Some(local_adt) = impl_self_local_adt(cx, impl_def_id) else {
                continue;
            };

            // Record each local type once in the matching trait family.
            match kind {
                TraitKind::Serde => targets.push_serde(local_adt),
                TraitKind::Schema => targets.push_schema(local_adt),
            }
        }
    }

    targets
}

/// Trait family implemented by a local ADT.
enum TraitKind {
    /// Serde serialization or deserialization.
    Serde,
    /// Schemars schema generation.
    Schema,
}

/// Helper for trait kind analysis.
fn trait_kind(cx: &LateContext<'_>, trait_def_id: rustc_span::def_id::DefId) -> Option<TraitKind> {
    // Match Serde derives across public, core, and derive-generated paths.
    let crate_name = cx.tcx.crate_name(trait_def_id.krate).to_ident_string();
    let item_name = cx.tcx.item_name(trait_def_id).to_ident_string();
    let path = cx.tcx.def_path_str(trait_def_id);

    // Match the schema trait separately because it has a different defining crate.
    let is_serde =
        matches!(item_name.as_str(), "Serialize" | "Deserialize") && is_serde_trait_path(&path);
    let is_schema = crate_name == "schemars" && item_name == "JsonSchema";

    is_serde
        .then_some(TraitKind::Serde)
        .or_else(|| is_schema.then_some(TraitKind::Schema))
}

/// Return whether a resolved path names one supported Serde trait.
fn is_serde_trait_path(path: &str) -> bool {
    [
        "serde::Serialize",
        "serde::Deserialize",
        "serde::ser::Serialize",
        "serde::de::Deserialize",
        "serde_core::ser::Serialize",
        "serde_core::de::Deserialize",
    ]
    .contains(&path)
        || [
            "::_serde::Serialize",
            "::_serde::Deserialize",
            "::serde::Serialize",
            "::serde::Deserialize",
            "::serde::ser::Serialize",
            "::serde::de::Deserialize",
            "::serde_core::ser::Serialize",
            "::serde_core::de::Deserialize",
        ]
        .iter()
        .any(|suffix| path.ends_with(suffix))
}

/// Helper for impl self local adt analysis.
fn impl_self_local_adt(cx: &LateContext<'_>, impl_def_id: LocalDefId) -> Option<LocalDefId> {
    // Only inherent or trait implementation definitions can have a meaningful self type.
    if !matches!(
        cx.tcx.def_kind(impl_def_id),
        rustc_hir::def::DefKind::Impl { .. }
    ) {
        return None;
    }
    // Derived impls target the user type directly; skip blanket, primitive, and external impls.
    let self_ty = cx
        .tcx
        .type_of(impl_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    let ty::Adt(adt, _) = self_ty.kind() else {
        return None;
    };

    adt.did().as_local()
}

/// Helper for serde dto candidate analysis.
fn serde_dto_candidate(
    cx: &LateContext<'_>,
    item: &Item<'_>,
    impls: &ImplTargets,
) -> Option<SerdeDtoCandidate> {
    // Require a public item with a Serde impl and no schema impl.
    let is_private = !cx.tcx.visibility(item.owner_id.def_id).is_public();
    let has_serde_without_schema = impls.implements_serde_without_schema(item.owner_id.def_id);
    if is_private || !has_serde_without_schema {
        return None;
    }

    // Limit diagnostics to named structs and enums at their identifier spans.
    let (name, kind, span) = match item.kind {
        ItemKind::Struct(ident, ..) => (ident.name.to_ident_string(), "struct", ident.span),
        ItemKind::Enum(ident, ..) => (ident.name.to_ident_string(), "enum", ident.span),
        _ => return None,
    };

    Some(SerdeDtoCandidate { name, kind, span })
}

/// Helper for manifest has schemars dependency analysis.
fn manifest_has_schemars_dependency(cx: &LateContext<'_>, span: Span) -> bool {
    // Resolve the nearest readable Cargo manifest from the item's source file.
    let Some(crate_root) = crate_root_path(cx, span) else {
        return false;
    };

    let Some(manifest_path) = nearest_manifest(&crate_root) else {
        return false;
    };

    read_file(&manifest_path).is_some_and(|manifest| manifest_declares_schemars(&manifest))
}

/// Helper for manifest declares schemars analysis.
fn manifest_declares_schemars(manifest: &str) -> bool {
    // Track whether bare dependency keys belong to an active dependency table.
    let mut active_dependency_table = false;

    for line in manifest.lines() {
        // A new table either proves a schemars subtable or changes key scope.
        if starts_table(line) {
            let Some(header) = table_header(line) else {
                active_dependency_table = false;
                continue;
            };

            if dependency_subtable_is_schemars(header) {
                return true;
            }

            active_dependency_table = dependency_table(header);
            continue;
        }

        // Ignore key-value entries outside dependency tables.
        if !active_dependency_table {
            continue;
        }

        let Some((key, _value)) = key_value(line) else {
            continue;
        };

        // Accept bare and dotted schemars dependency keys.
        if dependency_key_is_schemars(key) {
            return true;
        }
    }

    false
}

/// Helper for dependency table analysis.
fn dependency_table(header: &str) -> bool {
    matches!(
        header,
        "dependencies" | "dev-dependencies" | "build-dependencies" | "workspace.dependencies"
    ) || header.starts_with("target.")
        && (header.ends_with(".dependencies")
            || header.ends_with(".dev-dependencies")
            || header.ends_with(".build-dependencies"))
}

/// Helper for dependency subtable is schemars analysis.
fn dependency_subtable_is_schemars(header: &str) -> bool {
    [
        "dependencies.",
        "dev-dependencies.",
        "build-dependencies.",
        "workspace.dependencies.",
    ]
    .iter()
    .any(|prefix| header.strip_prefix(prefix).is_some_and(unquoted_schemars))
        || header.starts_with("target.")
            && [
                ".dependencies.",
                ".dev-dependencies.",
                ".build-dependencies.",
            ]
            .iter()
            .any(|marker| {
                header
                    .split_once(marker)
                    .is_some_and(|(_, dependency)| unquoted_schemars(dependency))
            })
}

/// Helper for dependency key is schemars analysis.
fn dependency_key_is_schemars(key: &str) -> bool {
    let dependency = key
        .split_once('.')
        .map_or(key, |(dependency, _)| dependency);
    unquoted_schemars(dependency)
}

/// Helper for unquoted schemars analysis.
fn unquoted_schemars(name: &str) -> bool {
    name.trim().trim_matches('"').trim_matches('\'') == "schemars"
}

/// Read file for source-based analysis.
fn read_file(path: &Path) -> Option<String> {
    let mut contents = String::new();

    // Lint passes run inside rustc, so keep manifest reads dependency-free.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut contents).ok()?;

    Some(contents)
}

/// Helper for starts table analysis.
fn starts_table(line: &str) -> bool {
    let line = line.trim_start();
    !line.starts_with('#') && line.starts_with('[')
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
    // Ignore empty lines and full-line comments before splitting the assignment.
    let line = line.trim_start();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let (key, value) = line.split_once('=')?;
    Some((
        key.trim(),
        value
            .split_once('#')
            .map_or(value, |(before, _)| before)
            .trim(),
    ))
}

/// Helper for crate root path analysis.
fn crate_root_path(cx: &LateContext<'_>, span: Span) -> Option<PathBuf> {
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

/// Emit the schema derive lint diagnostic.
fn emit_schema_derive_lint(cx: &LateContext<'_>, candidate: &SerdeDtoCandidate) {
    // Build a diagnostic that names the public ADT kind and identifier.
    let message = format!(
        "public serde {} `{}` is missing a schema derive",
        candidate.kind, candidate.name
    );

    // Point at the item name and prescribe the matching schema derive.
    cx.emit_span_lint(
        PUBLIC_SERDE_SCHEMA_DERIVE,
        candidate.span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag
                .help("derive `schemars::JsonSchema` so Rust, JSON Schema, and docs stay aligned");
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
    /// Helper for detects schemars manifest dependencies analysis.
    #[test]
    fn detects_schemars_manifest_dependencies() {
        let manifest = r#"
            [dependencies]
            serde = "1.0.228"

            [workspace.dependencies]
            schemars = "1.2.1"
        "#;

        assert!(super::manifest_declares_schemars(manifest));
    }
}
