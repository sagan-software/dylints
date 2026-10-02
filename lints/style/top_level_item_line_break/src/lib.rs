#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to require line breaks between top-level source items.
//!
//! It walks the source-order item list for the crate root and each loaded
//! module. It compares the source lines of neighboring item spans so comments
//! and attributes stay part of the source boundary that the lint checks.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::{Crate, Item, ItemKind, ModKind};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub TOP_LEVEL_ITEM_LINE_BREAK,
    Warn,
    "top-level items should have a line break between them",
    TopLevelItemLineBreak
}

impl EarlyLintPass for TopLevelItemLineBreak {
    /// Check the crate root and every loaded module for adjacent items.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_module_items(cx, &krate.items);
    }
}

/// Check one module's direct source-order items and recurse into child modules.
fn check_module_items(cx: &EarlyContext<'_>, items: &[Box<Item>]) {
    for pair in items.windows(2) {
        let [previous, current] = pair else {
            continue;
        };

        if missing_line_break(cx, previous.span, current.span) {
            emit_missing_line_break(cx, current.span);
        }
    }

    for item in items {
        if item.span.from_expansion() {
            continue;
        }

        if let ItemKind::Mod(_, _, ModKind::Loaded(child_items, ..)) = &item.kind {
            check_module_items(cx, child_items);
        }
    }
}

/// Return whether two source items start and end on the same physical line.
fn missing_line_break(cx: &EarlyContext<'_>, previous: Span, current: Span) -> bool {
    if previous.from_expansion() || current.from_expansion() {
        return false;
    }

    let source_map = cx.sess().source_map();
    let previous_line = source_map.lookup_char_pos(previous.hi()).line;
    let current_line = source_map.lookup_char_pos(current.lo()).line;
    previous_line == current_line
}

/// Emit the diagnostic on the item that needs to move to a new line.
fn emit_missing_line_break(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        TOP_LEVEL_ITEM_LINE_BREAK,
        span,
        DiagDecorator(|diag| {
            let _configured_message =
                diag.primary_message("top-level items should have a line break between them");
            let _configured_help = diag.help("put this item on a new line");
        }),
    );
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
