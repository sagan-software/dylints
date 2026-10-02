#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    reason = "the lint intentionally ignores diagnostic builders and slices verified ASCII punctuation"
)]

//! A lint to check for expensive inherent `as_*` methods.
//!
//! It examines source-authored methods whose names promise a cheap view or
//! conversion, then reports recognized allocation, parsing, cloning, and other
//! work in their bodies. The analysis uses rustc resolution so unrelated trait
//! methods and generated code remain outside the lint's intended scope.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{FnDecl, FnRetTy, ImplItem, ImplItemImplKind, ImplItemKind, QPath, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

/// `SOURCE_WORK_MARKERS` configuration used by this lint.
const SOURCE_WORK_MARKERS: &[&str] = &[
    ".parse(",
    ".parse::<",
    ".to_string(",
    ".to_owned(",
    ".clone(",
    "String::from(",
    "Vec::from(",
    "format!(",
    "format !",
    "decode(",
    ".decode(",
    "::decode(",
    "base64::",
];

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub EXPENSIVE_AS_METHOD,
    Warn,
    "inherent `as_*` method does non-accessor work",
    ExpensiveAsMethod
}

impl<'tcx> LateLintPass<'tcx> for ExpensiveAsMethod {
    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the naming policy to inherent methods owned by the type.
        let ImplItemImplKind::Inherent { .. } = item.impl_kind else {
            return;
        };
        let ImplItemKind::Fn(sig, _) = item.kind else {
            return;
        };

        // Combine the `as_*` vocabulary check with evidence of non-accessor work.
        let name = item.ident.name.to_ident_string();
        if !name.starts_with("as_") || !does_non_accessor_work(cx, item, sig.decl) {
            return;
        }

        // Recommend a method prefix that makes the observed work visible to callers.
        emit_span_lint_with_help(
            cx,
            EXPENSIVE_AS_METHOD,
            item.span,
            format!("inherent method `{name}` does work that should not hide behind `as_*`"),
            "reserve `as_*` for cheap borrowed accessors; use `to_*`, `try_*`, or a domain-specific verb for work",
        );
    }
}

/// Helper for does non accessor work analysis.
fn does_non_accessor_work(cx: &LateContext<'_>, item: &ImplItem<'_>, decl: &FnDecl<'_>) -> bool {
    if fallible_return(decl) {
        return true;
    }

    let Ok(source) = cx.sess().source_map().span_to_snippet(item.span) else {
        return false;
    };

    // Source scanning catches obvious macros and method calls without pretending to prove cost.
    SOURCE_WORK_MARKERS
        .iter()
        .any(|marker| source.contains(marker))
        || contains_question_mark_operator(&source)
}

/// Return whether the source contains question mark operator.
fn contains_question_mark_operator(source: &str) -> bool {
    source.match_indices('?').any(|(index, _)| {
        // `?Sized` and similar bounds are type-level syntax, not fallible control flow.
        source[index + '?'.len_utf8()..]
            .chars()
            .find(|character| !character.is_whitespace())
            .is_none_or(|character| !is_identifier_start(character))
    })
}

/// Return whether identifier start.
const fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

/// Helper for fallible return analysis.
fn fallible_return(decl: &FnDecl<'_>) -> bool {
    let FnRetTy::Return(ty) = decl.output else {
        return false;
    };

    path_ty_name(ty).is_some_and(|name| matches!(name.as_str(), "Option" | "Result"))
}

/// Return the path ty name.
fn path_ty_name(ty: &Ty<'_>) -> Option<String> {
    let TyKind::Path(QPath::Resolved(_, path)) = ty.kind else {
        return None;
    };

    path.segments
        .last()
        .map(|segment| segment.ident.name.to_ident_string())
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

    // Use rustc's native diagnostic decorator to match the rest of this lint suite.
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

/// Helper for detects obvious source work analysis.
#[test]
fn detects_obvious_source_work() {
    let source = "fn as_string(&self) -> String { self.raw.clone() }";

    assert!(
        SOURCE_WORK_MARKERS
            .iter()
            .any(|marker| source.contains(marker))
    );
}

/// Helper for distinguishes question mark operator from bounds analysis.
#[test]
fn distinguishes_question_mark_operator_from_bounds() {
    assert!(contains_question_mark_operator(
        "fn as_value(&self) -> Result<Value, Error> { self.load()? }"
    ));
    assert!(!contains_question_mark_operator(
        "fn as_value<T: ?Sized>(&self) -> &T { todo!() }"
    ));
}
