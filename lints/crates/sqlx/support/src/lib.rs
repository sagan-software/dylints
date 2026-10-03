#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for `SQLx`-specific private lints.
//!
//! These helpers resolve `SQLx` methods, constructors, and macro expansions with
//! rustc metadata. They preserve source spans and reject local lookalikes so
//! each private lint can focus on one documented `SQLx` contract without duplicating
//! compiler-resolution code across its implementation.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_hir::{
    BinOpKind, Expr, ExprKind,
    def::{CtorOf, DefKind, Res},
};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{ExpnKind, MacroKind, Span, Symbol, def_id::DefId, sym};

use dylint_linting as _;

/// One semantically resolved `SQLx` method call and its original arguments.
///
/// The value is borrowed from HIR and retains the exact method span needed for a
/// diagnostic while preserving the resolved name for the individual lint rule.
#[derive(Clone, Copy, Debug)]
pub struct SqlxMethodCall<'hir> {
    /// User-facing method-name span used for the primary diagnostic.
    pub span: Span,
    /// Resolved method name obtained from type-dependent lookup.
    pub name: Symbol,
    /// Explicit method arguments borrowed from the original HIR expression.
    pub arguments: &'hir [Expr<'hir>],
}

/// One semantically resolved `SQLx` macro call and its source location.
///
/// Macro expansion metadata is retained so callers can diagnose the public `SQLx`
/// spelling even when its implementation delegates through another macro.
#[derive(Clone, Copy, Debug)]
pub struct SqlxMacroCall {
    /// User-facing macro invocation span used for the primary diagnostic.
    pub span: Span,
    /// Resolved macro name obtained from expansion metadata.
    pub name: Symbol,
}

/// Resolve a method call to a `SQLx` owner and one allowed method name.
///
/// The owner is the trait that declares the method or the type whose inherent
/// impl defines it.
///
/// Type-dependent resolution and owner matching exclude extension traits, local
/// methods, and unrelated APIs that happen to share the same source spelling.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = sqlx_support::sqlx_method_call(cx, expr, &["Row"], &["get"]);
/// };
/// ```
pub fn sqlx_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_owners: &[&str],
    expected_methods: &[&str],
) -> Option<SqlxMethodCall<'hir>> {
    // Type-dependent resolution rejects extension traits and user methods with the same spelling.
    let ExprKind::MethodCall(segment, _, arguments, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    let method_name = segment.ident.name;

    let is_expected_method = expected_methods.contains(&method_name.as_str());
    let is_expected_owner =
        definition_owner(cx, def_id).is_some_and(|owner| expected_owners.contains(&owner.as_str()));
    (is_sqlx_def(cx, def_id) && is_expected_method && is_expected_owner).then_some(SqlxMethodCall {
        span: segment.ident.span,
        name: method_name,
        arguments,
    })
}

/// A documented argument shape for a `SQLx` method lint.
///
/// Each variant describes one closed source pattern so the caller can select a
/// precise diagnostic without embedding argument-shape matching in every lint.
#[derive(Clone, Copy, Debug)]
pub enum MethodArgumentViolation {
    /// Every resolved call is discouraged by the selected `SQLx` lint rule.
    Any,
    /// The first argument is a `format!` result, possibly borrowed, producing dynamic SQL.
    FormattedSql,
    /// The first argument is an empty array or an empty `Vec`, possibly borrowed.
    EmptyCollection,
    /// The first argument is statically known unsigned integer zero.
    Zero,
}

