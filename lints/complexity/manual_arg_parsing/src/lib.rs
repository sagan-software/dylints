#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for manual command line argument parsing.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use clap as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Expr, ExprKind, HirId, Node, QPath,
    def::{DefKind, Res},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{Span, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_ARG_PARSING,
    Warn,
    "manual command line argument parsing",
    ManualArgParsing
}

impl<'tcx> LateLintPass<'tcx> for ManualArgParsing {
    /// Check expr for this lint.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Restrict the check to resolved function calls before testing API identity.
        let ExprKind::Call(callee, _) = expr.kind else {
            return;
        };

        if !resolved_env_args_call(cx, callee) || is_forwarded_or_counted(cx, expr) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            MANUAL_ARG_PARSING,
            callee.span,
            "command line arguments are parsed manually",
            "derive `clap::Parser` for the CLI shape, or use `clap` parsing instead",
        );
    }
}

/// Maximum number of standard iterator adapters allowed in a forwarded argument chain.
const MAX_ARGUMENT_ADAPTERS: usize = 4;

/// Maximum number of parent expressions inspected for one argument use.
const MAX_USE_CHAIN_STEPS: usize = 12;

/// Relationship between an argument call and one of its parent expressions.
#[derive(Clone, Copy, Debug)]
enum ArgumentUse {
    /// The parent only adds a compiler temporary wrapper.
    Temporary,
    /// The parent counts the argument iterator.
    Count,
    /// The parent applies one supported iterator adapter.
    Adapter,
    /// The parent passes the iterator to `Command::args`.
    Forward,
    /// The parent has another use that remains subject to the lint.
    Other,
}

/// Structural kind of one expression in the argument use chain.
#[derive(Clone, Copy, Debug)]
enum ParentExpression<'hir> {
    /// A compiler-generated temporary wrapper around one expression.
    Temporary(&'hir Expr<'hir>),
    /// A resolved method call with its receiver and explicit arguments.
    MethodCall(&'hir Expr<'hir>, &'hir [Expr<'hir>]),
    /// Any expression shape outside the supported use chain.
    Other,
}

/// Return whether a standard argument iterator reaches forwarding or counting directly.
fn is_forwarded_or_counted(cx: &LateContext<'_>, args_call: &Expr<'_>) -> bool {
    let mut current = args_call;
    let mut adapters = 0;
    let mut parents = cx.tcx.hir_parent_iter(args_call.hir_id);

    // Follow only direct expression uses so stored or mixed argument streams remain visible.
    for _ in 0..MAX_USE_CHAIN_STEPS {
        let Some((_, Node::Expr(parent))) = parents.next() else {
            return false;
        };

        match argument_use(cx, parent, current, adapters) {
            ArgumentUse::Temporary => current = parent,
            ArgumentUse::Count | ArgumentUse::Forward => return true,
            ArgumentUse::Adapter => {
                adapters += 1;
                current = parent;
            }
            ArgumentUse::Other => return false,
        }
    }

    false
}

/// Classify one parent using direct child identity and resolved method definitions.
fn argument_use(
    cx: &LateContext<'_>,
    parent: &Expr<'_>,
    current: &Expr<'_>,
    adapters: usize,
) -> ArgumentUse {
    match parent_expression(parent) {
        ParentExpression::Temporary(inner) if inner.hir_id == current.hir_id => {
            ArgumentUse::Temporary
        }
        ParentExpression::MethodCall(receiver, arguments) => {
            if receiver.hir_id == current.hir_id && is_standard_iterator_method(cx, parent, "count")
            {
                ArgumentUse::Count
            } else if receiver.hir_id == current.hir_id
                && arguments.len() == 1
                && adapters < MAX_ARGUMENT_ADAPTERS
                && is_supported_iterator_adapter(cx, parent)
            {
                ArgumentUse::Adapter
            } else if arguments
                .iter()
                .any(|argument| argument.hir_id == current.hir_id)
                && is_standard_command_args(cx, parent)
            {
                ArgumentUse::Forward
            } else {
                ArgumentUse::Other
            }
        }
        ParentExpression::Temporary(_) | ParentExpression::Other => ArgumentUse::Other,
    }
}

