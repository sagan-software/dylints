#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores diagnostic builders"
)]

//! A lint to check for one-use private expression helpers.
//!
//! It counts resolved references to each private free function across every
//! body in the crate, then reports a single-expression helper whose only
//! reference is one direct call from another function. A source check keeps
//! helpers that `cfg`-disabled code also calls.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lexer;
extern crate rustc_middle;
extern crate rustc_parse;
extern crate rustc_span;

use std::collections::HashMap;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, Body, Expr, ExprKind, GenericParamKind, QPath,
    attrs::AttributeKind,
    def::{DefKind, Res},
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{TyCtxt, Visibility};
use rustc_parse::lexer::nfc_normalize;
use rustc_span::{
    BytePos, Span, Symbol,
    def_id::{CRATE_DEF_ID, LocalDefId},
    sym,
};

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub ONE_USE_PRIVATE_HELPER,
    Warn,
    "private one-expression helper is only called once",
    OneUsePrivateHelper,
    OneUsePrivateHelper::default()
}

/// Late pass that caches the crate's references to local functions.
#[derive(Default)]
struct OneUsePrivateHelper {
    /// References to each local function, built on the first checked function.
    references: Option<HashMap<LocalDefId, Vec<Reference>>>,
    /// Identifier counts indexed once for each immutable source file.
    source_identifiers: HashMap<BytePos, HashMap<Symbol, usize>>,
}

/// One resolved path to a local function.
#[derive(Clone, Copy)]
struct Reference {
    /// Span of the path expression.
    span: Span,
    /// Whether the path is the callee of a call expression.
    is_call: bool,
    /// Body owner that contains the path.
    owner: LocalDefId,
}

impl<'tcx> LateLintPass<'tcx> for OneUsePrivateHelper {
    /// Reports a private single-expression helper with exactly one call.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Apply the cheap item checks before building the crate-wide reference map.
        let Some(name) = candidate_name(cx, kind, body, span, local_def_id) else {
            return;
        };
        let references = self
            .references
            .get_or_insert_with(|| collect_references(cx.tcx));

        // Require one direct call from a different function, written outside a macro.
        let Some(&[call]) = references.get(&local_def_id).map(Vec::as_slice) else {
            return;
        };
        let is_outside_call = call.is_call
            && !call.span.from_expansion()
            && cx.tcx.typeck_root_def_id(call.owner.to_def_id()) != local_def_id.to_def_id();
        if !is_outside_call || !is_named_twice_in_file(cx, span, name, &mut self.source_identifiers)
        {
            return;
        }

        cx.emit_span_lint(
            ONE_USE_PRIVATE_HELPER,
            cx.tcx.def_span(local_def_id),
            DiagDecorator(|diag| {
                let _ = diag.primary_message("private one-expression helper is only called once");
                let _ = diag.span_note(call.span, "the only call is here");
                let _ = diag.help(
                    "inline the expression at the call site unless the helper names validation, a hook, fixture setup, or a domain rule",
                );
            }),
        );
    }
}

/// Returns the helper's name when the item itself qualifies.
fn candidate_name(
    cx: &LateContext<'_>,
    kind: FnKind<'_>,
    body: &Body<'_>,
    span: Span,
    local_def_id: LocalDefId,
) -> Option<Symbol> {
    // Restrict the analysis to safe, synchronous free functions without type parameters.
    let FnKind::ItemFn(ident, generics, header) = kind else {
        return None;
    };
    let has_type_params = generics
        .params
        .iter()
        .any(|param| !matches!(param.kind, GenericParamKind::Lifetime { .. }));
    if span.from_expansion() || has_type_params || header.is_unsafe() || header.is_async() {
        return None;
    }

    // Require visibility limited to the defining module.
    let module = cx.tcx.parent_module_from_def_id(local_def_id).to_def_id();
    if cx.tcx.visibility(local_def_id) != Visibility::Restricted(module) {
        return None;
    }

    // Other attributes, such as `#[test]` or `#[no_mangle]`, give the function an implicit role.
    let attrs = cx
        .tcx
        .hir_attrs(cx.tcx.local_def_id_to_hir_id(local_def_id));
    let has_role_attr = attrs.iter().any(|attr| !is_neutral_attr(attr));
    (!has_role_attr && single_expression_body(body) && !excluded_helper_name(ident.name.as_str()))
        .then_some(ident.name)
}

/// Returns whether an attribute leaves the function's role unchanged.
fn is_neutral_attr(attr: &Attribute) -> bool {
    // Rustc parses some built-in attributes, so match those by kind and the rest by name.
    attr.doc_str().is_some()
        || matches!(
            attr,
            Attribute::Parsed(AttributeKind::Inline(..) | AttributeKind::MustUse { .. })
        )
        || [
            sym::allow,
            sym::warn,
            sym::deny,
            sym::forbid,
            sym::expect,
            sym::inline,
            sym::must_use,
        ]
        .iter()
        .any(|name| attr.has_name(*name))
}

/// Returns whether the body is one tail expression without control flow.
const fn single_expression_body(body: &Body<'_>) -> bool {
    // Require a statement-free block with one inlineable tail expression.
    let ExprKind::Block(block, _) = body.value.kind else {
        return false;
    };
    let Some(expr) = block.expr else {
        return false;
    };

    block.stmts.is_empty()
        && !matches!(
            expr.kind,
            ExprKind::If(..)
                | ExprKind::Match(..)
                | ExprKind::Loop(..)
                | ExprKind::Closure(..)
                | ExprKind::Block(..)
                | ExprKind::Ret(..)
                | ExprKind::Break(..)
        )
}

