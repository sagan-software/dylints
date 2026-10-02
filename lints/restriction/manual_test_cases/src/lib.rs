#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc syntax variants"
)]
#![warn(unused_extern_crates)]

//! A lint to check for manually iterated test cases.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, Body, Expr, ExprKind, HirId, ItemKind, LangItem, LetStmt, MatchSource, Pat, PatKind,
    Stmt, StmtKind,
    attrs::AttributeKind,
    def::{DefKind, Res},
    intravisit::{self, FnKind, Visitor},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{Span, Symbol, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_TEST_CASES,
    Warn,
    "test manually iterates over cases instead of using `test-case`",
    ManualTestCases
}

impl<'tcx> LateLintPass<'tcx> for ManualTestCases {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        // Restrict analysis to test functions that do not already use `test-case`.
        if matches!(kind, FnKind::Closure)
            || !test_function_without_test_case(cx, local_def_id, kind)
        {
            return;
        }

        if let Some(span) = ManualCaseLoopFinder::find(cx, body) {
            emit_span_lint_with_help(
                cx,
                MANUAL_TEST_CASES,
                span,
                "test manually iterates over cases",
                "use `#[test_case(...)]` so each case has its own named test result",
            );
        }
    }
}

/// State used by the manual case loop finder analysis.
struct ManualCaseLoopFinder<'cx, 'tcx> {
    /// cx stored for this lint's analysis.
    cx: &'cx LateContext<'tcx>,
    /// case list bindings stored for this lint's analysis.
    case_list_bindings: Vec<CaseListBinding>,
    /// offense stored for this lint's analysis.
    offense: Option<Span>,
}

/// State used by the case list binding analysis.
#[derive(Clone, Copy)]
struct CaseListBinding {
    /// The compiler identity of the local binding.
    hir_id: HirId,
    /// literal stored for this lint's analysis.
    literal: bool,
}

impl<'cx, 'tcx> ManualCaseLoopFinder<'cx, 'tcx> {
    /// Helper for find analysis.
    fn find(cx: &'cx LateContext<'tcx>, body: &'tcx Body<'tcx>) -> Option<Span> {
        let mut finder = Self {
            cx,
            case_list_bindings: Vec::new(),
            offense: None,
        };

        // Visit after test filtering so ordinary loops outside tests never participate.
        finder.visit_expr(body.value);
        finder.offense
    }
}

impl<'tcx> Visitor<'tcx> for ManualCaseLoopFinder<'_, 'tcx> {
    /// Helper for visit block analysis.
    fn visit_block(&mut self, block: &'tcx rustc_hir::Block<'tcx>) {
        let scope_len = self.case_list_bindings.len();

        // Walk statements in order so local bindings obey normal lexical shadowing.
        for stmt in block.stmts {
            self.visit_stmt(stmt);
        }
        if let Some(expr) = block.expr {
            self.visit_expr(expr);
        }

        // Restore outer bindings after leaving this lexical block.
        self.case_list_bindings.truncate(scope_len);
    }

    /// Helper for visit stmt analysis.
    fn visit_stmt(&mut self, stmt: &'tcx Stmt<'tcx>) {
        match stmt.kind {
            StmtKind::Let(local) => self.visit_local_binding(local),
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => self.visit_expr(expr),
            StmtKind::Item(_) => {}
        }
    }

    /// Helper for visit expr analysis.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.offense.is_none() {
            // Match compiler-resolved iteration shapes before tracing their case source.
            let manual_iteration =
                for_loop_source(self.cx, expr).or_else(|| iterator_for_each_source(self.cx, expr));
            if let Some((source, span)) = manual_iteration
                && literal_case_source(self.cx, source, &self.case_list_bindings)
            {
                self.offense = Some(span);
            }
        }

        intravisit::walk_expr(self, expr);
    }
}

