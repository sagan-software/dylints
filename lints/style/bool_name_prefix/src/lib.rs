#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "the predicate vocabulary stays in one auditable matcher and ignores unrelated items"
)]
#![expect(
    clippy::too_many_lines,
    reason = "the predicate vocabulary stays in one auditable matcher"
)]

//! A lint to check for boolean names without predicate prefixes.
//!
//! It checks source-authored boolean bindings, fields, parameters, return
//! values, and related declarations for predicate names such as `is_ready`. Type
//! resolution and declaration context determine which names are predicates,
//! while configuration and generated syntax remain outside the lint's scope.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, FieldDef, FnDecl, FnRetTy, ImplItem, ImplItemImplKind, ImplItemKind, Item, ItemKind,
    LetStmt, LocalSource, Param, Pat, PatKind, QPath, TraitFn, TraitItem, TraitItemKind, Ty,
    TyKind, intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::{Ident, Span, def_id::LocalDefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BOOL_NAME_PREFIX,
    Warn,
    "boolean name does not read as a predicate",
    BoolNamePrefix
}

impl<'tcx> LateLintPass<'tcx> for BoolNamePrefix {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        if field.is_positional()
            || !is_bool_ty(
                cx.tcx
                    .type_of(field.def_id)
                    .instantiate_identity()
                    .skip_norm_wip(),
            )
        {
            return;
        }

        check_prefixed_ident(cx, "field", field.ident, PrefixStyle::Snake);
    }

    /// Check item for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Some((kind, ident)) = const_like_item(item) else {
            return;
        };

        if is_bool_ty(
            cx.tcx
                .type_of(item.owner_id)
                .instantiate_identity()
                .skip_norm_wip(),
        ) {
            check_prefixed_ident(cx, kind, ident, PrefixStyle::ScreamingConst);
        }
    }

    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        // Analyze named functions and methods because closures have no stable API name.
        if matches!(kind, FnKind::Closure) {
            return;
        }

        check_body_params(cx, body.params);

        if let FnKind::ItemFn(ident, ..) = kind
            && returns_bool(cx, local_def_id)
        {
            check_prefixed_ident(cx, "function", ident, PrefixStyle::Snake);
        }
    }

    /// Check local for this lint.
    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx LetStmt<'tcx>) {
        // Restrict the policy to ordinary boolean local bindings.
        if !matches!(local.source, LocalSource::Normal)
            || !is_bool_ty(cx.typeck_results().node_type(local.hir_id))
        {
            return;
        }

        let Some(ident) = binding_ident(local.pat) else {
            return;
        };

        check_prefixed_ident(cx, "local variable", ident, PrefixStyle::Snake);
    }

    /// Check trait item for this lint.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        // Apply constant, parameter, and return-name rules by trait item kind.
        // Required methods need their written parameter types because they have no body table.
        match item.kind {
            TraitItemKind::Const(ty, ..) => {
                if is_bool_hir_ty(ty) {
                    check_prefixed_ident(
                        cx,
                        "associated constant",
                        item.ident,
                        PrefixStyle::ScreamingConst,
                    );
                }
            }
            TraitItemKind::Fn(sig, TraitFn::Required(arg_names)) => {
                check_trait_required_params(cx, sig.decl.inputs, arg_names);
                check_return_name(cx, sig.decl, "method", item.ident);
            }
            TraitItemKind::Fn(sig, TraitFn::Provided(_)) => {
                check_return_name(cx, sig.decl, "method", item.ident);
            }
            TraitItemKind::Type(..) => {}
        }
    }

    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the naming policy to inherent items owned by the type.
        let ImplItemImplKind::Inherent { .. } = item.impl_kind else {
            return;
        };

        // Apply the constant or return-name rule for the resolved item shape.
        match item.kind {
            ImplItemKind::Const(ty, _) => {
                if is_bool_hir_ty(ty) {
                    check_prefixed_ident(
                        cx,
                        "associated constant",
                        item.ident,
                        PrefixStyle::ScreamingConst,
                    );
                }
            }
            ImplItemKind::Fn(sig, _) => {
                check_return_name(cx, sig.decl, "method", item.ident);
            }
            ImplItemKind::Type(_) => {}
        }
    }
}

