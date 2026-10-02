#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for a redundant thiserror `#[backtrace]` attribute.
//!
//! The lint proves that a type derives `thiserror::Error` through the
//! implementation the derive generated. thiserror picks the first field whose
//! written type is named `Backtrace` as the backtrace, so `#[backtrace]` on that
//! field changes nothing unless the field is also the error source.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use thiserror as _;

use rustc_errors::Applicability;
use rustc_hir::{FieldDef, Item, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use thiserror_support::{Fix, emit, find_attr, is_field_named, thiserror_shapes};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_REDUNDANT_BACKTRACE_ATTR,
    Warn,
    "`thiserror` backtrace field has a redundant backtrace attribute",
    ThiserrorRedundantBacktraceAttr
}

impl<'tcx> LateLintPass<'tcx> for ThiserrorRedundantBacktraceAttr {
    /// Check every struct and variant of a type that derives `thiserror::Error`.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        for shape in thiserror_shapes(cx, item) {
            // thiserror falls back to the first `Backtrace`-typed field.
            let Some(field) = shape.fields.iter().find(|field| is_backtrace_type(field)) else {
                continue;
            };
            let Some(attribute) = find_attr(cx, field.hir_id, "backtrace") else {
                continue;
            };
            // On a source field, `#[backtrace]` forwards the source's backtrace.
            let is_source = find_attr(cx, field.hir_id, "source").is_some()
                || find_attr(cx, field.hir_id, "from").is_some()
                || is_field_named(cx, field, "source");
            if is_source {
                continue;
            }
            emit(
                cx,
                THISERROR_REDUNDANT_BACKTRACE_ATTR,
                attribute,
                "`Backtrace` fields are detected without `#[backtrace]`",
                "remove the redundant `#[backtrace]` attribute",
                Some(Fix {
                    span: cx
                        .sess()
                        .source_map()
                        .span_extend_while_whitespace(attribute),
                    replacement: String::new(),
                    applicability: Applicability::MachineApplicable,
                }),
            );
        }
    }
}

/// Check thiserror's own rule: the written type path ends in a bare `Backtrace`.
fn is_backtrace_type(field: &FieldDef<'_>) -> bool {
    let TyKind::Path(QPath::Resolved(_, path)) = &field.ty.kind else {
        return false;
    };
    path.segments
        .last()
        .is_some_and(|segment| segment.ident.name.as_str() == "Backtrace" && segment.args.is_none())
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