impl<'tcx> ManualCaseLoopFinder<'_, 'tcx> {
    /// Helper for visit local binding analysis.
    fn visit_local_binding(&mut self, local: &'tcx LetStmt<'tcx>) {
        if let Some(init) = local.init {
            self.visit_expr(init);
        }

        // Record every binding so a dynamic local shadows any outer literal list of the same name.
        if let Some(hir_id) = binding_hir_id(local.pat) {
            self.case_list_bindings.push(CaseListBinding {
                hir_id,
                literal: local.init.is_some_and(|init| {
                    literal_case_source(self.cx, init, &self.case_list_bindings)
                }),
            });
        }

        if let Some(els) = local.els {
            // Visit let-else blocks after recording the local's binding state.
            self.visit_block(els);
        }
    }
}

/// Helper for test function without test case analysis.
fn test_function_without_test_case(
    cx: &LateContext<'_>,
    local_def_id: LocalDefId,
    kind: FnKind<'_>,
) -> bool {
    let hir_id = cx.tcx.local_def_id_to_hir_id(local_def_id);
    let attrs: &[Attribute] = cx.tcx.hir_attrs(hir_id);

    // `#[test_case]` already gives each case its own result, so skip those tests entirely.
    !has_test_case_attr(attrs)
        && (is_test_function(cx, local_def_id)
            || fn_name(kind).is_some_and(|name| name.starts_with("test_")))
}

/// Return whether test case attr is present.
fn has_test_case_attr(attrs: &[Attribute]) -> bool {
    let test_case = Symbol::intern("test_case");

    // Accept only the real single-segment form and its crate-qualified spelling.
    attrs.iter().any(|attr| {
        let segments = attr.path();
        matches!(
            segments.as_slice(),
            [single] if *single == test_case
        ) || matches!(
            segments.as_slice(),
            [first, second] if *first == test_case && *second == test_case
        )
    })
}

/// Return whether a function is a `#[test]` function in a `--test` build.
fn is_test_function(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    // The test harness replaces `#[test]` with a same-named marker constant in the same module.
    let name = cx.tcx.item_name(local_def_id.to_def_id());
    let module = cx.tcx.parent_module_from_def_id(local_def_id);
    cx.tcx.hir_module_free_items(module).any(|item_id| {
        let item = cx.tcx.hir_item(item_id);
        matches!(item.kind, ItemKind::Const(ident, ..) if ident.name == name)
            && cx
                .tcx
                .hir_attrs(item.hir_id())
                .iter()
                .any(|attr| matches!(attr, Attribute::Parsed(AttributeKind::RustcTestMarker(_))))
    })
}

/// Return the fn name.
fn fn_name(kind: FnKind<'_>) -> Option<String> {
    match kind {
        FnKind::ItemFn(ident, ..) | FnKind::Method(ident, _) => Some(ident.name.to_ident_string()),
        FnKind::Closure => None,
    }
}

/// Return the binding compiler identity.
const fn binding_hir_id(pat: &Pat<'_>) -> Option<HirId> {
    let PatKind::Binding(_mode, hir_id, _ident, None) = pat.kind else {
        return None;
    };

    Some(hir_id)
}

/// Helper for array or slice literal analysis.
const fn array_or_slice_literal(expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Array(_) | ExprKind::Repeat(..) => true,
        ExprKind::AddrOf(_borrow_kind, _mutability, inner) => array_or_slice_literal(inner),
        _ => false,
    }
}

/// Recover the source expression and header span from a standard `for` loop.
fn for_loop_source<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, Span)> {
    // Peel the temporary wrapper and match rustc's standard for-loop shell.
    let expr = peel_drop_temps(expr);
    match expr.kind {
        ExprKind::Match(iter_expr, [iter_arm], MatchSource::ForLoopDesugar) => {
            Some((iter_expr, iter_arm))
        }
        _ => None,
    }
    .and_then(|(iter_expr, iter_arm)| match iter_expr.kind {
        ExprKind::Call(callee, [source]) => Some((callee, source, iter_arm)),
        _ => None,
    })
    .and_then(|(callee, source, iter_arm)| match callee.kind {
        ExprKind::Path(qpath) => Some((callee, qpath, source, iter_arm)),
        _ => None,
    })
    // Resolve the iterator constructor to the standard IntoIterator trait.
    .and_then(
        |(callee, qpath, source, iter_arm)| match cx.qpath_res(&qpath, callee.hir_id) {
            Res::Def(_, def_id) => Some((def_id, source, iter_arm)),
            _ => None,
        },
    )
    .filter(|(def_id, _, _)| cx.tcx.is_lang_item(*def_id, LangItem::IntoIterIntoIter))
    // Recover the user-written loop header span from the generated loop.
    .and_then(|(_, source, iter_arm)| match iter_arm.body.kind {
        ExprKind::Loop(_block, _label, rustc_hir::LoopSource::ForLoop, header_span) => {
            let diagnostic_span = if source.span.from_expansion() {
                source.span.source_callsite()
            } else {
                header_span
            };
            Some((source, diagnostic_span))
        }
        _ => None,
    })
}