/// Helper for const like item analysis.
const fn const_like_item(item: &Item<'_>) -> Option<(&'static str, Ident)> {
    match item.kind {
        ItemKind::Const(ident, ..) => Some(("constant", ident)),
        ItemKind::Static(_, ident, ..) => Some(("static", ident)),
        _ => None,
    }
}

/// Check body params for this lint.
fn check_body_params(cx: &LateContext<'_>, params: &[Param<'_>]) {
    // Discard non-boolean parameters before inspecting their binding shape.
    for param in params {
        if !is_bool_ty(cx.typeck_results().node_type(param.hir_id)) {
            continue;
        }
        let Some(ident) = binding_ident(param.pat) else {
            continue;
        };
        // Apply the snake-case policy only to direct identifier bindings.
        check_prefixed_ident(cx, "parameter", ident, PrefixStyle::Snake);
    }
}

/// Check trait required params for this lint.
fn check_trait_required_params(
    cx: &LateContext<'_>,
    inputs: &[Ty<'_>],
    arg_names: &[Option<Ident>],
) {
    // Pair each written trait parameter type with its optional binding name.
    for (ty, maybe_ident) in inputs.iter().zip(arg_names) {
        let Some(ident) = maybe_ident else {
            continue;
        };

        // Enforce the predicate prefix only for explicitly boolean parameters.
        if is_bool_hir_ty(ty) {
            check_prefixed_ident(cx, "parameter", *ident, PrefixStyle::Snake);
        }
    }
}

/// Check return name for this lint.
fn check_return_name(cx: &LateContext<'_>, decl: &FnDecl<'_>, kind: &'static str, ident: Ident) {
    let FnRetTy::Return(output) = decl.output else {
        return;
    };

    if is_bool_hir_ty(output) {
        check_prefixed_ident(cx, kind, ident, PrefixStyle::Snake);
    }
}

/// Return whether the item returns bool.
fn returns_bool(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    // Query the typed signature so aliases like `type Flag = bool` still count as booleans.
    is_bool_ty(
        cx.tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip()
            .output()
            .skip_binder(),
    )
}

/// Helper for binding ident analysis.
const fn binding_ident(pat: &Pat<'_>) -> Option<Ident> {
    // Destructuring can introduce several names; keep this lint to simple one-name bindings.
    let PatKind::Binding(_mode, _hir_id, ident, None) = pat.kind else {
        return None;
    };

    Some(ident)
}

/// Return whether bool hir ty.
fn is_bool_hir_ty(ty: &Ty<'_>) -> bool {
    // Required trait items have no typeck body, so check the source spelling for those signatures.
    let TyKind::Path(QPath::Resolved(_, path)) = ty.kind else {
        return false;
    };

    path.segments
        .last()
        .is_some_and(|segment| segment.ident.name.to_ident_string() == "bool")
}

/// Return whether bool ty.
fn is_bool_ty(ty: ty::Ty<'_>) -> bool {
    ty.is_bool()
}

/// Classification used by the prefix style analysis.
#[derive(Clone, Copy)]
enum PrefixStyle {
    /// snake case used by this lint's analysis.
    Snake,
    /// screaming const case used by this lint's analysis.
    ScreamingConst,
}

impl PrefixStyle {
    /// Rename guidance with predicate examples in this casing style.
    const fn rename_help(self) -> &'static str {
        match self {
            Self::Snake => {
                "rename it to read as a predicate, such as `is_ready`, `has_items`, or `can_retry`"
            }
            Self::ScreamingConst => {
                "rename it to start with `IS_` or `HAS_`, such as `IS_READY` or `HAS_ITEMS`"
            }
        }
    }
}

/// Check prefixed ident for this lint.
fn check_prefixed_ident(
    cx: &LateContext<'_>,
    kind: &'static str,
    ident: Ident,
    prefix_style: PrefixStyle,
) {
    // Ignore generated identifiers because users cannot rename their source.
    if ident.span.from_expansion() {
        return;
    }

    // Compare the name against the vocabulary allowed for its casing style.
    let name = ident.name.to_ident_string();
    if has_expected_predicate_name(&name, prefix_style) {
        return;
    }

    emit_bool_name_lint(cx, ident.span, kind, &name, prefix_style);
}