/// Match a `SQLx` method call with one argument violation.
///
/// The returned call is available only when the owner, method name, and selected
/// argument pattern all match the resolved `SQLx` API contract.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, owner, expected_method, violation| {
///     let _ = sqlx_support::sqlx_method_argument_violation(cx, expr, owner, expected_method, violation);
/// };
/// ```
pub fn sqlx_method_argument_violation<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    owner: &str,
    expected_method: &str,
    violation: MethodArgumentViolation,
) -> Option<SqlxMethodCall<'hir>> {
    let call = sqlx_method_call(cx, expr, &[owner], &[expected_method])?;
    let argument = call.arguments.first();
    let is_invalid = match violation {
        MethodArgumentViolation::Any => true,
        MethodArgumentViolation::FormattedSql => {
            argument.is_some_and(|argument| is_format_result(cx, argument))
        }
        MethodArgumentViolation::EmptyCollection => {
            argument.is_some_and(|argument| is_empty_collection(cx, argument))
        }
        MethodArgumentViolation::Zero => {
            argument.is_some_and(|argument| is_zero_integer_constant(cx, argument, 0))
        }
    };
    is_invalid.then_some(call)
}

/// Return the expression under any number of `&` and `&mut` borrows.
const fn peel_borrows<'hir>(mut expr: &'hir Expr<'hir>) -> &'hir Expr<'hir> {
    while let ExprKind::AddrOf(_, _, inner) = expr.kind {
        expr = inner;
    }
    expr
}

/// Return true when an expression is the output of the standard `format!` macro.
///
/// The macro is resolved through expansion data, so `format!`, `std::format!`,
/// and `alloc::format!` all match, and a borrow such as `&format!(...)` is peeled.
fn is_format_result(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let expr = peel_borrows(expr);
    // Only the outermost expression of the expansion counts, not a value nested in it.
    if !expr.span.from_expansion() {
        return false;
    }
    let expansion = expr.span.ctxt().outer_expn_data();
    expansion
        .macro_def_id
        .is_some_and(|def_id| cx.tcx.is_diagnostic_item(sym::format_macro, def_id))
}

