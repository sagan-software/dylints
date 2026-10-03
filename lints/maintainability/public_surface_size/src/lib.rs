#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to bound reachable public names per module.
//!
//! It counts names that rustc marks reachable from each source module and
//! reports a module whose public surface exceeds the configured limit. Hidden
//! documentation roots and internal support or fixture crates are excluded so
//! implementation scaffolding does not distort the public API measurement.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use std::collections::HashSet;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, HirId, ItemKind, Mod, Node, PrimTy,
    attrs::AttributeKind,
    def::{DefKind, Res},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::def_id::{CRATE_DEF_ID, DefId};

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
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || cx.tcx.is_doc_hidden(CRATE_DEF_ID)
        {
            return;
        }
        if module_id != CRATE_DEF_ID && !cx.effective_visibilities.is_reachable(module_id) {
            return;
        }

        // Resolved module children retain each exported name, including glob re-exports, while
        // visibility filters private ancestry. Impl blocks and global assembly add no names.
        let public_names = cx
            .tcx
            .module_children_local(module_id)
            .iter()
            .filter_map(|child| {
                if !child.vis.is_public() || is_test_marker(cx, &child.res) {
                    return None;
                }
                normalized_definition(cx, &child.res)
                    .map(|definition| (child.ident.name, definition))
            })
            .collect::<HashSet<_>>()
            .len();
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

/// Identifies a resolved public declaration, including primitive type aliases.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum PublicDefinition {
    /// A definition with a compiler-assigned identity.
    Def(DefId),
    /// A primitive type identified by its language-level type.
    Primitive(PrimTy),
}

/// Normalize a constructor to its parent so one exported tuple or variant
/// name counts once.
fn normalized_definition<Id>(cx: &LateContext<'_>, res: &Res<Id>) -> Option<PublicDefinition> {
    match res {
        Res::Def(DefKind::Ctor(..), def_id) => Some(PublicDefinition::Def(cx.tcx.parent(*def_id))),
        Res::Def(_, def_id) => Some(PublicDefinition::Def(*def_id)),
        Res::PrimTy(prim_ty) => Some(PublicDefinition::Primitive(*prim_ty)),
        Res::SelfTyParam { .. }
        | Res::SelfTyAlias { .. }
        | Res::SelfCtor(_)
        | Res::Local(_)
        | Res::ToolMod
        | Res::OpenMod(_)
        | Res::NonMacroAttr(_)
        | Res::Err => None,
    }
}

/// Return whether an item is the compiler-generated marker for a `#[test]` function.
fn is_test_marker<Id>(cx: &LateContext<'_>, res: &Res<Id>) -> bool {
    // Test markers resolve to generated constants; inspect their typed attributes before counting.
    let Some(local_def_id) = res.opt_def_id().and_then(DefId::as_local) else {
        return false;
    };
    let Node::Item(item) = cx.tcx.hir_node_by_def_id(local_def_id) else {
        return false;
    };
    matches!(item.kind, ItemKind::Const(..))
        && cx
            .tcx
            .hir_attrs(item.hir_id())
            .iter()
            .any(|attr| matches!(attr, Attribute::Parsed(AttributeKind::RustcTestMarker(_))))
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
