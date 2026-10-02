#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only string HTTP method fields are relevant and diagnostics are configured in place"
)]

//! A lint to check for HTTP methods stored as strings.
//!
//! It pairs HTTP method vocabulary on fields, parameters, and return values
//! with the resolved `String` or `&str` type. The generic names `method` and
//! `verb` only count when the crate links an HTTP library, so command-line and
//! reflection code that names a method is not reported.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, FieldDef, FnDecl, FnRetTy, LangItem, Pat, PatKind, TraitFn, TraitItem, TraitItemKind,
    intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::{Ident, Span, Symbol, def_id::LocalDefId};

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub HTTP_METHOD_STRING,
    Warn,
    "HTTP method stored as a string",
    HttpMethodString,
    HttpMethodString::default()
}

/// Lint pass that remembers whether the crate links an HTTP library.
#[derive(Default)]
struct HttpMethodString {
    /// Whether the generic names `method` and `verb` denote HTTP methods in this crate.
    has_http_crate: bool,
}

/// Crates whose presence makes the generic names `method` and `verb` HTTP vocabulary.
const HTTP_CRATES: &[&str] = &[
    "actix_web",
    "axum",
    "http",
    "hyper",
    "isahc",
    "poem",
    "reqwest",
    "rocket",
    "surf",
    "tide",
    "ureq",
    "warp",
];

impl<'tcx> LateLintPass<'tcx> for HttpMethodString {
    /// Record whether an HTTP library is a direct dependency or a loaded crate.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        // Record direct dependencies before inspecting names and field types.
        self.has_http_crate = has_http_extern_dependency(cx);
        if !self.has_http_crate {
            self.has_http_crate = has_loaded_http_crate(cx);
        }
    }

    /// Check one named field.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        let field_ty = cx
            .tcx
            .type_of(field.def_id)
            .instantiate_identity()
            .skip_norm_wip();
        self.check_named_ty(cx, "field", field.ident.name, field.ty.span, field_ty);
    }

    /// Check the parameters and return value of a named function or method.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        // Closures have no API name, and trait impls inherit their signature from the trait.
        let (FnKind::ItemFn(ident, ..) | FnKind::Method(ident, _)) = kind else {
            return;
        };
        if implements_trait_item(cx, local_def_id) {
            return;
        }

        // Body parameters carry source names, while the declaration carries the source types.
        for (ty, param) in decl.inputs.iter().zip(body.params) {
            if let Some(name) = binding_name(param.pat) {
                let param_ty = cx.typeck_results().node_type(param.hir_id);
                self.check_named_ty(cx, "parameter", name, ty.span, param_ty);
            }
        }
        self.check_return(cx, decl, local_def_id, ident);
    }

    /// Check a required trait method, whose parameter names live on its declaration.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        let TraitItemKind::Fn(sig, TraitFn::Required(arg_names)) = item.kind else {
            return;
        };
        let local_def_id = item.owner_id.def_id;

        // Required methods have no body type table, so read the resolved signature instead.
        let sig_inputs = cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_binder()
            .inputs();
        // Check named parameters before the return value so diagnostics follow source order.
        sig.decl
            .inputs
            .iter()
            .zip(arg_names)
            .zip(sig_inputs)
            .filter_map(|((ty, maybe_ident), param_ty)| {
                maybe_ident.map(|ident| (ident, ty.span, *param_ty))
            })
            .for_each(|(ident, span, param_ty)| {
                self.check_named_ty(cx, "parameter", ident.name, span, param_ty);
            });
        self.check_return(cx, sig.decl, local_def_id, item.ident);
    }
}

impl HttpMethodString {
    /// Check an explicit return type against the function name.
    fn check_return(
        &self,
        cx: &LateContext<'_>,
        decl: &FnDecl<'_>,
        local_def_id: LocalDefId,
        name: Ident,
    ) {
        let FnRetTy::Return(output) = decl.output else {
            return;
        };
        let output_ty = cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip()
            .output()
            .skip_binder();
        self.check_named_ty(cx, "return value", name.name, output.span, output_ty);
    }

    /// Report a string type whose binding name denotes an HTTP method.
    fn check_named_ty(
        &self,
        cx: &LateContext<'_>,
        label: &'static str,
        name: Symbol,
        span: Span,
        ty: ty::Ty<'_>,
    ) {
        if !self.http_method_name(name.as_str()) {
            return;
        }
        let Some(string_ty) = string_ty(cx, ty) else {
            return;
        };

        // Name the affected API position and recommend a typed method at that boundary.
        cx.emit_span_lint(
            HTTP_METHOD_STRING,
            span,
            DiagDecorator(|diag| {
                let _ = diag
                    .primary_message(format!("HTTP method {label} `{name}` uses `{string_ty}`"));
                let _ = diag.help("use `http::Method` or `reqwest::Method`");
            }),
        );
    }

    /// Return whether a binding name denotes an HTTP method in this crate.
    fn http_method_name(&self, name: &str) -> bool {
        // Match exact vocabulary so names such as `payment_method` and `method_name` do not warn.
        match name {
            "http_method" | "request_method" => true,
            "method" | "verb" => self.has_http_crate,
            _ => false,
        }
    }
}

/// Return whether a direct dependency uses HTTP method vocabulary.
fn has_http_extern_dependency(cx: &LateContext<'_>) -> bool {
    cx.sess()
        .opts
        .externs
        .iter()
        .any(|(name, _)| HTTP_CRATES.contains(&name.as_str()))
}

/// Return whether a loaded crate uses HTTP method vocabulary.
fn has_loaded_http_crate(cx: &LateContext<'_>) -> bool {
    cx.tcx
        .crates(())
        .iter()
        .any(|krate| HTTP_CRATES.contains(&cx.tcx.crate_name(*krate).as_str()))
}

/// Return whether a function implements an item of a trait impl.
fn implements_trait_item(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    cx.tcx
        .impl_of_assoc(local_def_id.to_def_id())
        .is_some_and(|impl_def_id| cx.tcx.impl_opt_trait_id(impl_def_id).is_some())
}

/// Return the name of a simple parameter binding.
const fn binding_name(pat: &Pat<'_>) -> Option<Symbol> {
    // Only simple bindings have a single API name; destructured patterns stay out of scope.
    let PatKind::Binding(_mode, _hir_id, ident, None) = pat.kind else {
        return None;
    };

    Some(ident.name)
}

/// Return the written form of a resolved `String` or `&str` type.
fn string_ty(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Adt(adt, _) if cx.tcx.is_lang_item(adt.did(), LangItem::String) => Some("String"),
        ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Str) => Some("&str"),
        _ => None,
    }
}

/// Run the UI fixture suite.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
