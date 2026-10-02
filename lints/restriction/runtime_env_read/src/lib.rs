#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for runtime environment reads outside configuration boundaries.
//!
//! It resolves calls that read process environment state, walks their enclosing
//! function context, and reports reads that bypass an explicit configuration
//! boundary. Reads inside closures, including `LazyLock` initializers, use the
//! context of the item that owns the closure.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use std::path::Path;

use rustc_ast::attr::data_structures::CfgEntry;
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, Body, Expr, ExprKind, ItemKind,
    attrs::AttributeKind,
    def::{DefKind, Res},
    intravisit::{self, FnKind, Visitor},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{
    Span,
    def_id::{DefId, LocalDefId},
    sym,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub RUNTIME_ENV_READ,
    Warn,
    "runtime environment read outside configuration/bootstrap code",
    RuntimeEnvRead
}

impl<'tcx> LateLintPass<'tcx> for RuntimeEnvRead {
    /// Check the reads written directly in one function or closure body.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        // A closure body is visited on its own, so judge it by the item that owns it.
        let owner = cx.tcx.typeck_root_def_id_local(local_def_id);
        if allowed_context(cx, owner) {
            return;
        }

        EnvReadFinder { cx }.visit_expr(body.value);
    }
}

/// Visitor that reports environment reads in one body without entering nested closures.
struct EnvReadFinder<'cx, 'tcx> {
    /// Lint context used to resolve calls and emit diagnostics.
    cx: &'cx LateContext<'tcx>,
}

impl<'tcx> Visitor<'tcx> for EnvReadFinder<'_, 'tcx> {
    /// Report a resolved environment read, then continue into child expressions.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(callee, _) = expr.kind
            && let Some(name) = env_read_name(self.cx, callee)
        {
            emit_env_read_lint(self.cx, callee.span, name);
        }

        // Continue traversal so nested environment reads also receive diagnostics.
        intravisit::walk_expr(self, expr);
    }
}

/// Return the function name when a callee resolves to `std::env::var` or `std::env::var_os`.
fn env_read_name(cx: &LateContext<'_>, callee: &Expr<'_>) -> Option<&'static str> {
    let ExprKind::Path(qpath) = callee.kind else {
        return None;
    };
    let Res::Def(DefKind::Fn, def_id) = cx.qpath_res(&qpath, callee.hir_id) else {
        return None;
    };

    // The definition path names the defining crate and module, so renames and aliases match.
    match cx.get_def_path(def_id).as_slice() {
        [krate, module, name] if *krate == sym::std && *module == sym::env => match name.as_str() {
            "var" => Some("var"),
            "var_os" => Some("var_os"),
            _ => None,
        },
        _ => None,
    }
}

/// Return whether an item is a configuration, entry point, build-script, or test context.
fn allowed_context(cx: &LateContext<'_>, owner: LocalDefId) -> bool {
    // Allow entrypoints and named boundaries before scanning the body for environment reads.
    cx.tcx.opt_item_name(owner.to_def_id()) == Some(sym::main)
        || in_test_code(cx, owner)
        || allowed_def_path(cx, owner)
        || allowed_source_path(cx, cx.tcx.def_span(owner))
}

/// Return whether an item or one of its ancestors is test-only code.
fn in_test_code(cx: &LateContext<'_>, owner: LocalDefId) -> bool {
    // Walk outward so helpers inside a `#[cfg(test)]` module are also test code.
    let mut current = Some(owner.to_def_id());
    while let Some(def_id) = current {
        // Stop at an external ancestor because only local attributes define this policy.
        let Some(local_def_id) = def_id.as_local() else {
            return false;
        };
        let attrs = cx
            .tcx
            .hir_attrs(cx.tcx.local_def_id_to_hir_id(local_def_id));
        if attrs.iter().any(cfg_requires_test_attr) || is_test_function(cx, local_def_id) {
            return true;
        }
        current = cx.tcx.opt_parent(def_id);
    }
    false
}

/// Return whether a retained `cfg` trace can only be true in test builds.
fn cfg_requires_test_attr(attr: &Attribute) -> bool {
    let Attribute::Parsed(AttributeKind::CfgTrace(entries)) = attr else {
        return false;
    };
    entries
        .iter()
        .any(|(entry, _span)| cfg_requires_test(entry))
}

