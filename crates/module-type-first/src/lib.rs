#![feature(rustc_private)]

//! A lint to check for module representative types that are not first.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::{Crate, Item, ItemKind, ModKind};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub MODULE_TYPE_FIRST,
    Warn,
    "module representative type should be the first source-order item after imports",
    ModuleTypeFirst
}

impl EarlyLintPass for ModuleTypeFirst {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // The crate root has no reliable snake_case module name, so start at named modules.
        for item in &krate.items {
            check_loaded_module(cx, item);
        }
    }
}

/// State used by the representative type analysis.
struct RepresentativeType {
    /// name stored for this lint's analysis.
    name: String,
    /// kind stored for this lint's analysis.
    kind: &'static str,
    /// span stored for this lint's analysis.
    span: Span,
}

/// Check loaded module for this lint.
fn check_loaded_module(cx: &EarlyContext<'_>, item: &Item) {
    // Restrict recursion to modules whose items were loaded into this crate.
    let ItemKind::Mod(_, ident, ModKind::Loaded(items, _, spans)) = &item.kind else {
        return;
    };

    // Ignore generated modules because their source order is not user-controlled.
    if ident.span.from_expansion() || spans.inner_span.from_expansion() {
        return;
    }

    let module_name = ident.name.to_ident_string();
    check_module_items(cx, &module_name, items);
}

/// Check module items for this lint.
fn check_module_items(cx: &EarlyContext<'_>, module_name: &str, items: &[Box<Item>]) {
    // Check the current module before recursing so diagnostics follow source nesting order.
    if let Some(representative_type) = misplaced_representative_type(module_name, items) {
        emit_module_type_first_lint(cx, module_name, &representative_type);
    }

    for item in items {
        check_loaded_module(cx, item);
    }
}

/// Return type information for misplaced representative.
fn misplaced_representative_type(
    module_name: &str,
    items: &[Box<Item>],
) -> Option<RepresentativeType> {
    let expected_name = pascal_case_module_name(module_name)?;
    let first_definition = items
        .iter()
        .map(Box::as_ref)
        .find(|item| is_source_order_item(item))?;

    // If the representative type already heads the module body, the local order is correct.
    if is_type_named(first_definition, &expected_name) {
        return None;
    }

    items
        .iter()
        .map(Box::as_ref)
        .find_map(|item| matching_type_definition(item, &expected_name))
}

/// Return whether import item.
const fn is_import_item(item: &Item) -> bool {
    matches!(item.kind, ItemKind::Use(_) | ItemKind::ExternCrate(..))
}

/// Return whether source order item.
fn is_source_order_item(item: &Item) -> bool {
    !is_import_item(item)
        && !item.span.from_expansion()
        && !matches!(item.kind, ItemKind::MacCall(_))
}

/// Return whether type named.
fn is_type_named(item: &Item, expected_name: &str) -> bool {
    matching_type_definition(item, expected_name).is_some()
}

/// Helper for matching type definition analysis.
fn matching_type_definition(item: &Item, expected_name: &str) -> Option<RepresentativeType> {
    let type_definition = type_definition(item)?;
    (type_definition.name == expected_name).then_some(type_definition)
}

/// Helper for type definition analysis.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only type-defining item variants can provide a representative type"
)]
fn type_definition(item: &Item) -> Option<RepresentativeType> {
    let (name, kind, span) = match &item.kind {
        ItemKind::Struct(ident, _, _) => (ident.name.to_ident_string(), "struct", ident.span),
        ItemKind::Enum(ident, _, _) => (ident.name.to_ident_string(), "enum", ident.span),
        ItemKind::Union(ident, _, _) => (ident.name.to_ident_string(), "union", ident.span),
        ItemKind::TyAlias(alias) => (
            alias.ident.name.to_ident_string(),
            "type alias",
            alias.ident.span,
        ),
        _ => return None,
    };

    if item.span.from_expansion() || span.from_expansion() {
        return None;
    }

    Some(RepresentativeType { name, kind, span })
}

/// Return the pascal case module name.
fn pascal_case_module_name(module_name: &str) -> Option<String> {
    // Convert only canonical ASCII snake_case module names.
    if !is_ascii_snake_case(module_name) {
        return None;
    }

    let mut pascal_case = String::new();
    for segment in module_name.split('_').filter(|segment| !segment.is_empty()) {
        let mut chars = segment.chars();
        let first = chars.next()?;

        // Preserve the rest of each segment because snake_case modules have already been filtered.
        pascal_case.extend(first.to_uppercase());
        pascal_case.push_str(chars.as_str());
    }

    (!pascal_case.is_empty()).then_some(pascal_case)
}

/// Return whether ascii snake case.
fn is_ascii_snake_case(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_lowercase() || ch.is_ascii_digit())
        && name.chars().any(|ch| ch.is_ascii_lowercase())
}

/// Emit the module type first lint diagnostic.
fn emit_module_type_first_lint(
    cx: &EarlyContext<'_>,
    module_name: &str,
    representative_type: &RepresentativeType,
) {
    // Compose the diagnostic from the resolved representative type and module names.
    // Keep the help focused on the source-order boundary after leading imports.
    emit_span_lint_with_help(
        cx,
        MODULE_TYPE_FIRST,
        representative_type.span,
        format!(
            "{} `{}` should be the first source-order item after imports in module `{module_name}`",
            representative_type.kind, representative_type.name
        ),
        "place the representative type below leading imports and above functions, constants, impls, nested modules, and other source items",
    );
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
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
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