/// Classify expression structure without treating unknown parents as safe.
const fn parent_expression<'hir>(parent: &'hir Expr<'hir>) -> ParentExpression<'hir> {
    match parent.kind {
        ExprKind::DropTemps(inner) => ParentExpression::Temporary(inner),
        ExprKind::MethodCall(_, receiver, arguments, _) => {
            ParentExpression::MethodCall(receiver, arguments)
        }
        ExprKind::ConstBlock(_)
        | ExprKind::Array(_)
        | ExprKind::Call(..)
        | ExprKind::Use(..)
        | ExprKind::Tup(_)
        | ExprKind::Binary(..)
        | ExprKind::Unary(..)
        | ExprKind::Lit(_)
        | ExprKind::Cast(..)
        | ExprKind::Type(..)
        | ExprKind::Let(..)
        | ExprKind::If(..)
        | ExprKind::Loop(..)
        | ExprKind::Match(..)
        | ExprKind::Closure(..)
        | ExprKind::Block(..)
        | ExprKind::Assign(..)
        | ExprKind::AssignOp(..)
        | ExprKind::Field(..)
        | ExprKind::Index(..)
        | ExprKind::Path(..)
        | ExprKind::AddrOf(..)
        | ExprKind::Break(..)
        | ExprKind::Continue(..)
        | ExprKind::Ret(..)
        | ExprKind::Become(..)
        | ExprKind::InlineAsm(..)
        | ExprKind::OffsetOf(..)
        | ExprKind::Struct(..)
        | ExprKind::Repeat(..)
        | ExprKind::Yield(..)
        | ExprKind::UnsafeBinderCast(..)
        | ExprKind::Err(..) => ParentExpression::Other,
    }
}

/// Return whether an expression resolves to `std::process::Command::args`.
fn is_standard_command_args(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    cx.typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .is_some_and(|method| {
            let path = cx.get_def_path(method);
            path.iter()
                .map(rustc_span::Symbol::as_str)
                .eq(["std", "process", "Command", "args"])
        })
}

/// Return whether an expression resolves to a standard Iterator method by name.
fn is_standard_iterator_method(cx: &LateContext<'_>, expr: &Expr<'_>, name: &str) -> bool {
    let Some(method) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    let Some(associated_item) = cx.tcx.opt_associated_item(method) else {
        return false;
    };
    let Ok(trait_item) = associated_item.trait_item_or_self() else {
        return false;
    };
    let Some(trait_id) = cx.tcx.trait_of_assoc(trait_item) else {
        return false;
    };

    cx.tcx.item_name(method).as_str() == name && cx.tcx.is_diagnostic_item(sym::Iterator, trait_id)
}

/// Return whether an expression resolves to a bounded standard Iterator adapter.
fn is_supported_iterator_adapter(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    is_standard_iterator_method(cx, expr, "skip") || is_standard_iterator_method(cx, expr, "take")
}

/// Return whether resolution found env args call.
fn resolved_env_args_call(cx: &LateContext<'_>, callee: &Expr<'_>) -> bool {
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };

    // Use rustc resolution instead of name-only matching so local `args()` helpers are ignored.
    resolves_to_std_env_args(cx, qpath, callee.hir_id)
}

/// Helper for resolves to std env args analysis.
fn resolves_to_std_env_args(cx: &LateContext<'_>, qpath: QPath<'_>, hir_id: HirId) -> bool {
    let Res::Def(DefKind::Fn, def_id) = cx.qpath_res(&qpath, hir_id) else {
        return false;
    };

    // `std::env::args` and `args_os` have no diagnostic items, so compare the resolved
    // definition path segment by segment. This catches fully qualified calls, module aliases,
    // and imported functions while rejecting unrelated local functions named `args`.
    let def_path = cx.get_def_path(def_id);
    let [krate, module, function] = def_path.as_slice() else {
        return false;
    };
    *krate == sym::std
        && *module == sym::env
        && (*function == sym::args || function.as_str() == "args_os")
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
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