/// Returns whether the name states a role that keeps the helper useful.
fn excluded_helper_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let words = lower
        .split('_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    // Names that encode validation, tests, framework hooks, construction roles, or domain
    // predicates carry review value even when a first pass sees only one direct call.
    words.iter().any(|word| {
        matches!(
            *word,
            "validate"
                | "validation"
                | "valid"
                | "check"
                | "ensure"
                | "assert"
                | "test"
                | "tests"
                | "fixture"
                | "fixtures"
                | "build"
                | "builder"
                | "make"
                | "hook"
                | "rule"
                | "policy"
                | "invariant"
        )
    }) || matches!(
        words.first().copied(),
        Some("can" | "should" | "is" | "has" | "must")
    )
}

/// Collects every resolved path to a local function in every body of the crate.
fn collect_references(tcx: TyCtxt<'_>) -> HashMap<LocalDefId, Vec<Reference>> {
    // Start with a placeholder owner; the loop sets the real owner before each visit.
    let mut collector = ReferenceCollector {
        owner: CRATE_DEF_ID,
        references: HashMap::new(),
    };

    // Each closure and constant has its own body owner, so every body is visited once.
    for owner in tcx.hir_body_owners() {
        collector.owner = owner;
        collector.visit_body(tcx.hir_body_owned_by(owner));
    }
    collector.references
}

/// Visitor that records paths to local functions.
struct ReferenceCollector {
    /// Body owner currently being visited.
    owner: LocalDefId,
    /// Recorded references keyed by the referenced function.
    references: HashMap<LocalDefId, Vec<Reference>>,
}

impl ReferenceCollector {
    /// Records one reference to a local function.
    fn record(&mut self, function: LocalDefId, span: Span, is_call: bool) {
        self.references
            .entry(function)
            .or_default()
            .push(Reference {
                span,
                is_call,
                owner: self.owner,
            });
    }
}

/// Returns the local function that a path expression resolves to.
fn local_function(expr: &Expr<'_>) -> Option<LocalDefId> {
    // Accept only a resolved path to a function defined in this crate.
    let ExprKind::Path(QPath::Resolved(_, path)) = expr.kind else {
        return None;
    };
    let Res::Def(DefKind::Fn, def_id) = path.res else {
        return None;
    };
    def_id.as_local()
}

impl<'tcx> Visitor<'tcx> for ReferenceCollector {
    /// Records calls and value uses, then descends into child expressions.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Record a callee as a call and skip it so it is not counted again as a value use.
        if let ExprKind::Call(callee, args) = expr.kind
            && let Some(function) = local_function(callee)
        {
            self.record(function, callee.span, true);
            for arg in args {
                self.visit_expr(arg);
            }
            return;
        }

        // Any other path to a local function is a value use, such as `.map(helper)`.
        if let Some(function) = local_function(expr) {
            self.record(function, expr.span, false);
        }
        walk_expr(self, expr);
    }
}

/// Returns whether the name appears exactly twice in the helper's source file.
///
/// HIR omits code removed by `cfg`, such as a `#[cfg(test)]` module in a
/// library build. This source check keeps a helper that such code also calls,
/// because the definition and the one call are then not the only mentions.
fn is_named_twice_in_file(
    cx: &LateContext<'_>,
    span: Span,
    name: Symbol,
    files: &mut HashMap<BytePos, HashMap<Symbol, usize>>,
) -> bool {
    let source_file = cx.sess().source_map().lookup_source_file(span.lo());
    let Some(source) = source_file.src.as_deref() else {
        return false;
    };

    // Source files are immutable: tokenize each file once rather than once per helper.
    let counts = files.entry(source_file.start_pos).or_insert_with(|| {
        let mut counts = HashMap::new();
        for identifier in identifier_tokens(source) {
            // Match rustc's NFC normalization of written Unicode identifiers.
            *counts.entry(nfc_normalize(identifier)).or_insert(0) += 1;
        }
        counts
    });
    counts.get(&name) == Some(&2)
}

/// Borrows Rust identifier tokens, excluding comments and literal contents.
fn identifier_tokens(source: &str) -> impl Iterator<Item = &str> {
    // Token lengths preserve UTF-8 boundaries while raw identifiers share their plain name.
    let mut offset = 0;
    tokenize(source, FrontmatterAllowed::No).filter_map(move |token| {
        let start = offset;
        offset += token.len as usize;
        // Read the token through a checked UTF-8 range before borrowing its identifier.
        matches!(token.kind, TokenKind::Ident | TokenKind::RawIdent)
            .then(|| source.get(start..offset))
            .flatten()
            .map(|identifier| identifier.strip_prefix("r#").unwrap_or(identifier))
    })
}

/// Counts identifier tokens for focused lexer boundary tests.
#[cfg(test)]
fn identifier_occurrences(source: &str, name: &str) -> usize {
    identifier_tokens(source)
        .filter(|identifier| *identifier == name)
        .count()
}

/// Runs the UI fixtures.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

/// Counts code identifiers while ignoring a mention in a line comment.
#[test]
fn counts_whole_identifiers() {
    assert_eq!(identifier_occurrences("fn total() {} total()", "total"), 2);
    assert_eq!(identifier_occurrences("x // total\ntotal", "total"), 1);
}

/// Ignores longer identifiers that contain the name.
#[test]
fn ignores_partial_identifiers() {
    assert_eq!(
        identifier_occurrences("subtotal totals total_x", "total"),
        0
    );
}

/// Excludes every comment and literal form while preserving raw and Unicode identifiers.
#[test]
fn handles_lexer_boundaries() {
    let source = r##"/* total /* total */ total */ // total
"total" r#"total"# b"total" br#"total"# 't' b't'
r#total total café r#café"##;
    assert_eq!(identifier_occurrences(source, "total"), 2);
    assert_eq!(identifier_occurrences(source, "café"), 2);
}
