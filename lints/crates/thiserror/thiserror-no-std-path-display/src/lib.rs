#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Path formatting with thiserror's `std` feature disabled.
//!
//! Without the `std` feature, thiserror's derived `Display` cannot format a
//! `Path` or `PathBuf` field, and type checking fails with a message that does
//! not name the feature. Late lints do not run after that failure, so this is
//! an early lint: it finds types whose `Error` implementation thiserror's
//! derive generated, then reads the compiled package's `Cargo.toml` to decide
//! whether the `std` feature is disabled.

extern crate rustc_ast;
extern crate rustc_span;

mod manifest;

#[cfg(test)]
use thiserror as _;

use rustc_ast::{Crate, FieldDef, TyKind};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext as _};
use rustc_span::Span;
use thiserror_support::{
    early::{error_attrs, for_each_module, thiserror_shapes},
    emit,
    format::Argument,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_NO_STD_PATH_DISPLAY,
    Warn,
    "`thiserror` formats a path field while its std feature is disabled",
    ThiserrorNoStdPathDisplay
}

impl EarlyLintPass for ThiserrorNoStdPathDisplay {
    /// Collect path captures, then report them when the manifest disables `std`.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let source_map = cx.sess().source_map();
        let mut captures = Vec::new();
        for_each_module(&krate.items, &mut |items| {
            for shape in thiserror_shapes(items) {
                captures.extend(
                    error_attrs(source_map, shape.attrs)
                        .into_iter()
                        .filter(|(_, attr)| {
                            attr.format().is_some_and(|format| {
                                format.placeholders().iter().any(|placeholder| {
                                    let Argument::Named(name) = &placeholder.argument else {
                                        return false;
                                    };
                                    // Only a bare `{field}` uses path display in thiserror.
                                    placeholder.spec.is_empty()
                                        && shape.fields.iter().any(|field| {
                                            field.ident.is_some_and(|ident| ident.as_str() == name)
                                                && is_path_type(field)
                                        })
                                })
                            })
                        })
                        .map(|(span, _)| span),
                );
            }
        });
        // Read the manifest only when a capture needs the answer.
        if captures.is_empty() || !manifest::is_thiserror_std_disabled() {
            return;
        }
        for span in captures {
            report(cx, span);
        }
    }
}

/// Report one path capture.
fn report(cx: &EarlyContext<'_>, span: Span) {
    emit(
        cx,
        THISERROR_NO_STD_PATH_DISPLAY,
        span,
        "`thiserror` path capture needs its `std` feature",
        "enable thiserror's `std` feature, or format the field with `.display()`",
        None,
    );
}

/// Check for a field written as `Path` or `PathBuf`, possibly behind references.
fn is_path_type(field: &FieldDef) -> bool {
    // Peel references before checking the written terminal type name.
    let mut ty = &field.ty;
    while let TyKind::Ref(_, inner) = &ty.kind {
        ty = &inner.ty;
    }
    let TyKind::Path(None, path) = &ty.kind else {
        return false;
    };
    // Generic path arguments do not prove the exact standard path type.
    path.segments.last().is_some_and(|segment| {
        matches!(segment.ident.as_str(), "Path" | "PathBuf") && segment.args.is_none()
    })
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