/// Return true for `[]`, `Vec::new()`, or `vec![]`, possibly borrowed.
fn is_empty_collection(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let expr = peel_borrows(expr);
    // An empty array literal holds no rows.
    if matches!(expr.kind, ExprKind::Array([])) {
        return true;
    }
    // `vec![]` expands to `Vec::new()`, which resolves to the `vec_new` diagnostic item.
    let ExprKind::Call(callee, []) = expr.kind else {
        return false;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.qpath_res(path, callee.hir_id) else {
        return false;
    };
    cx.tcx.get_diagnostic_name(def_id) == Some(Symbol::intern("vec_new"))
}

/// Return whether a bounded, statically evaluable unsigned integer is zero.
fn is_zero_integer_constant(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> bool {
    integer_constant(cx, expr, depth) == Some(0)
}

/// Evaluate unsigned literals, local constants, and bounded pure arithmetic.
fn integer_constant(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<u128> {
    // Bound recursion through arithmetic expressions and nested constants.
    if depth > 16 {
        return None;
    }

    // Require an unsigned expression type before interpreting its syntax.
    let integer_maximum = unsigned_integer_maximum(cx, expr)?;

    // Evaluate only literals, selected arithmetic, and resolved constant paths.
    let value = if let ExprKind::Lit(literal) = expr.kind {
        if let LitKind::Int(value, _) = literal.node {
            Some(value.get())
        } else {
            None
        }
    } else if let ExprKind::Binary(operator, left, right) = expr.kind {
        binary_integer_constant(cx, expr, operator.node, left, right, depth)
    } else if let ExprKind::Path(ref path) = expr.kind {
        path_integer_constant(cx, expr, path, depth)
    } else {
        None
    }?;

    // Reject intermediate values outside their resolved unsigned type.
    (value <= integer_maximum).then_some(value)
}

/// Return the maximum value of an unsigned integer expression's type.
fn unsigned_integer_maximum(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<u128> {
    // Use the expression body's type context, including for constant initializers.
    let expression_type = cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr);
    let ty::Uint(integer_type) = expression_type.kind() else {
        return None;
    };

    // Resolve `usize` from the compilation target and retain fixed integer widths.
    let width = match integer_type {
        ty::UintTy::U8 => 8,
        ty::UintTy::U16 => 16,
        ty::UintTy::U32 => 32,
        ty::UintTy::U64 => 64,
        ty::UintTy::U128 => 128,
        ty::UintTy::Usize => cx.tcx.data_layout.pointer_size().bits(),
    };
    if width == u64::from(u128::BITS) {
        Some(u128::MAX)
    } else {
        Some((1_u128 << u32::try_from(width).ok()?) - 1)
    }
}

/// Evaluate checked arithmetic when both operands have the result's type.
fn binary_integer_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    operator: BinOpKind,
    left: &Expr<'_>,
    right: &Expr<'_>,
    depth: usize,
) -> Option<u128> {
    // Refuse mixed-width operations instead of modeling coercions or casts.
    let result_type = cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr);
    let left_type = cx.tcx.typeck(left.hir_id.owner.def_id).expr_ty(left);
    let right_type = cx.tcx.typeck(right.hir_id.owner.def_id).expr_ty(right);
    if result_type != left_type || result_type != right_type {
        return None;
    }

    // Evaluate operands recursively before applying the selected operation.
    let left = integer_constant(cx, left, depth + 1)?;
    let right = integer_constant(cx, right, depth + 1)?;
    match operator {
        BinOpKind::Add => left.checked_add(right),
        BinOpKind::Sub => left.checked_sub(right),
        BinOpKind::Mul => left.checked_mul(right),
        BinOpKind::Div => left.checked_div(right),
        BinOpKind::Rem => left.checked_rem(right),
        BinOpKind::And
        | BinOpKind::Or
        | BinOpKind::BitXor
        | BinOpKind::BitAnd
        | BinOpKind::BitOr
        | BinOpKind::Shl
        | BinOpKind::Shr
        | BinOpKind::Eq
        | BinOpKind::Lt
        | BinOpKind::Le
        | BinOpKind::Ne
        | BinOpKind::Ge
        | BinOpKind::Gt => None,
    }
}

/// Resolve and evaluate a local non-trait constant initializer.
fn path_integer_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    path: &rustc_hir::QPath<'_>,
    depth: usize,
) -> Option<u128> {
    // Accept constant paths and skip trait defaults that depend on an implementation.
    let Res::Def(DefKind::Const { .. } | DefKind::AssocConst { .. }, definition) = cx
        .tcx
        .typeck(expr.hir_id.owner.def_id)
        .qpath_res(path, expr.hir_id)
    else {
        return None;
    };
    if cx.tcx.def_kind(cx.tcx.parent(definition)) == DefKind::Trait {
        return None;
    }
    let local = definition.as_local()?;
    let initializer = cx.tcx.hir_maybe_body_owned_by(local)?.value;

    // Evaluate the initializer under its own owner and resolved type.
    integer_constant(cx, initializer, depth + 1)
}

/// Declare one `SQLx` method-argument lint.
#[macro_export]
macro_rules! declare_method_argument_lint {
    (
        $lint:ident, $pass:ident, $owner:literal, $method:literal, $violation:ident,
        $description:literal, $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }
        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check the resolved SQLx owner, method, and argument shape.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(call) = $crate::sqlx_method_argument_violation(
                    cx,
                    expr,
                    $owner,
                    $method,
                    $crate::MethodArgumentViolation::$violation,
                ) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    call.span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help($help);
                    }),
                );
            }
        }
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Return the argument when an expression constructs `SQLx`'s `AssertSqlSafe`.
///
/// The constructor is resolved semantically, allowing aliases while excluding
/// local tuple constructors that merely reuse the same type name.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = sqlx_support::assert_sql_safe_argument(cx, expr);
/// };
/// ```
pub fn assert_sql_safe_argument<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
) -> Option<&'hir Expr<'hir>> {
    // Resolve the tuple-struct constructor so aliases still match and local lookalikes do not.
    let ExprKind::Call(callee, [argument]) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let Res::Def(DefKind::Ctor(CtorOf::Struct, _), ctor_def_id) =
        cx.typeck_results().qpath_res(path, callee.hir_id)
    else {
        return None;
    };
    // The constructor's parent is the struct it builds.
    let struct_def_id = cx.tcx.parent(ctor_def_id);
    let is_assert_sql_safe = cx.tcx.item_name(struct_def_id).as_str() == "AssertSqlSafe";
    (is_sqlx_def(cx, struct_def_id) && is_assert_sql_safe).then_some(argument)
}

