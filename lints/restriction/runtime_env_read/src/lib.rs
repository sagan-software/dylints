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

use std::{path::Path, str::FromStr};

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
        // Allow entrypoints and named boundaries before scanning the body for environment reads.
        if cx.tcx.opt_item_name(owner.to_def_id()) == Some(sym::main)
            || in_test_code(cx, owner)
            || allowed_def_path(cx, owner)
            || allowed_source_path(cx, cx.tcx.def_span(owner))
        {
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
            && let Some(read) = resolved_env_read(self.cx, callee)
        {
            emit_env_read_lint(self.cx, callee.span, read);
        }

        // Continue traversal so nested environment reads also receive diagnostics.
        intravisit::walk_expr(self, expr);
    }
}

/// Identifies the standard environment read operations checked by this lint.
#[derive(Clone, Copy, Debug)]
enum EnvRead {
    /// Reads one UTF-8 environment value with `std::env::var`.
    Var,
    /// Reads one platform-specific environment value with `std::env::var_os`.
    VarOs,
    /// Iterates over UTF-8 environment values with `std::env::vars`.
    Vars,
    /// Iterates over platform-specific environment values with `std::env::vars_os`.
    VarsOs,
}

impl EnvRead {
    /// Return the standard-library function name used by the diagnostic.
    const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::Var => "var",
            Self::VarOs => "var_os",
            Self::Vars => "vars",
            Self::VarsOs => "vars_os",
        }
    }
}

impl FromStr for EnvRead {
    type Err = ();

    /// Parse only the four standard-library environment read names reported here.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "var" => Ok(Self::Var),
            "var_os" => Ok(Self::VarOs),
            "vars" => Ok(Self::Vars),
            "vars_os" => Ok(Self::VarsOs),
            _ => Err(()),
        }
    }
}

/// Resolve a callee to one of the standard environment read operations.
fn resolved_env_read(cx: &LateContext<'_>, callee: &Expr<'_>) -> Option<EnvRead> {
    let ExprKind::Path(qpath) = callee.kind else {
        return None;
    };
    let Res::Def(DefKind::Fn, def_id) = cx.qpath_res(&qpath, callee.hir_id) else {
        return None;
    };

    // The definition path names the defining crate and module, so renames and aliases match.
    // Restrict parsing to `std::env` before accepting a function from the closed vocabulary.
    match cx.get_def_path(def_id).as_slice() {
        [krate, module, name] if *krate == sym::std && *module == sym::env => {
            name.as_str().parse().ok()
        }
        _ => None,
    }
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
        current = cx
            .tcx
            .opt_parent(def_id)
            .filter(|parent| !parent.is_crate_root());
    }
    false
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
    // Inherent impls have no item name, but their resolved path ends with the self type name.
    let impl_self_name = matches!(cx.tcx.def_kind(def_id), DefKind::Impl { .. })
        .then(|| cx.get_def_path(def_id))
        .and_then(|path| path.last().copied());
    cx.tcx
        .opt_item_name(def_id)
        .or(impl_self_name)
        .is_some_and(|name| {
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
    tokens(name).any(|word| {
        word_matches_any(
            word,
            &[
                "cli",
                "config",
                "configuration",
                "bootstrap",
                "settings",
                "setting",
                "env",
                "environment",
            ],
        )
    })
}

/// Return whether a name contains a test word.
fn test_context_name(name: &str) -> bool {
    tokens(name).any(|word| word_matches_any(word, &["test", "tests", "testing"]))
}

/// Split a name lazily into borrowed ASCII alphanumeric words.
///
/// ASCII lower-to-upper and acronym-to-word transitions split tokens. Other
/// non-alphanumeric ASCII characters and each non-ASCII Unicode scalar separate
/// tokens, matching the lint's ASCII-only vocabulary.
fn tokens(name: &str) -> impl Iterator<Item = &str> {
    let mut remaining = name;
    std::iter::from_fn(move || next_token(&mut remaining))
}

/// Return the next borrowed word and advance past its following separator.
fn next_token<'a>(remaining: &mut &'a str) -> Option<&'a str> {
    loop {
        // Restart at the unread suffix so earlier words stay out of this scan.
        let mut chars = remaining.char_indices().peekable();
        let (_, first) = chars.next()?;
        if !first.is_ascii_alphanumeric() {
            // `char_indices` provides a UTF-8 boundary for the skipped character.
            *remaining = remaining.split_at(first.len_utf8()).1;
            continue;
        }

        let mut previous = first;
        while let Some((index, current)) = chars.next() {
            if !current.is_ascii_alphanumeric() {
                // Both offsets come from `char_indices` or a complete character width.
                let (word, suffix) = remaining.split_at(index);
                *remaining = suffix.split_at(current.len_utf8()).1;
                return Some(word);
            }
            if is_word_boundary(previous, current, chars.peek().map(|(_, next)| *next)) {
                // The word boundary is the current character's `char_indices` offset.
                let (word, suffix) = remaining.split_at(index);
                *remaining = suffix;
                return Some(word);
            }
            previous = current;
        }

        // With no separator or case transition, the unread suffix is the final word.
        let word = *remaining;
        *remaining = "";
        return Some(word);
    }
}

/// Return whether an ASCII lower-to-upper or acronym-to-word boundary starts here.
fn is_word_boundary(previous: char, current: char, next: Option<char>) -> bool {
    (previous.is_ascii_lowercase() && current.is_ascii_uppercase())
        || (previous.is_ascii_uppercase()
            && current.is_ascii_uppercase()
            && next.is_some_and(|next| next.is_ascii_lowercase()))
}

/// Return whether a name word matches any lowercase policy word without allocating.
fn word_matches_any(word: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| word.eq_ignore_ascii_case(candidate))
}

/// Emit the environment-read diagnostic at the callee.
fn emit_env_read_lint(cx: &LateContext<'_>, span: Span, read: EnvRead) {
    cx.emit_span_lint(
        RUNTIME_ENV_READ,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(format!(
                "`std::env::{}` reads runtime environment outside config/bootstrap code",
                read.diagnostic_name()
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
