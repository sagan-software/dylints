//! Shared semantic helpers for the method-chaining lint family.

#![allow(
    dead_code,
    reason = "each constituent lint uses a different subset of these shared helpers"
)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the shared lint helpers intentionally ignore diagnostic builders and unmatched rustc variants"
)]

use rustc_errors::DiagDecorator;
use rustc_hir::{
    BindingMode, Block, ByRef, Expr, ExprKind, HirId, LangItem, MatchSource, Pat, PatKind, Stmt,
    StmtKind,
    def::{DefKind, Res},
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, Symbol, sym};

/// The source, pattern, and body recovered from one standard `for` loop.
pub(crate) struct ForLoop<'tcx> {
    /// Span of the complete `for` loop.
    pub(crate) span: Span,
    /// Expression passed to `IntoIterator::into_iter`.
    pub(crate) source: &'tcx Expr<'tcx>,
    /// User-written loop pattern that binds each item.
    pub(crate) pat: &'tcx Pat<'tcx>,
    /// User-written loop body.
    pub(crate) body: &'tcx Block<'tcx>,
}

/// Recovers a standard `for` loop from rustc's desugared HIR.
///
/// rustc lowers `for pat in source { body }` to a match on
/// `IntoIterator::into_iter(source)` whose arm loops over a second match on
/// `next(&mut iter)` with `None => break` and `Some(pat) => body` arms.
pub(crate) fn for_loop<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<ForLoop<'tcx>> {
    // Peel the outer temporary wrapper and descend through the generated shell.
    let expr = peel_drop_temps(expr);
    if let ExprKind::Match(iter_expr, [iter_arm], MatchSource::ForLoopDesugar) = expr.kind
        && let ExprKind::Call(callee, [source]) = iter_expr.kind
        && resolved_into_iter(cx, callee)
        && let ExprKind::Loop(loop_block, ..) = iter_arm.body.kind
        && let Some(next_match) = block_only_expr(loop_block)
        && let ExprKind::Match(_, arms, MatchSource::ForLoopDesugar) = next_match.kind
    {
        // Select the generated `Some(pat)` arm, lowered as a one-field struct pattern.
        arms.iter()
            .find_map(|arm| match (arm.pat.kind, arm.body.kind) {
                (PatKind::Struct(_, [field], _), ExprKind::Block(body, _)) => Some(ForLoop {
                    span: expr.span,
                    source,
                    pat: field.pat,
                    body,
                }),
                _ => None,
            })
    } else {
        None
    }
}

/// Returns the only expression in a block.
pub(crate) const fn block_only_expr<'tcx>(block: &'tcx Block<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match (block.stmts, block.expr) {
        ([stmt], None) => stmt_expr(stmt),
        ([], Some(expr)) => Some(expr),
        _ => None,
    }
}

/// Returns the expression carried by an expression statement.
pub(crate) const fn stmt_expr<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
        StmtKind::Let(_) | StmtKind::Item(_) => None,
    }
}

/// Removes compiler-generated temporary wrappers.
pub(crate) fn peel_drop_temps<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match expr.kind {
        ExprKind::DropTemps(inner) => peel_drop_temps(inner),
        _ => expr,
    }
}

/// Returns the source text at the user-written call site for one span.
pub(crate) fn snippet(cx: &LateContext<'_>, span: Span) -> Option<String> {
    cx.sess()
        .source_map()
        .span_to_snippet(span.source_callsite())
        .ok()
}

/// Returns whether a path resolves to the `IntoIterator::into_iter` lang item.
fn resolved_into_iter(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(
        expr.kind,
        ExprKind::Path(qpath) if matches!(
            cx.qpath_res(&qpath, expr.hir_id),
            Res::Def(_, def_id) if cx.tcx.is_lang_item(def_id, LangItem::IntoIterIntoIter)
        )
    )
}

/// Returns the collection and method names of an inherent collection method.
///
/// The first symbol is the collection's diagnostic item, such as `Vec`, and the
/// second is the method name, such as `push`.
pub(crate) fn standard_collection_method(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
) -> Option<(Symbol, Symbol)> {
    let (self_ty, method) = inherent_method(cx, expr)?;
    standard_collection_name(cx, self_ty).map(|collection| (collection, method))
}

/// Returns the impl self type and method name of a resolved inherent method call.
pub(crate) fn inherent_method<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'_>,
) -> Option<(Ty<'tcx>, Symbol)> {
    // Paths can also carry type-dependent definitions, so require a method call.
    if !matches!(expr.kind, ExprKind::MethodCall(..)) {
        return None;
    }
    // Resolve the call, then require that its parent is an inherent impl.
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    let impl_id = cx.tcx.opt_parent(def_id)?;
    let DefKind::Impl { of_trait: false } = cx.tcx.def_kind(impl_id) else {
        return None;
    };
    // The impl self type names the receiver type after autoderef.
    let self_ty = cx
        .tcx
        .type_of(impl_id)
        .instantiate_identity()
        .skip_normalization();
    Some((self_ty, cx.tcx.item_name(def_id)))
}

/// Returns whether a type is the standard `Option` type.
pub(crate) fn is_option(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return false;
    };
    cx.tcx.is_diagnostic_item(sym::Option, adt.did())
}

/// Returns whether a type is the standard `Result` type.
pub(crate) fn is_result(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return false;
    };
    cx.tcx.is_diagnostic_item(sym::Result, adt.did())
}

