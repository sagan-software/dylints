#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to require line breaks between top-level source items.
//!
//! It walks the source-order item list for the crate root and each loaded
//! module. It compares the line where one item ends with the line where the
//! next item, including its outer attributes, starts. The fix inserts a line
//! break before the second item, which never changes the meaning of the code.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::{AttrStyle, Crate, Item, ItemKind, ModKind};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{BytePos, Pos, Span, source_map::SourceMap};

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
    // Compare adjacent items because only neighboring source spans can share a line.
    for pair in items.windows(2) {
        let [previous, current] = pair else {
            continue;
        };

        // Include outer attributes and doc comments when locating the item's source start.
        let current_start = current
            .attrs
            .iter()
            .filter(|attr| attr.style == AttrStyle::Outer && !attr.span.from_expansion())
            .map(|attr| attr.span.lo())
            .fold(current.span.lo(), BytePos::min);
        // Build one machine-applicable edit for each missing separator.
        if is_missing_line_break(cx, previous.span, current.span, current_start) {
            let (replaced, replacement) = line_break_fix(cx, previous.span, current_start);
            emit_missing_line_break(cx, current.span, replaced, replacement);
        }
    }

    // Recurse only into source-loaded modules so macro expansions stay untouched.
    for item in items {
        if item.span.from_expansion() {
            continue;
        }

        if let ItemKind::Mod(_, _, ModKind::Loaded(child_items, ..)) = &item.kind {
            check_module_items(cx, child_items);
        }
    }
}

/// Return whether the previous item ends on the line where the current item starts.
fn is_missing_line_break(
    cx: &EarlyContext<'_>,
    previous: Span,
    current: Span,
    current_start: BytePos,
) -> bool {
    // Generated spans cannot receive a source edit.
    if previous.from_expansion() || current.from_expansion() {
        return false;
    }

    // Compare the previous item's end with the next item's first source line.
    let source_map = cx.sess().source_map();
    let previous_line = source_map.lookup_char_pos(previous.hi()).line;
    let current_line = source_map.lookup_char_pos(current_start).line;
    previous_line == current_line
}

/// Return the source range and text that move the current item to its own line.
///
/// The new line repeats the indentation of the shared line. When only
/// whitespace separates the items, that whitespace is replaced; otherwise, such
/// as after a block comment, the line break is inserted before the item.
fn line_break_fix(cx: &EarlyContext<'_>, previous: Span, current_start: BytePos) -> (Span, String) {
    // Preserve the indentation already used by the current item.
    let source_map = cx.sess().source_map();
    let location = source_map.lookup_char_pos(current_start);
    let indentation: String = location
        .file
        .get_line(location.line.saturating_sub(1))
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).collect())
        .unwrap_or_default();
    let gap = previous.with_lo(previous.hi()).with_hi(current_start);
    // Replace existing whitespace, or insert before a non-whitespace separator.
    let replaced = if is_whitespace_gap(source_map, gap) {
        gap
    } else {
        // Keep comments on the previous item while consuming the separator's trailing whitespace.
        let trailing_whitespace = source_map
            .span_to_snippet(gap)
            .map_or(0, |snippet| snippet.len() - snippet.trim_end().len());
        gap.with_lo(gap.hi() - BytePos::from_usize(trailing_whitespace))
    };
    (replaced, format!("\n{indentation}"))
}

/// Return whether a source gap contains only whitespace.
fn is_whitespace_gap(source_map: &SourceMap, gap: Span) -> bool {
    source_map
        .span_to_snippet(gap)
        .is_ok_and(|snippet| snippet.trim().is_empty())
}

/// Emit the diagnostic on the item that needs to move to a new line.
///
/// The replacement changes only whitespace between two items, so the suggestion
/// is machine applicable.
fn emit_missing_line_break(cx: &EarlyContext<'_>, span: Span, replaced: Span, replacement: String) {
    cx.emit_span_lint(
        TOP_LEVEL_ITEM_LINE_BREAK,
        span,
        DiagDecorator(move |diag| {
            let _configured_message =
                diag.primary_message("top-level items should have a line break between them");
            let _configured_suggestion = diag.span_suggestion(
                replaced,
                "put this item on a new line",
                replacement,
                Applicability::MachineApplicable,
            );
        }),
    );
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
