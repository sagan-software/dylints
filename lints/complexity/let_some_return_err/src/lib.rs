#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "rustc diagnostic builder results are configured through side effects, and the \
              pattern and expression checks treat every unlisted rustc variant as unsafe or \
              transparent"
)]

//! A lint to replace let-else `Option` error returns with `ok_or`.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::ops::ControlFlow;

use rustc_ast::{LitFloatType, LitIntType, LitKind};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    Block, Body, ByRef, Expr, ExprKind, FnDecl, LangItem, LetStmt, MatchSource, Pat, PatKind,
    QPath, Stmt, StmtKind, UnOp,
    def::{CtorKind, DefKind, Res},
    intravisit::{FnKind, Visitor, walk_expr, walk_stmt},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty, TypeVisitableExt as _};
use rustc_span::{Span, def_id::DefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub LET_SOME_RETURN_ERR,
    Warn,
    "`let Some` else branch returns `Err` manually",
    LetSomeReturnErr
}

impl<'tcx> LateLintPass<'tcx> for LetSomeReturnErr {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        _local_def_id: rustc_span::def_id::LocalDefId,
    ) {
        let Some(function_error_ty) = result_error_ty(cx, cx.typeck_results().expr_ty(body.value))
        else {
            return;
        };

        // Carry the enclosing `Result` error type so nested `let else` statements are only
        // considered where `?` can preserve the current function or closure error type.
        LetSomeVisitor {
            cx,
            function_error_ty,
        }
        .visit_expr(body.value);
    }
}

/// State used by the let some visitor analysis.
struct LetSomeVisitor<'cx, 'tcx> {
    /// cx stored for this lint's analysis.
    cx: &'cx LateContext<'tcx>,
    /// function error ty stored for this lint's analysis.
    function_error_ty: Ty<'tcx>,
}

impl<'tcx> Visitor<'tcx> for LetSomeVisitor<'_, 'tcx> {
    /// Helper for visit stmt analysis.
    fn visit_stmt(&mut self, stmt: &'tcx Stmt<'tcx>) {
        // Analyze let statements while preserving traversal of every other statement.
        let StmtKind::Let(let_stmt) = stmt.kind else {
            walk_stmt(self, stmt);
            return;
        };

        // Emit a source-preserving replacement only when the rewrite keeps the meaning.
        if let Some(found) = let_some_returns_err(self.cx, self.function_error_ty, let_stmt) {
            emit_span_lint_with_help(
                self.cx,
                LET_SOME_RETURN_ERR,
                let_stmt.span,
                "`Option` mismatch returns `Err` manually",
                "convert the initializer with `.ok_or(...)` or `.ok_or_else(...)` and use `?`",
                suggestion(self.cx, let_stmt, &found),
            );
        }

        walk_stmt(self, stmt);
    }

    /// Helper for visit expr analysis.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if matches!(expr.kind, ExprKind::Closure(_)) {
            return;
        }

        walk_expr(self, expr);
    }
}

/// One matched `let Some(..) = .. else { return Err(..) };` statement.
struct LetSomeReturnsErr<'tcx> {
    /// Initializer that produces the `Option`.
    init: &'tcx Expr<'tcx>,
    /// Pattern inside `Some(..)`.
    pattern: &'tcx Pat<'tcx>,
    /// Error value passed to `Err(..)`.
    error: &'tcx Expr<'tcx>,
}

/// Helper for let some returns err analysis.
fn let_some_returns_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    let_stmt: &'tcx LetStmt<'tcx>,
) -> Option<LetSomeReturnsErr<'tcx>> {
    // Require the initializer, Some pattern, else block, and returned error together.
    let init = let_stmt.init?;
    let pattern = some_option_pattern(cx, let_stmt.pat)?;
    let error = block_only_returns_err(cx, function_error_ty, let_stmt.els?)?;
    Some(LetSomeReturnsErr {
        init,
        pattern,
        error,
    })
}

