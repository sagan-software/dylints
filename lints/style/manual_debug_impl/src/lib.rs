#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for manual Debug implementations that could be derived.
//!
//! It resolves `Debug` implementations for local, non-generic structs whose
//! `fmt` body is one `debug_struct` or `debug_tuple` builder chain that names
//! the struct and lists every field in declaration order as `&self.field`. That
//! chain prints exactly what `#[derive(Debug)]` prints, so the fix replaces the
//! implementation with the derive. Redacting or reordering implementations do
//! not match the chain and are left alone.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{Body, Expr, ExprKind, Impl, ImplItemKind, Item, ItemKind, PatKind, def::Res};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, AdtDef, TyCtxt, TypeckResults};
use rustc_span::{Span, Symbol, def_id::DefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_DEBUG_IMPL,
    Warn,
    "manual `Debug` implementation could be derived",
    ManualDebugImpl
}

impl<'tcx> LateLintPass<'tcx> for ManualDebugImpl {
    /// Check one `Debug` implementation.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Filter unsupported implementations before resolving the formatter body.
        let Some(adt) = supported_debug_struct(cx, item) else {
            return;
        };
        // Require one source-level `fmt` method before inspecting its body.
        let ItemKind::Impl(Impl {
            of_trait: Some(_),
            items: [fmt_id],
            ..
        }) = item.kind
        else {
            return;
        };

        // The `fmt` body must be one builder chain that matches the derived output.
        let ImplItemKind::Fn(_, body_id) = cx.tcx.hir_impl_item(*fmt_id).kind else {
            return;
        };
        let body = cx.tcx.hir_body(body_id);
        let checker = ChainChecker {
            tcx: cx.tcx,
            typeck_results: cx.tcx.typeck_body(body_id),
            adt,
            receiver: receiver_binding(body),
        };
        let ExprKind::Block(block, None) = body.value.kind else {
            return;
        };
        let is_derived_body = is_derived_body(&checker, block);
        if is_derived_body {
            emit(cx, item, adt.did());
        }
    }
}

/// Return the local struct for a source-authored, concrete `Debug` implementation.
fn supported_debug_struct<'tcx>(cx: &LateContext<'tcx>, item: &Item<'tcx>) -> Option<AdtDef<'tcx>> {
    let ItemKind::Impl(Impl {
        of_trait: Some(_), ..
    }) = item.kind
    else {
        return None;
    };
    // Resolve the implemented trait and its receiver before checking derive safety.
    let trait_ref = cx.tcx.impl_trait_ref(item.owner_id).skip_binder();
    let ty::Adt(adt, args) = trait_ref.self_ty().kind() else {
        return None;
    };
    // Exclude generated implementations and types that cannot receive a safe derive fix.
    let is_source_authored = !item.span.from_expansion();
    let debug_trait = cx.tcx.is_diagnostic_item(sym::Debug, trait_ref.def_id);
    let is_local_struct = adt.is_struct() && adt.did().is_local();
    let is_unpacked = !adt.repr().packed();
    let is_concrete = args.non_erasable_generics().next().is_none();
    // Keep the source, type, representation, and generic checks independent.
    // The derive replacement is valid only after all these restrictions hold.
    (is_source_authored && debug_trait && is_local_struct && is_unpacked && is_concrete)
        .then_some(*adt)
}

/// Return whether the body has no statements and matches derived formatting.
fn is_derived_body<'tcx>(checker: &ChainChecker<'tcx>, block: &rustc_hir::Block<'tcx>) -> bool {
    let Some(expression) = block.expr else {
        return false;
    };
    block.stmts.is_empty() && checker.matches_derive(expression)
}

/// Resolved context for comparing one `fmt` body with the derived output.
struct ChainChecker<'tcx> {
    /// Compiler context used to resolve methods and items.
    tcx: TyCtxt<'tcx>,
    /// Type-checking results of the `fmt` body.
    typeck_results: &'tcx TypeckResults<'tcx>,
    /// The struct whose `Debug` output is checked.
    adt: AdtDef<'tcx>,
    /// The `self` binding of `fmt`, when it is a plain binding.
    receiver: Option<rustc_hir::HirId>,
}

impl<'tcx> ChainChecker<'tcx> {
    /// Return whether the expression is `f.debug_*("Name").field(..)*.finish()`
    /// for every field.
    fn matches_derive(&self, chain: &'tcx Expr<'tcx>) -> bool {
        // Peel `.finish()` and then each `.field(..)` call, outermost first.
        let Some((receiver, [])) = self.method_call(chain, "finish") else {
            return false;
        };
        let mut field_calls = Vec::new();
        let mut current = receiver;
        while let Some((inner, arguments)) = self.method_call(current, "field") {
            field_calls.push(arguments);
            current = inner;
        }
        field_calls.reverse();

        // The chain must start at a `Formatter` builder named after the struct.
        let variant = self.adt.non_enum_variant();
        let name = self.tcx.item_name(self.adt.did());
        let Some((builder, is_tuple)) = self.builder_call(current) else {
            return false;
        };
        // The builder must target `Formatter` and carry the struct's exact name.
        if !self.is_valid_builder(
            current,
            builder,
            name,
            field_calls.len(),
            variant.fields.len(),
        ) {
            return false;
        }

        // Each call must show one field, in declaration order, under its own name.
        // The final comparison also rejects missing, extra, or reordered fields.
        variant
            .fields
            .iter()
            .zip(field_calls)
            .all(|(field, arguments)| match (is_tuple, arguments) {
                (false, [label, value]) => {
                    is_string_literal(label, field.name) && self.is_self_field(value, field.name)
                }
                (true, [value]) => self.is_self_field(value, field.name),
                _ => false,
            })
    }