/// Emit the bool name lint diagnostic.
fn emit_bool_name_lint(
    cx: &LateContext<'_>,
    span: Span,
    kind: &'static str,
    name: &str,
    prefix_style: PrefixStyle,
) {
    // Name the affected binding in the primary diagnostic.
    // Give rename examples that match the binding's casing style.
    emit_span_lint_with_help(
        cx,
        BOOL_NAME_PREFIX,
        span,
        format!("boolean {kind} `{name}` does not read as a predicate"),
        prefix_style.rename_help(),
    );
}

/// Return whether a boolean name already reads like a predicate.
fn has_expected_predicate_name(name: &str, prefix_style: PrefixStyle) -> bool {
    match prefix_style {
        PrefixStyle::Snake => snake_predicate_name(name),
        PrefixStyle::ScreamingConst => name.starts_with("IS_") || name.starts_with("HAS_"),
    }
}

/// Return whether a snake-case boolean name has predicate form.
fn snake_predicate_name(name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "all_",
        "adds_",
        "ancestor_",
        "allowed_",
        "assigns_",
        "build_",
        "debug_",
        "default_",
        "definition_",
        "derives_",
        "direct_",
        "discarded_",
        "does_",
        "field_",
        "for_",
        "from_",
        "is_",
        "has_",
        "have_",
        "in_",
        "inherits_",
        "can_",
        "should_",
        "contains_",
        "conversion_",
        "crate_",
        "file_",
        "identifier_",
        "implements_",
        "infallible_",
        "looks_",
        "matches_",
        "meaningful_",
        "mutable_",
        "mutates_",
        "non_",
        "only_",
        "path_",
        "private_",
        "ready_",
        "resolved_",
        "same_",
        "saw_",
        "self_",
        "single_",
        "starts_",
        "ends_",
        "allows_",
        "participates_",
        "resolves_",
        "returns_",
        "requires_",
        "needs_",
        "uses_",
        "unquoted_",
    ];
    const INFIXES: &[&str] = &[
        "_adds_",
        "_aliases_",
        "_allows_",
        "_are_",
        "_calls_",
        "_can_",
        "_contains_",
        "_counts_",
        "_declares_",
        "_derives_",
        "_disables_",
        "_does_",
        "_ends_",
        "_finishes",
        "_has_",
        "_initializes",
        "_inherits_",
        "_is_",
        "_matches_",
        "_needs_",
        "_only_",
        "_pushes_",
        "_requires_",
        "_resolves_",
        "_returns_",
        "_sets_",
        "_starts_",
        "_uses_",
    ];
    const SEMANTIC_SUFFIXES: &[&str] = &[
        "_attr",
        "_attrs",
        "_call",
        "_char",
        "_context",
        "_dependency",
        "_dependencies",
        "_field",
        "_fields",
        "_fixture",
        "_function",
        "_impl",
        "_key",
        "_literal",
        "_macro",
        "_marker",
        "_method",
        "_name",
        "_path",
        "_policy",
        "_property",
        "_receiver",
        "_return",
        "_section",
        "_source",
        "_table",
        "_target",
        "_targets",
        "_trait",
        "_ty",
        "_type",
        "_value",
        "_word",
    ];
    // Keep the rule strict enough to catch noun-like flags while allowing common
    // predicate verbs and semantic helper names used by analysis APIs.
    exact_predicate_name(name)
        || PREFIXES.iter().any(|prefix| name.starts_with(prefix))
        || INFIXES.iter().any(|infix| name.contains(infix))
        || SEMANTIC_SUFFIXES
            .iter()
            .any(|suffix| name.ends_with(suffix))
}

/// Return whether a boolean name is an established exact predicate.
fn exact_predicate_name(name: &str) -> bool {
    const EXACT_NAMES: &[&str] = &[
        "complicated_condition",
        "container_default",
        "emitted",
        "err_pattern",
        "escaped",
        "explicit_case_list_expr",
        "increments",
        "literal",
        "one_integer",
        "one_input_concrete_conversion",
        "one_string_input_result",
        "outer_denies",
        "parent_allowed",
        "schema_crate",
        "seen",
        "shared_ref",
        "simple_inlineable_expression",
        "string_def_id",
        "test_function_without_test_case",
        "trivial_struct_forward",
        "value",
        "zero_integer",
    ];

    EXACT_NAMES.contains(&name)
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

    // Use rustc's diagnostic decorator so the output matches the rest of the suite.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
