#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc items"
)]

//! A lint to check for public serde DTOs missing a schema derive.
//!
//! It resolves the serde and schemars traits that local types implement and
//! reports exported serde types without a schema in crates that depend on
//! schemars, either through a `JsonSchema` impl or a `--extern schemars` input.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use schemars as _;
#[cfg(test)]
use serde as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, Symbol, def_id::LocalDefId};

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
    /// Whether the crate depends on `schemars` or implements `JsonSchema`.
    has_schema_crate: bool,
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
    /// Collect trait impl targets and decide whether the crate uses schema generation.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.impls = local_impl_targets(cx);

        // Cargo passes each direct dependency of this target as `--extern`, including
        // dev-dependencies for test targets; a loaded crate also counts.
        // Cache schema usage before item analysis so each item uses one stable decision.
        self.has_schema_crate = !self.impls.schema.is_empty();
        if !self.has_schema_crate {
            self.has_schema_crate = has_schemars_dependency(cx);
        }
    }

    /// Check one item for a missing schema implementation.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Skip crates without schema support before resolving item-level trait evidence.
        if !self.has_schema_crate {
            return;
        }
        let Some(candidate) = serde_dto_candidate(cx, item, &self.impls) else {
            return;
        };
        emit_schema_derive_lint(cx, &candidate);
    }
}

/// Return whether the current crate links or loads `schemars`.
fn has_schemars_dependency(cx: &LateContext<'_>) -> bool {
    let schemars = Symbol::intern("schemars");
    cx.sess().opts.externs.get("schemars").is_some()
        || cx
            .tcx
            .crates(())
            .iter()
            .any(|krate| cx.tcx.crate_name(*krate) == schemars)
}

impl ImplTargets {
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

/// Classify a trait by its defining crate and definition path.
fn trait_kind(cx: &LateContext<'_>, trait_def_id: rustc_span::def_id::DefId) -> Option<TraitKind> {
    // The definition path names the defining crate, so re-exports such as `serde::Serialize`
    // resolve to `serde_core::ser::Serialize` or `serde::ser::Serialize`.
    let path = cx.get_def_path(trait_def_id);
    let segments = path.iter().map(Symbol::as_str).collect::<Vec<_>>();
    match segments.as_slice() {
        ["serde" | "serde_core", "ser", "Serialize"]
        | ["serde" | "serde_core", "de", "Deserialize"] => Some(TraitKind::Serde),
        ["schemars", .., "JsonSchema"] => Some(TraitKind::Schema),
        _ => None,
    }
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
    // Require an exported item with a Serde impl and no schema impl; `pub` inside a private
    // module is not reachable by other crates.
    let is_private = !cx.effective_visibilities.is_exported(item.owner_id.def_id);
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
    use super::ImplTargets;
    use rustc_span::def_id::{DefIndex, LocalDefId};

    /// Build a stable local definition identifier for collection tests.
    fn local_def_id(index: u32) -> LocalDefId {
        LocalDefId {
            local_def_index: DefIndex::from_u32(index),
        }
    }

    /// Trait target collection remains unique and separates schema implementations.
    #[test]
    fn tracks_unique_trait_targets() {
        // Use separate local IDs to distinguish serde-only and schema-backed targets.
        let first = local_def_id(1);
        let second = local_def_id(2);
        let mut targets = ImplTargets::default();

        // Insert each target twice so duplicate suppression and separation run together.
        targets.push_serde(first);
        targets.push_serde(first);
        targets.push_schema(second);
        targets.push_schema(second);

        assert!(
            targets.serde == vec![first]
                && targets.schema == vec![second]
                && targets.implements_serde_without_schema(first)
                && !targets.implements_serde_without_schema(second)
        );
    }
}
