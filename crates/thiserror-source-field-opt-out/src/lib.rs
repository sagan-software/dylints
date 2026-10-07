#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for thiserror `source` fields that hold ordinary data.
//!
//! thiserror treats a field named `source` as the error source, so a field of
//! a plain data type fails to type-check with a message that does not mention
//! the field name. Late lints do not run after that failure, so this is an
//! early lint: it finds types whose `Error` implementation thiserror's derive
//! generated and reports a `source` field whose written type is a primitive or
//! string. Renaming the field to `r#source` keeps the same identifier for Rust
//! and makes thiserror treat it as data.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use thiserror as _;

use rustc_ast::{Crate, FieldDef, Ty, TyKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext as _};
use thiserror_support::{
    Fix,
    early::{EarlyShape, for_each_module, has_attr, is_field_named, thiserror_shapes},
    emit,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_SOURCE_FIELD_OPT_OUT,
    Warn,
    "`thiserror` treats a field named source as an error source",
    ThiserrorSourceFieldOptOut
}

impl EarlyLintPass for ThiserrorSourceFieldOptOut {
    /// Check every struct and variant of a type that derives `thiserror::Error`.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        for_each_module(&krate.items, &mut |items| {
            for shape in thiserror_shapes(items) {
                if let Some(field) = data_source_field(cx, &shape) {
                    let span = field.ident.map_or(field.span, |ident| ident.span);
                    emit(
                        cx,
                        THISERROR_SOURCE_FIELD_OPT_OUT,
                        span,
                        "`thiserror` treats this `source` field as `Error::source()`",
                        "rename ordinary data fields to `r#source` to opt out",
                        Some(Fix {
                            span,
                            replacement: "r#source".to_owned(),
                            applicability: Applicability::MachineApplicable,
                        }),
                    );
                }
            }
        });
    }
}

/// Return the implicit `source` field when its written type is plain data.
fn data_source_field<'a>(cx: &EarlyContext<'_>, shape: &EarlyShape<'a>) -> Option<&'a FieldDef> {
    // An explicit `#[source]` or `#[from]` field replaces the implicit `source` field.
    if shape
        .fields
        .iter()
        .any(|field| has_attr(field, "source") || has_attr(field, "from"))
    {
        return None;
    }
    shape.fields.iter().find(|field| {
        is_field_named(cx.sess().source_map(), field, "source") && is_plain_data(&field.ty)
    })
}

/// Check a written type that cannot implement `std::error::Error`.
fn is_plain_data(ty: &Ty) -> bool {
    // Peel references before checking the written terminal type.
    let ty = if let TyKind::Ref(_, inner) = &ty.kind {
        &inner.ty
    } else {
        ty
    };
    // Recognize only a path with no generic arguments as plain data.
    let TyKind::Path(None, path) = &ty.kind else {
        return false;
    };
    let [segment] = path.segments.as_slice() else {
        return false;
    };
    segment.args.is_none() && PLAIN_DATA_TYPES.contains(&segment.ident.as_str())
}

/// Primitive and string type names that never implement `std::error::Error`.
const PLAIN_DATA_TYPES: &[&str] = &[
    "String", "str", "char", "bool", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16",
    "i32", "i64", "i128", "isize", "f32", "f64",
];

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