/// Build the machine-applicable rewrite when it keeps the statement's meaning.
///
/// The rewrite `let PATTERN = (INIT).ok_or_else(|| ERROR)?;` compiles with the
/// same behavior only when the pattern is irrefutable, the initializer is not
/// a place that the method call would move or copy, the error moves into a
/// closure unchanged, and no type annotation or macro is involved.
fn suggestion<'tcx>(
    cx: &LateContext<'tcx>,
    let_stmt: &LetStmt<'tcx>,
    found: &LetSomeReturnsErr<'tcx>,
) -> Option<String> {
    if !is_rewrite_safe(cx, let_stmt, found) {
        return None;
    }

    // Build the replacement from source snippets so `--fix` preserves the user's expressions.
    let source_map = cx.sess().source_map();
    let pattern = source_map.span_to_snippet(found.pattern.span).ok()?;
    let init = source_map.span_to_snippet(found.init.span).ok()?;
    let error = source_map.span_to_snippet(found.error.span).ok()?;
    // Parenthesize the initializer so method-call precedence remains stable.
    Some(format!("let {pattern} = ({init}).ok_or_else(|| {error})?;"))
}

/// Return whether the `ok_or_else` rewrite compiles and keeps the statement's
/// behavior.
fn is_rewrite_safe<'tcx>(
    cx: &LateContext<'tcx>,
    let_stmt: &LetStmt<'tcx>,
    found: &LetSomeReturnsErr<'tcx>,
) -> bool {
    // Keep annotated and macro-generated statements on the help-only path.
    let is_plain_statement = let_stmt.ty.is_none() && !let_stmt.span.from_expansion();
    // `ok_or_else` takes `self`, so the initializer must be an owned `Option`.
    let init_ty = cx.typeck_results().expr_ty(found.init);
    let mut has_ref_binding = false;
    let is_irrefutable_pattern = is_irrefutable(cx, found.pattern, &mut has_ref_binding);
    // A place initializer is moved or copied by the rewrite, unlike `let`-`else`.
    let is_moved_place = is_moved_place(cx, found.init, has_ref_binding);
    // The error moves into a closure whose return type `?` does not constrain.
    let is_error_safe = is_unchanged_in_closure(found.error) && keeps_error_type(cx, found.error);
    is_plain_statement
        && option_ty(cx, init_ty)
        && is_irrefutable_pattern
        && !is_moved_place
        && is_error_safe
}

/// Return whether the rewrite would move or copy a place that `let`-`else`
/// only borrows or binds by reference.
fn is_moved_place(cx: &LateContext<'_>, init: &Expr<'_>, has_ref_binding: bool) -> bool {
    let init_ty = cx.typeck_results().expr_ty(init);
    let is_copy = cx.tcx.type_is_copy_modulo_regions(cx.typing_env(), init_ty);
    is_place_expr(init) && (has_ref_binding || !is_copy)
}

/// Return whether a pattern always matches, recording by-reference bindings.
fn is_irrefutable(cx: &LateContext<'_>, pat: &Pat<'_>, has_ref_binding: &mut bool) -> bool {
    match pat.kind {
        PatKind::Wild => true,
        PatKind::Binding(mode, _, _, subpattern) => {
            *has_ref_binding |= !matches!(mode.0, ByRef::No);
            subpattern.is_none_or(|subpattern| is_irrefutable(cx, subpattern, has_ref_binding))
        }
        PatKind::Ref(inner, ..) => is_irrefutable(cx, inner, has_ref_binding),
        PatKind::Tuple(subpatterns, _) => subpatterns
            .iter()
            .all(|subpattern| is_irrefutable(cx, subpattern, has_ref_binding)),
        // A struct has one variant, so only its fields can make the pattern refutable.
        PatKind::TupleStruct(_, subpatterns, _) => {
            is_struct_pat(cx, pat)
                && subpatterns
                    .iter()
                    .all(|subpattern| is_irrefutable(cx, subpattern, has_ref_binding))
        }
        PatKind::Struct(_, fields, _) => {
            is_struct_pat(cx, pat)
                && fields
                    .iter()
                    .all(|field| is_irrefutable(cx, field.pat, has_ref_binding))
        }
        _ => false,
    }
}