    /// Return the initial `debug_struct` or `debug_tuple` call.
    fn builder_call(&self, expr: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, bool)> {
        // Prefer the named builder used by ordinary structs.
        if let Some((_, [label])) = self.method_call(expr, "debug_struct") {
            return Some((label, false));
        }
        if let Some((_, [label])) = self.method_call(expr, "debug_tuple") {
            return Some((label, true));
        }
        None
    }

    /// Return whether the builder has the resolved formatter and expected field count.
    fn is_valid_builder(
        &self,
        current: &Expr<'_>,
        builder: &Expr<'_>,
        name: Symbol,
        field_count: usize,
        expected_fields: usize,
    ) -> bool {
        self.is_formatter_method(current)
            && is_string_literal(builder, name)
            && field_count == expected_fields
    }

    /// Return the receiver and arguments of a resolved method call with the given name.
    fn method_call(
        &self,
        expr: &'tcx Expr<'tcx>,
        name: &str,
    ) -> Option<(&'tcx Expr<'tcx>, &'tcx [Expr<'tcx>])> {
        let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind else {
            return None;
        };
        let method = self.typeck_results.type_dependent_def_id(expr.hir_id)?;
        (!expr.span.from_expansion() && self.tcx.item_name(method).as_str() == name)
            .then_some((receiver, arguments))
    }

    /// Return whether a builder method is defined on `std::fmt::Formatter`.
    fn is_formatter_method(&self, call: &Expr<'_>) -> bool {
        self.typeck_results
            .type_dependent_def_id(call.hir_id)
            .map(|method| self.tcx.parent(method))
            .is_some_and(|owner| {
                matches!(
                    self.tcx.type_of(owner).instantiate_identity().skip_norm_wip().kind(),
                    ty::Adt(formatter, _) if self.tcx.is_diagnostic_item(sym::Formatter, formatter.did())
                )
            })
    }

    /// Return whether an expression is `&self.field` for the given field.
    fn is_self_field(&self, value: &Expr<'_>, field_name: Symbol) -> bool {
        // The derived formatter borrows each field from the receiver.
        let ExprKind::AddrOf(_, _, inner) = value.kind else {
            return false;
        };
        // Resolve the selected field before checking the receiver binding.
        let ExprKind::Field(base, field) = inner.kind else {
            return false;
        };
        // The base must resolve to the method's plain `self` binding.
        let ExprKind::Path(ref qpath) = base.kind else {
            return false;
        };
        field.name == field_name
            && matches!(
                self.typeck_results.qpath_res(qpath, base.hir_id),
                Res::Local(binding) if Some(binding) == self.receiver
            )
    }
}

/// Return the `self` binding of a method body.
fn receiver_binding(body: &Body<'_>) -> Option<rustc_hir::HirId> {
    let param = body.params.first()?;
    let PatKind::Binding(_, binding, _, _) = param.pat.kind else {
        return None;
    };
    Some(binding)
}

/// Return whether an expression is a string literal equal to the expected text.
fn is_string_literal(expr: &Expr<'_>, expected: Symbol) -> bool {
    matches!(expr.kind, ExprKind::Lit(literal) if matches!(literal.node, LitKind::Str(text, _) if text == expected))
}

/// Emit the lint with a derive fix when the implementation can be removed cleanly.
fn emit(cx: &LateContext<'_>, item: &Item<'_>, struct_id: DefId) {
    let fix = derive_fix(cx, item, struct_id);
    cx.emit_span_lint(
        MANUAL_DEBUG_IMPL,
        item.span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message("manual `Debug` implementation looks derivable");
            let help = "derive `Debug` instead; it prints the same output";
            if let Some(fix) = fix {
                let _ = diag.multipart_suggestion(help, fix, Applicability::MachineApplicable);
            } else {
                let _ = diag.help(help);
            }
        }),
    );
}

/// Return the edits that add `#[derive(Debug)]` to the struct and remove the impl.
///
/// The fix is offered only when neither item comes from a macro and the impl has
/// no attributes or doc comments that would be left without an item.
fn derive_fix(
    cx: &LateContext<'_>,
    item: &Item<'_>,
    struct_id: DefId,
) -> Option<Vec<(Span, String)>> {
    let struct_item = cx.tcx.hir_expect_item(struct_id.as_local()?);
    // Preserve attributes and source ownership when removing the implementation.
    let is_generated = struct_item.span.from_expansion();
    let has_attributes = !cx.tcx.hir_attrs(item.hir_id()).is_empty();
    if is_generated || has_attributes {
        return None;
    }
    // Derive placement follows the source indentation of the struct item.
    let source_map = cx.sess().source_map();
    let location = source_map.lookup_char_pos(struct_item.span.lo());
    let indentation: String = location
        .file
        .get_line(location.line.saturating_sub(1))?
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .collect();
    Some(vec![
        (
            struct_item.span.shrink_to_lo(),
            format!("#[derive(Debug)]\n{indentation}"),
        ),
        (item.span, String::new()),
    ])
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