/// Recover the receiver and method span from standard `Iterator::for_each`.
fn iterator_for_each_source<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, Span)> {
    // Resolve the associated trait item so custom same-named methods stay excluded.
    match expr.kind {
        ExprKind::MethodCall(segment, receiver, _, _) => Some((segment, receiver)),
        _ => None,
    }
    .and_then(|(segment, receiver)| {
        cx.typeck_results()
            .type_dependent_def_id(expr.hir_id)
            .map(|def_id| (segment, receiver, def_id))
    })
    .and_then(|(segment, receiver, def_id)| {
        cx.tcx
            .opt_associated_item(def_id)
            .and_then(|associated_item| associated_item.trait_item_or_self().ok())
            .and_then(|trait_item| cx.tcx.trait_of_assoc(trait_item))
            .map(|trait_def_id| (segment, receiver, def_id, trait_def_id))
    })
    // Trace the associated item back to the standard Iterator diagnostic item.
    .filter(|(_, _, def_id, trait_def_id)| {
        cx.tcx.item_name(*def_id).as_str() == "for_each"
            && cx.tcx.is_diagnostic_item(sym::Iterator, *trait_def_id)
    })
    .map(|(segment, receiver, _, _)| (receiver, segment.ident.span))
}

/// Return whether an expression ultimately comes from a literal case table.
fn literal_case_source(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    case_list_bindings: &[CaseListBinding],
) -> bool {
    // Peel compiler wrappers before following only resolved local and constant origins.
    let expr = peel_drop_temps(expr);
    if array_or_slice_literal(expr) || vec_macro_literal(cx, expr.span) {
        return true;
    }

    match expr.kind {
        ExprKind::Path(qpath) => match cx.qpath_res(&qpath, expr.hir_id) {
            Res::Local(hir_id) => case_list_bindings
                .iter()
                .rev()
                .find(|binding| binding.hir_id == hir_id)
                .is_some_and(|binding| binding.literal),
            Res::Def(
                DefKind::Const { .. } | DefKind::Static { .. } | DefKind::AssocConst { .. },
                def_id,
            ) => def_id
                .as_local()
                .and_then(|local_def_id| cx.tcx.hir_maybe_body_owned_by(local_def_id))
                .is_some_and(|body| literal_case_source(cx, body.value, case_list_bindings)),
            _ => false,
        },
        ExprKind::AddrOf(_, _, inner) | ExprKind::MethodCall(_, inner, _, _) => {
            literal_case_source(cx, inner, case_list_bindings)
        }
        _ => false,
    }
}

/// Return whether this expression is the expansion of the standard `vec!` macro.
fn vec_macro_literal(cx: &LateContext<'_>, span: Span) -> bool {
    // `vec![a, b]` lowers to a call whose span records the resolved macro definition.
    span.ctxt()
        .outer_expn_data()
        .macro_def_id
        .and_then(|def_id| cx.tcx.get_diagnostic_name(def_id))
        .is_some_and(|name| name.as_str() == "vec_macro")
}

/// Remove compiler-generated temporary wrappers.
const fn peel_drop_temps<'hir>(expr: &'hir Expr<'hir>) -> &'hir Expr<'hir> {
    match expr.kind {
        ExprKind::DropTemps(inner) => peel_drop_temps(inner),
        _ => expr,
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
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