/// Return whether a pattern's resolved type is a struct.
fn is_struct_pat(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    matches!(cx.typeck_results().pat_ty(pat).kind(), ty::Adt(adt, _) if adt.is_struct())
}

/// Return whether an expression denotes a place rather than a fresh value.
const fn is_place_expr(expr: &Expr<'_>) -> bool {
    matches!(
        expr.kind,
        ExprKind::Path(_)
            | ExprKind::Field(..)
            | ExprKind::Index(..)
            | ExprKind::Unary(UnOp::Deref, _)
    )
}

/// Return whether an error expression keeps its meaning inside `|| error`.
///
/// The closure would capture locals, which can conflict with borrows held by
/// the initializer or with later uses. Control flow such as `return`, `?`, or
/// `.await` would apply to the closure instead of the enclosing function.
fn is_unchanged_in_closure(error: &Expr<'_>) -> bool {
    ClosureHazard.visit_expr(error).is_continue()
}

/// Visitor that stops at the first expression a closure would change.
struct ClosureHazard;

impl<'tcx> Visitor<'tcx> for ClosureHazard {
    type Result = ControlFlow<()>;

    /// Stop on locals, closures, and control flow; otherwise keep walking.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> Self::Result {
        match expr.kind {
            ExprKind::Path(QPath::Resolved(None, path)) if matches!(path.res, Res::Local(_)) => {
                ControlFlow::Break(())
            }
            ExprKind::Ret(_)
            | ExprKind::Break(..)
            | ExprKind::Continue(_)
            | ExprKind::Yield(..)
            | ExprKind::Become(_)
            | ExprKind::Closure(_)
            | ExprKind::Match(_, _, MatchSource::TryDesugar(_) | MatchSource::AwaitDesugar) => {
                ControlFlow::Break(())
            }
            _ => walk_expr(self, expr),
        }
    }
}

/// Return whether `|| error` returns exactly the type that `Err(error)` received.
///
/// Inside `Err(..)`, the function's error type is the expected type of `error`.
/// Inside `ok_or_else(|| error)?`, nothing constrains the closure's return type,
/// because `?` converts it through `From`. The rewrite therefore keeps the type
/// only when `error` has no type-changing coercion and its producer fixes the
/// type without inference: `"x".into()` would not compile, and `Box::new(error)`
/// would no longer coerce to `Box<dyn Error>`.
fn keeps_error_type<'tcx>(cx: &LateContext<'tcx>, error: &Expr<'tcx>) -> bool {
    let typeck_results = cx.typeck_results();
    typeck_results.expr_ty_adjusted(error) == typeck_results.expr_ty(error)
        && has_fixed_type(cx, error)
}

/// Return whether an expression's type is fixed by its literal or by its
/// producer's signature.
fn has_fixed_type(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    match expr.kind {
        // Unsuffixed numeric literals take their type from the expected type.
        ExprKind::Lit(literal) => !matches!(
            literal.node,
            LitKind::Int(_, LitIntType::Unsuffixed) | LitKind::Float(_, LitFloatType::Unsuffixed)
        ),
        ExprKind::Path(ref qpath) => match cx.qpath_res(qpath, expr.hir_id) {
            Res::Def(
                kind @ (DefKind::Const { .. }
                | DefKind::AssocConst { .. }
                | DefKind::Static { .. }
                | DefKind::Ctor(_, CtorKind::Const)),
                def_id,
            ) => is_concrete_item_type(cx, kind, def_id),
            _ => false,
        },
        ExprKind::Call(callee, _) => {
            let ExprKind::Path(ref qpath) = callee.kind else {
                return false;
            };
            match cx.qpath_res(qpath, callee.hir_id) {
                Res::Def(kind @ (DefKind::Fn | DefKind::AssocFn | DefKind::Ctor(..)), def_id) => {
                    is_concrete_item_type(cx, kind, def_id)
                }
                _ => false,
            }
        }
        ExprKind::MethodCall(..) => cx
            .typeck_results()
            .type_dependent_def_id(expr.hir_id)
            .is_some_and(|def_id| is_concrete_item_type(cx, DefKind::AssocFn, def_id)),
        // A struct literal fixes its type when the struct has no type or const parameters.
        ExprKind::Struct(..) => match cx.typeck_results().expr_ty(expr).kind() {
            ty::Adt(adt, _) => {
                let counts = cx.tcx.generics_of(adt.did()).own_counts();
                counts.types + counts.consts == 0
            }
            _ => false,
        },
        _ => false,
    }
}

