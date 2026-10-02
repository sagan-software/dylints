#![feature(rustc_private)]

//! A lint to bound reachable public names per module.
//!
//! It counts names that rustc marks reachable from each source module and
//! reports a module whose public surface exceeds the configured limit. Hidden
//! documentation roots and internal support or fixture crates are excluded so
//! implementation scaffolding does not distort the public API measurement.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{HirId, ItemKind, Mod};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::def_id::CRATE_DEF_ID;
use std::{ffi::OsStr, path::Component};

/// Largest accepted number of reachable public names exported by one module.
const PUBLIC_NAME_LIMIT: usize = 25;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub PUBLIC_SURFACE_SIZE,
    Warn,
    "module exceeds the reachable public-name limit",
    PublicSurfaceSize
}

impl<'tcx> LateLintPass<'tcx> for PublicSurfaceSize {
    /// Check one reachable module's direct namespace surface.
    fn check_mod(&mut self, cx: &LateContext<'tcx>, module: &'tcx Mod<'tcx>, hir_id: HirId) {
        let module_id = hir_id.owner.def_id;
        if crate_is_internal_support(cx) || crate_is_doc_hidden(cx) {
            return;
        }
        if module_id != CRATE_DEF_ID && !cx.effective_visibilities.is_reachable(module_id) {
            return;
        }

        // HIR use leaves retain each re-exported name, while effective visibility filters private
        // ancestry. Impl blocks and global assembly do not introduce caller-visible names.
        let public_names = module
            .item_ids
            .iter()
            .filter(|item_id| {
                let item = cx.tcx.hir_item(**item_id);
                !matches!(item.kind, ItemKind::Impl(..) | ItemKind::GlobalAsm { .. })
                    && cx.effective_visibilities.is_reachable(item.owner_id.def_id)
            })
            .count();
        if public_names <= PUBLIC_NAME_LIMIT {
            return;
        }

        cx.emit_span_lint(
            PUBLIC_SURFACE_SIZE,
            module.spans.inner_span,
            DiagDecorator(move |diagnostic| {
                let _configured = diagnostic
                    .primary_message(format!(
                        "module exports {public_names} reachable public names, which exceeds {PUBLIC_NAME_LIMIT}"
                    ))
                    .help("expose a focused facade and keep implementation modules private");
            }),
        );
    }
}

/// Return whether the crate is an internal cross-crate helper or UI fixture.
fn crate_is_internal_support(cx: &LateContext<'_>) -> bool {
    // Exclude support and fixture crate names before inspecting their source path.
    let crate_symbol = cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE);
    let crate_name = crate_symbol.as_str();
    if crate_name.ends_with("_support") || crate_name.ends_with("_fixture") {
        return true;
    }

    let Some(path) = cx
        .sess()
        .local_crate_source_file()
        .and_then(rustc_span::RealFileName::into_local_path)
    else {
        return false;
    };
    // Treat nested support and fixture directories as implementation-only crates.
    path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if name == OsStr::new("support") || name == OsStr::new("fixture")
        )
    })
}

/// Return whether the crate explicitly hides its support-only API from documentation.
fn crate_is_doc_hidden(cx: &LateContext<'_>) -> bool {
    cx.tcx
        .hir_attrs(cx.tcx.local_def_id_to_hir_id(CRATE_DEF_ID))
        .iter()
        .any(|attr| {
            attr.has_name(rustc_span::symbol::sym::doc)
                && attr.meta_item_list().is_some_and(|items| {
                    items.iter().any(|item| match item {
                        rustc_ast::MetaItemInner::MetaItem(meta) => {
                            meta.path.segments.len() == 1
                                && meta.path.segments.first().is_some_and(|segment| {
                                    segment.ident.name == rustc_span::symbol::sym::hidden
                                })
                                && matches!(meta.kind, rustc_ast::MetaItemKind::Word)
                        }
                        rustc_ast::MetaItemInner::Lit(_) => false,
                    })
                })
        })
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
