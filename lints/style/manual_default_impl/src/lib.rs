#![feature(rustc_private)]
#![expect(
    clippy::string_slice,
    clippy::let_underscore_must_use,
    reason = "rustc source offsets identify impl bodies and diagnostics are configured in place"
)]

//! A lint to check for manual Default implementations that could be derived.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_DEFAULT_IMPL,
    Warn,
    "manual `Default` implementation could be derived",
    ManualDefaultImpl
}

impl<'tcx> LateLintPass<'tcx> for ManualDefaultImpl {
    /// Check item for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Ok(source) = cx.sess().source_map().span_to_snippet(item.span) else {
            return;
        };

        if manual_default_impl_can_derive(&source) {
            emit_span_lint_with_help(
                cx,
                MANUAL_DEFAULT_IMPL,
                item.span,
                "manual `Default` implementation looks derivable",
                "use `#[derive(Default)]` when all fields use their own default values",
            );
        }
    }
}

/// Helper for manual default impl can derive analysis.
fn manual_default_impl_can_derive(source: &str) -> bool {
    // Establish the implementation and method shape before inspecting its result.
    let Some(header) = source.split('{').next() else {
        return false;
    };
    if !default_impl_header(header) || !source.contains("fn default(") || has_custom_marker(source)
    {
        return false;
    }

    // Only diagnose a direct `Self { ... }` result whose fields are all explicit defaults.
    let Some(fields) = self_struct_fields(source) else {
        return false;
    };
    // Require every field initializer to delegate to its field type's default.
    all_field_initializers_are_default_calls(fields)
}

/// Helper for default impl header analysis.
fn default_impl_header(header: &str) -> bool {
    header.contains(" Default for ") || header.contains(" std::default::Default for ")
}

/// Return whether custom marker is present.
fn has_custom_marker(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    if ["if ", "match ", "todo", "panic"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return true;
    }

    // String literals and explicit custom/invariant comments are strong signals this is deliberate.
    source.contains('"')
        || source.lines().any(|line| {
            comment_line_has_marker(line, "custom") || comment_line_has_marker(line, "invariant")
        })
}

/// Helper for comment line has marker analysis.
fn comment_line_has_marker(line: &str, marker: &str) -> bool {
    let Some(comment_start) = line
        .as_bytes()
        .windows(2)
        .position(|pair| matches!(pair, [b'/', b'/' | b'*']))
    else {
        return false;
    };

    line[comment_start..].to_ascii_lowercase().contains(marker)
}

/// Helper for self struct fields analysis.
fn self_struct_fields(source: &str) -> Option<&str> {
    let start = source.rfind("Self {")?;
    let open_brace = start + "Self ".len();
    let close_brace = matching_brace(source, open_brace)?;

    Some(&source[open_brace + 1..close_brace])
}

/// Helper for matching brace analysis.
fn matching_brace(source: &str, open_brace: usize) -> Option<usize> {
    // Track nested delimiters from the known opening brace.
    let mut depth = 0_u32;

    // Match braces from the `Self {` open brace so nested blocks force a conservative skip.
    for (offset, ch) in source[open_brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                // Return the first closing brace that balances the known opening brace.
                if depth == 0 {
                    return Some(open_brace + offset);
                }
            }
            _ => {}
        }
    }

    None
}

/// Helper for all field initializers are default calls analysis.
fn all_field_initializers_are_default_calls(fields: &str) -> bool {
    let mut saw_field = false;

    // Inspect each top-level field without splitting nested initializer syntax.
    for field in split_top_level_fields(fields) {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }

        // Require an explicit field name and initializer instead of shorthand syntax.
        let Some((_, initializer)) = field.split_once(':') else {
            return false;
        };
        // Accept the struct only when every observed initializer is a default call.
        if !default_call(initializer.trim()) {
            return false;
        }
        saw_field = true;
    }

    saw_field
}

/// Split top level fields while preserving source structure.
fn split_top_level_fields(fields: &str) -> Vec<&str> {
    // Record slice boundaries while retaining the original initializer text.
    let mut parts = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0_i32;
    let mut angle_depth = 0_i32;
    let mut bracket_depth = 0_i32;

    // Split only on commas that are not part of generic arguments, arrays, or call syntax.
    for (index, ch) in fields.char_indices() {
        if is_delimiter(ch, &mut paren_depth, &mut angle_depth, &mut bracket_depth) {
            continue;
        }
        if ch == ',' && paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 {
            parts.push(&fields[start..index]);
            start = index + ch.len_utf8();
        }
    }
    // Preserve the final field after the last top-level comma.
    parts.push(&fields[start..]);

    parts
}

/// Update nested delimiter depth and report whether the character was a delimiter.
const fn is_delimiter(
    ch: char,
    paren_depth: &mut i32,
    angle_depth: &mut i32,
    bracket_depth: &mut i32,
) -> bool {
    match ch {
        '(' => *paren_depth += 1,
        ')' => *paren_depth -= 1,
        '<' => *angle_depth += 1,
        '>' => *angle_depth -= 1,
        '[' => *bracket_depth += 1,
        ']' => *bracket_depth -= 1,
        _ => return false,
    }
    true
}

/// Helper for default call analysis.
fn default_call(initializer: &str) -> bool {
    let initializer = initializer.trim();

    initializer == "Default::default()"
        || initializer.ends_with("::Default::default()")
        || (initializer.starts_with('<') && initializer.ends_with(">::default()"))
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to match the rest of this lint suite.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

/// Helper for detects all default struct literal analysis.
#[test]
fn detects_all_default_struct_literal() {
    let source = r"
impl Default for Config {
    /// Helper for default analysis.
    fn default() -> Self {
        Self {
            retries: Default::default(),
            labels: <Vec<String>>::default(),
        }
    }
}
";

    assert!(manual_default_impl_can_derive(source));
}