/// Resolve an unchecked `SQLx` query macro from an expanded expression.
///
/// Expansion traversal follows `SQLx`'s public macro into its implementation until
/// it finds the unchecked definition, preserving the public call-site span.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, span| {
///     let _ = sqlx_support::unchecked_query_macro(cx, span);
/// };
/// ```
pub fn unchecked_query_macro(cx: &LateContext<'_>, span: Span) -> Option<SqlxMacroCall> {
    let mut expansion = span.ctxt().outer_expn_data();

    // Walk outward because SQLx's public macro delegates to its proc-macro implementation.
    loop {
        if matches!(expansion.kind, ExpnKind::Macro(MacroKind::Bang, _))
            && let Some(def_id) = expansion.macro_def_id
            && is_sqlx_def(cx, def_id)
            && is_unchecked_query_macro(cx.tcx.item_name(def_id).as_str())
        {
            return Some(SqlxMacroCall {
                span: expansion.call_site,
                name: cx.tcx.item_name(def_id),
            });
        }

        if !expansion.call_site.from_expansion() {
            return None;
        }
        // Continue with the caller expansion until the public SQLx macro is found.
        expansion = expansion.call_site.ctxt().outer_expn_data();
    }
}

/// Return the checked counterpart for an unchecked `SQLx` query macro.
///
/// The conversion removes only `SQLx`'s `_unchecked` suffix and returns `None` for
/// names that do not use that exact macro naming convention.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |name| {
///     let _ = sqlx_support::checked_query_macro_name(name);
/// };
/// ```
pub fn checked_query_macro_name(name: &str) -> Option<&str> {
    name.strip_suffix("_unchecked")
}

/// Prove that a definition comes from `SQLx`'s facade or core crate.
fn is_sqlx_def(cx: &LateContext<'_>, def_id: DefId) -> bool {
    matches!(
        cx.tcx.crate_name(def_id.krate).as_str(),
        "sqlx" | "sqlx_core"
    )
}

/// Return the trait or type that owns an associated function.
///
/// A trait method is owned by its trait. An inherent method is owned by the
/// type its impl block names, and a trait impl method by the implemented trait.
fn definition_owner(cx: &LateContext<'_>, def_id: DefId) -> Option<Symbol> {
    let parent = cx.tcx.parent(def_id);
    if matches!(cx.tcx.def_kind(parent), DefKind::Trait) {
        return Some(cx.tcx.item_name(parent));
    }
    if matches!(cx.tcx.def_kind(parent), DefKind::Impl { of_trait: true }) {
        return Some(cx.tcx.item_name(cx.tcx.impl_trait_id(parent)));
    }
    if !matches!(cx.tcx.def_kind(parent), DefKind::Impl { of_trait: false }) {
        return None;
    }
    let self_ty = cx
        .tcx
        .type_of(parent)
        .instantiate_identity()
        .skip_norm_wip();
    let ty::Adt(adt, _) = self_ty.kind() else {
        return None;
    };
    Some(cx.tcx.item_name(adt.did()))
}

/// Return whether the macro skips `SQLx`'s input or output type checking.
fn is_unchecked_query_macro(name: &str) -> bool {
    matches!(
        name,
        "query_unchecked"
            | "query_as_unchecked"
            | "query_file_unchecked"
            | "query_file_as_unchecked"
            | "query_scalar_unchecked"
            | "query_file_scalar_unchecked"
    )
}