/// Return whether an item's value or return type contains no type or const
/// parameter.
fn is_concrete_item_type(cx: &LateContext<'_>, kind: DefKind, def_id: DefId) -> bool {
    let ty = match kind {
        DefKind::Fn | DefKind::AssocFn | DefKind::Ctor(_, CtorKind::Fn) => cx
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_norm_wip()
            .output()
            .skip_binder(),
        _ => cx
            .tcx
            .type_of(def_id)
            .instantiate_identity()
            .skip_norm_wip(),
    };
    !ty.has_non_region_param()
}

/// Helper for some option pattern analysis.
fn some_option_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &'tcx Pat<'tcx>,
) -> Option<&'tcx Pat<'tcx>> {
    // Require a one-field tuple-struct pattern before checking its resolved Option type.
    let PatKind::TupleStruct(_qpath, subpats, _) = pat.kind else {
        return None;
    };

    if subpats.len() == 1 && option_ty(cx, cx.typeck_results().pat_ty(pat)) {
        subpats.first()
    } else {
        None
    }
}

/// Return whether a type is the standard `Option`.
fn option_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    cx.tcx.is_diagnostic_item(sym::Option, adt.did())
}

/// Helper for block only returns err analysis.
fn block_only_returns_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    block: &'tcx Block<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    match (block.stmts, block.expr) {
        // Match both `return Err(..);` and `return Err(..)` forms inside the else block.
        ([], Some(expr)) => returns_err(cx, function_error_ty, expr),
        ([stmt], None) => stmt_returns_err(cx, function_error_ty, stmt),
        _ => None,
    }
}

/// Helper for stmt returns err analysis.
fn stmt_returns_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    stmt: &'tcx Stmt<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => returns_err(cx, function_error_ty, expr),
        StmtKind::Let(_) | StmtKind::Item(_) => None,
    }
}

/// Return whether the item returns err.
fn returns_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Ret(Some(returned)) = expr.kind else {
        return None;
    };

    err_call(cx, function_error_ty, returned)
}

/// Helper for err call analysis.
fn err_call<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Require a one-argument call before resolving the standard Err constructor.
    let ExprKind::Call(callee, [error]) = expr.kind else {
        return None;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return None;
    };

    // Match both the function's error type and the resolved standard constructor.
    if result_error_ty(cx, cx.typeck_results().expr_ty(expr)) != Some(function_error_ty)
        || !resolved_std_err(cx, qpath, callee.hir_id)
    {
        return None;
    }

    Some(error)
}

/// Return type information for result error.
fn result_error_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    // Extract the error argument only from the standard Result diagnostic item.
    let ty::Adt(adt, args) = ty.kind() else {
        return None;
    };
    if !cx.tcx.is_diagnostic_item(sym::Result, adt.did()) {
        return None;
    }

    Some(args.type_at(1))
}

/// Return whether resolution found std err.
fn resolved_std_err(cx: &LateContext<'_>, qpath: QPath<'_>, hir_id: rustc_hir::HirId) -> bool {
    // Resolve the constructor to its variant so prelude, qualified, and aliased spellings match
    // the `Result::Err` lang item while local functions named `Err` stay excluded.
    let Res::Def(DefKind::Ctor(..), ctor_def_id) = cx.qpath_res(&qpath, hir_id) else {
        return false;
    };
    cx.tcx
        .is_lang_item(cx.tcx.parent(ctor_def_id), LangItem::ResultErr)
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
    suggestion: Option<String>,
) {
    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(message);
            if let Some(suggestion) = suggestion {
                let _ =
                    diag.span_suggestion(span, help, suggestion, Applicability::MachineApplicable);
            } else {
                let _ = diag.help(help);
            }
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