/// Evaluate whether one `cfg` predicate logically requires `test`.
fn cfg_requires_test(entry: &CfgEntry) -> bool {
    match entry {
        CfgEntry::NameValue { name, value, .. } => *name == sym::test && value.is_none(),
        CfgEntry::All(children, _) => children.iter().any(cfg_requires_test),
        CfgEntry::Any(children, _) => {
            !children.is_empty() && children.iter().all(cfg_requires_test)
        }
        CfgEntry::Not(..) | CfgEntry::Bool(..) | CfgEntry::Version(..) => false,
    }
}

/// Return whether a function is a `#[test]` function in a `--test` build.
fn is_test_function(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    if !matches!(cx.tcx.def_kind(local_def_id), DefKind::Fn) {
        return false;
    }

    // The test harness replaces `#[test]` with a same-named marker constant in the same module.
    let name = cx.tcx.item_name(local_def_id.to_def_id());
    let module = cx.tcx.parent_module_from_def_id(local_def_id);
    cx.tcx.hir_module_free_items(module).any(|item_id| {
        let item = cx.tcx.hir_item(item_id);
        matches!(item.kind, ItemKind::Const(ident, ..) if ident.name == name)
            && cx
                .tcx
                .hir_attrs(item.hir_id())
                .iter()
                .any(|attr| matches!(attr, Attribute::Parsed(AttributeKind::RustcTestMarker(_))))
    })
}

/// Return whether the item path contains a configuration or test word.
fn allowed_def_path(cx: &LateContext<'_>, owner: LocalDefId) -> bool {
    // Def paths include module names, so `config::load` and `tests::helper` are covered even when
    // the function name itself is generic.
    let mut current = Some(owner.to_def_id());
    while let Some(def_id) = current {
        if has_allowed_def_name(cx, def_id) {
            return true;
        }
        current = parent_def_id(cx, def_id);
    }
    false
}

/// Return the parent of a definition, stopping at the crate root.
fn parent_def_id(cx: &LateContext<'_>, def_id: DefId) -> Option<DefId> {
    cx.tcx
        .opt_parent(def_id)
        .filter(|parent| !parent.is_crate_root())
}

/// Return whether the item's file is a build script or a configuration source file.
fn allowed_source_path(cx: &LateContext<'_>, span: Span) -> bool {
    let Some(path) = cx
        .sess()
        .source_map()
        .span_to_filename(span)
        .into_local_path()
    else {
        return false;
    };

    // Only inspect the file stem and immediate parent to avoid allowing every fixture in this lint
    // crate just because the checkout path contains `runtime_env_read`.
    let file_stem_allowed = is_allowed_file_stem(&path);
    let parent_allowed = is_allowed_parent_directory(&path);
    file_stem_allowed || parent_allowed
}

/// Return whether a definition name marks a configuration or test boundary.
fn has_allowed_def_name(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx.opt_item_name(def_id).is_some_and(|name| {
        let name = name.as_str();
        name_has_allowed_context(name) || test_context_name(name)
    })
}

/// Return whether a source file stem marks a build or configuration boundary.
fn is_allowed_file_stem(path: &Path) -> bool {
    path.file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "build" || name_has_allowed_context(name))
}

/// Return whether a source file's parent directory marks a configuration boundary.
fn is_allowed_parent_directory(path: &Path) -> bool {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .is_some_and(name_has_allowed_context)
}

/// Return whether a name contains a configuration or entry-point word.
fn name_has_allowed_context(name: &str) -> bool {
    tokens(name).any(|token| {
        matches!(
            token,
            "cli"
                | "config"
                | "configuration"
                | "bootstrap"
                | "settings"
                | "setting"
                | "env"
                | "environment"
        )
    })
}

/// Return whether a name contains a test word.
fn test_context_name(name: &str) -> bool {
    tokens(name).any(|token| matches!(token, "test" | "tests" | "testing"))
}

/// Split a name into its alphanumeric words.
fn tokens(name: &str) -> impl Iterator<Item = &str> {
    name.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
}

/// Emit the environment-read diagnostic at the callee.
fn emit_env_read_lint(cx: &LateContext<'_>, span: Span, name: &'static str) {
    cx.emit_span_lint(
        RUNTIME_ENV_READ,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(format!(
                "`std::env::{name}` reads runtime environment outside config/bootstrap code"
            ));
            let _ = diag.help(
                "read environment variables in config/bootstrap, CLI entrypoints, Cargo scripts, or tests, then pass typed configuration inward",
            );
        }),
    );
}

/// Run the UI fixture suite.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