/// Returns the diagnostic item of a supported standard collection type.
fn standard_collection_name(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<Symbol> {
    // Resolve the collection by diagnostic item so local lookalikes stay distinct.
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return None;
    };
    cx.tcx.get_diagnostic_name(adt.did()).filter(|name| {
        matches!(
            name.as_str(),
            "Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "HashMap" | "BTreeMap"
        )
    })
}

/// Returns whether a type is one of the supported standard collections.
pub(crate) fn is_standard_collection(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    standard_collection_name(cx, ty).is_some()
}

/// Returns whether a resolution names the constructor of a lang-item variant.
pub(crate) fn is_lang_ctor(cx: &LateContext<'_>, res: Res, item: LangItem) -> bool {
    // A tuple-variant constructor's parent is the variant that carries the lang item.
    let Res::Def(DefKind::Ctor(..), ctor) = res else {
        return false;
    };
    cx.tcx
        .opt_parent(ctor)
        .is_some_and(|variant| cx.tcx.is_lang_item(variant, item))
}

/// Returns the inner pattern of a one-field lang-item variant pattern such as `Some(inner)`.
pub(crate) fn lang_ctor_pat<'tcx>(
    cx: &LateContext<'_>,
    pat: &'tcx Pat<'tcx>,
    item: LangItem,
) -> Option<&'tcx Pat<'tcx>> {
    let PatKind::TupleStruct(qpath, [inner], _) = pat.kind else {
        return None;
    };
    is_lang_ctor(cx, cx.qpath_res(&qpath, pat.hir_id), item).then_some(inner)
}

/// Returns the argument of a one-field lang-item variant construction such as `Ok(value)`.
pub(crate) fn lang_ctor_call<'tcx>(
    cx: &LateContext<'_>,
    expr: &'tcx Expr<'tcx>,
    item: LangItem,
) -> Option<&'tcx Expr<'tcx>> {
    // Resolve the callee path so a local function named `Ok` stays distinct.
    let ExprKind::Call(callee, [argument]) = peel_drop_temps(expr).kind else {
        return None;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return None;
    };
    is_lang_ctor(cx, cx.qpath_res(&qpath, callee.hir_id), item).then_some(argument)
}

/// Returns the local binding resolved by a path expression.
pub(crate) fn local_binding(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    // Use HIR resolution so shadowed names remain distinct.
    let expr = peel_drop_temps(expr);
    let ExprKind::Path(qpath) = expr.kind else {
        return None;
    };
    // Statics, constants, and functions resolve to definitions, not locals.
    let Res::Local(id) = cx.qpath_res(&qpath, expr.hir_id) else {
        return None;
    };
    Some(id)
}

/// Returns the binding of a by-value identifier pattern without a subpattern.
pub(crate) const fn simple_binding(pat: &Pat<'_>) -> Option<HirId> {
    match pat.kind {
        PatKind::Binding(BindingMode(ByRef::No, _), id, _, None) => Some(id),
        _ => None,
    }
}

/// Returns whether an expression is an argument-free associated `new` or
/// `default` call.
pub(crate) fn is_empty_constructor(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Resolve the callee so the constructor name comes from its definition.
    let ExprKind::Call(callee, []) = peel_drop_temps(expr).kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    // Free functions are rejected because their result can hold items.
    let Res::Def(DefKind::AssocFn, def_id) = cx.qpath_res(&qpath, callee.hir_id) else {
        return false;
    };
    matches!(cx.tcx.item_name(def_id).as_str(), "new" | "default")
}

/// Early exits found in one expression, excluding nested closure and async bodies.
pub(crate) struct EarlyExits<'tcx> {
    /// Operands of each `?` operator, in visiting order.
    pub(crate) try_operands: Vec<&'tcx Expr<'tcx>>,
    /// Whether the expression contains `return`, `break`, `continue`, `become`,
    /// `yield`, or `.await`.
    pub(crate) has_other: bool,
}

/// Collects the early exits in one expression from its HIR.
pub(crate) fn early_exits<'tcx>(expr: &'tcx Expr<'tcx>) -> EarlyExits<'tcx> {
    let mut exits = EarlyExits {
        try_operands: Vec::new(),
        has_other: false,
    };
    exits.visit_expr(expr);
    exits
}

/// Returns whether an expression contains any `?` or other early exit.
pub(crate) fn contains_control_flow(expr: &Expr<'_>) -> bool {
    let exits = early_exits(expr);
    exits.has_other || !exits.try_operands.is_empty()
}

impl<'tcx> Visitor<'tcx> for EarlyExits<'tcx> {
    /// Records early exits without entering nested bodies.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            // `operand?` lowers to a match on `Try::branch(operand)` whose arm returns.
            // Record the operand and skip the generated arms.
            ExprKind::Match(scrutinee, _, MatchSource::TryDesugar(_)) => {
                if let ExprKind::Call(_, operands) = scrutinee.kind {
                    self.try_operands.extend(operands);
                }
                walk_expr(self, scrutinee);
            }
            ExprKind::Match(_, _, MatchSource::AwaitDesugar)
            | ExprKind::Ret(_)
            | ExprKind::Break(..)
            | ExprKind::Continue(_)
            | ExprKind::Become(_)
            | ExprKind::Yield(..) => self.has_other = true,
            _ => walk_expr(self, expr),
        }
    }
}

/// Emits a help-only lint diagnostic.
pub(crate) fn emit(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Keep these lints help-only until exact closure source can be rendered safely.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}
