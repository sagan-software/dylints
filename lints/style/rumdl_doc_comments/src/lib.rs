#![feature(rustc_private)]

//! A lint to run rumdl formatting checks against Rust doc comments.
//!
//! It extracts source doc-comment blocks from Rust items, runs the configured
//! rumdl rules, and maps each warning back to the original source span. The
//! implementation preserves rule filtering and fix metadata so callers receive
//! ordinary lint diagnostics with the same formatting guidance as Markdown.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rumdl_lib::{
    config::Config,
    doc_comment_lint::{
        DocCommentBlock, SKIPPED_RULES, check_doc_comment_blocks, extract_doc_comment_blocks,
    },
    fix_coordinator::FixCoordinator,
    lint_context::LintContext as RumdlLintContext,
    rule::{LintWarning, Rule},
    rules::{MD013LineLength, all_rules, filter_rules},
};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{Attribute, FieldDef, ForeignItem, ImplItem, Item, TraitItem, Variant};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{
    Span, Symbol,
    def_id::{CRATE_DEF_ID, LocalDefId},
    sym,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub RUMDL_DOC_COMMENTS,
    Warn,
    "Rust doc comments should satisfy rumdl Markdown rules",
    RumdlDocComments
}

impl<'tcx> LateLintPass<'tcx> for RumdlDocComments {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        check_def_docs(cx, CRATE_DEF_ID, cx.tcx.def_span(CRATE_DEF_ID));
    }

    /// Check item for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        check_def_docs(cx, item.owner_id.def_id, item.span);
    }

    /// Check trait item for this lint.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        check_def_docs(cx, item.owner_id.def_id, item.span);
    }

    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        check_def_docs(cx, item.owner_id.def_id, item.span);
    }

    /// Check foreign item for this lint.
    fn check_foreign_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ForeignItem<'tcx>) {
        check_def_docs(cx, item.owner_id.def_id, item.span);
    }

    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        check_def_docs(cx, field.def_id, field.span);
    }

    /// Check variant for this lint.
    fn check_variant(&mut self, cx: &LateContext<'tcx>, variant: &'tcx Variant<'tcx>) {
        check_def_docs(cx, variant.def_id, variant.span);
    }
}

/// Check def docs for this lint.
fn check_def_docs(cx: &LateContext<'_>, def_id: LocalDefId, span: Span) {
    // Recover exact source when possible so rumdl fixes can become suggestions.
    let attrs = cx.tcx.hir_attrs(cx.tcx.local_def_id_to_hir_id(def_id));
    let Some(source) = doc_comment_source(cx, attrs, span) else {
        return;
    };

    let config = Config::default();
    let rules = doc_comment_rules(&config);
    let warnings = check_doc_comment_blocks(&source.text, &rules, &config);

    // Report one warning per docs block to avoid noisy duplicate diagnostics on the same comment.
    if let Some(warning) = warnings.first() {
        let suggestion = source
            .is_exact
            .then(|| suggested_doc_comment_source(&source.text, warning, &rules, &config))
            .flatten()
            .filter(|suggested| suggested != &source.text);

        emit_span_lint_with_suggestion(
            cx,
            RUMDL_DOC_COMMENTS,
            source.span,
            warning_message(warning),
            "fix the Markdown in this Rust doc comment",
            suggestion,
        );
    }
}

/// State used by the doc comment source analysis.
struct DocCommentSource {
    /// text stored for this lint's analysis.
    text: String,
    /// span stored for this lint's analysis.
    span: Span,
    /// is exact stored for this lint's analysis.
    is_exact: bool,
}

/// Return source text for doc comment.
fn doc_comment_source(
    cx: &LateContext<'_>,
    attrs: &[Attribute],
    fallback_span: Span,
) -> Option<DocCommentSource> {
    if let Some((span, text)) = exact_doc_comment_source(cx, attrs) {
        return Some(DocCommentSource {
            text,
            span,
            is_exact: true,
        });
    }

    normalized_doc_comment_source(attrs).map(|text| DocCommentSource {
        text,
        span: fallback_span,
        is_exact: false,
    })
}

/// Return source text for exact doc comment.
fn exact_doc_comment_source(cx: &LateContext<'_>, attrs: &[Attribute]) -> Option<(Span, String)> {
    let doc_spans = attrs
        .iter()
        .filter(|attr| doc_attr_text(attr).is_some())
        .map(Attribute::span)
        .filter(|span| !span.from_expansion())
        .collect::<Vec<_>>();

    let first = doc_spans.first().copied()?;
    let source_map = cx.sess().source_map();

    // Only exact line-doc-comment source is safe to replace. Attribute and block forms still lint
    // through the normalized fallback because rumdl cannot restore those Rust syntaxes here.
    if doc_spans.iter().any(|span| {
        source_map.span_to_snippet(*span).map_or(true, |source| {
            !matches!(source.trim_start().get(..3), Some("///" | "//!"))
        })
    }) {
        return None;
    }

    let span = doc_spans.iter().copied().skip(1).fold(first, Span::to);
    let source = source_map.span_to_snippet(span).ok()?;

    // rumdl's exact doc-comment path only supports line comments. Keep block comments and
    // synthetic doc attributes on the normalized, non-fixable fallback path.
    if source.contains('\r') || extract_doc_comment_blocks(&source).is_empty() {
        return None;
    }

    Some((span, source))
}

/// Return the normalized doc comment source.
fn normalized_doc_comment_source(attrs: &[Attribute]) -> Option<String> {
    // Preserve attribute and line order while rebuilding line-doc-comment source.
    let mut source = String::new();

    for attr in attrs {
        let Some(doc) = doc_attr_text(attr) else {
            continue;
        };

        // rumdl's doc-comment checker expects Rust source lines, so reconstruct a minimal line
        // comment block from the normalized doc attribute text.
        for line in doc
            .as_str()
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .lines()
        {
            source.push_str("///");
            source.push_str(line);
            source.push('\n');
        }
    }

    (!source.trim().is_empty()).then_some(source)
}

/// Helper for doc comment rules analysis.
fn doc_comment_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    let all = all_rules(config);

    filter_rules(&all, &config.global)
        .into_iter()
        .filter_map(|rule| {
            if SKIPPED_RULES.contains(&rule.name()) {
                None
            } else if let Some(md013) = rule.as_any().downcast_ref::<MD013LineLength>() {
                let rule: Box<dyn Rule> = Box::new(md013.with_code_blocks_disabled());
                Some(rule)
            } else {
                Some(rule)
            }
        })
        .collect()
}

/// Return source text for formatted doc comment.
fn formatted_doc_comment_source(
    source: &str,
    rules: &[Box<dyn Rule>],
    config: &Config,
) -> Option<String> {
    // Extract source blocks once and replace them from the end to preserve offsets.
    let blocks = extract_doc_comment_blocks(source);
    let mut formatted = source.to_string();

    for block in blocks.iter().rev() {
        // Leave empty Markdown blocks unchanged.
        if block.markdown.trim().is_empty() {
            continue;
        }

        let mut markdown = block.markdown.clone();
        let ctx = RumdlLintContext::new(&markdown, config.markdown_flavor(), None);
        let warnings = rules
            .iter()
            .filter_map(|rule| rule.check(&ctx).ok())
            .flatten()
            .collect::<Vec<_>>();

        // Apply rumdl's coordinated fixes before restoring Rust comment prefixes.
        let _fix_result = FixCoordinator::new()
            .apply_fixes_iterative(rules, &warnings, &mut markdown, config, 100, None)
            .ok()?;

        let replacement = restore_doc_comment_block(block, &markdown);
        formatted.replace_range(block.byte_start..block.byte_end, &replacement);
    }

    Some(formatted)
}

/// Return source text for suggested doc comment.
fn suggested_doc_comment_source(
    source: &str,
    warning: &LintWarning,
    rules: &[Box<dyn Rule>],
    config: &Config,
) -> Option<String> {
    let formatted = formatted_doc_comment_source(source, rules, config)?;
    if formatted == source {
        return None;
    }

    // Only expose a machine-applicable suggestion when it clears the diagnostic being emitted.
    // Some rumdl warnings, such as MD013, can coexist with other fixable warnings in a block.
    let remaining_warnings = check_doc_comment_blocks(&formatted, rules, config);
    remaining_warnings
        .iter()
        .all(|remaining| !same_warning_location(warning, remaining))
        .then_some(formatted)
}

/// Return whether warning location match.
fn same_warning_location(left: &LintWarning, right: &LintWarning) -> bool {
    left.rule_name == right.rule_name && left.line == right.line && left.column == right.column
}

/// Helper for restore doc comment block analysis.
fn restore_doc_comment_block(block: &DocCommentBlock, markdown: &str) -> String {
    // Reuse the last available metadata when formatting adds Markdown lines.
    let Some(default_metadata) = block
        .line_metadata
        .last()
        .or_else(|| block.line_metadata.first())
    else {
        return String::new();
    };

    // Preserve whether this block documents an outer or inner item.
    let bare_prefix = match block.kind {
        rumdl_lib::doc_comment_lint::DocCommentKind::Outer => "///",
        rumdl_lib::doc_comment_lint::DocCommentKind::Inner => "//!",
    };

    // Restore each line with its original whitespace and comment prefix.
    let mut restored = String::new();
    let markdown_lines = markdown.split('\n').collect::<Vec<_>>();
    for (index, line) in markdown_lines.iter().enumerate() {
        let metadata = block.line_metadata.get(index).unwrap_or(default_metadata);
        restored.push_str(&metadata.leading_whitespace);
        if line.is_empty() {
            restored.push_str(bare_prefix);
        } else {
            restored.push_str(&metadata.prefix);
            restored.push_str(line);
        }

        // Reinsert separators between lines without adding a trailing newline.
        if index + 1 < markdown_lines.len() {
            restored.push('\n');
        }
    }

    restored
}

/// Helper for doc attr text analysis.
fn doc_attr_text(attr: &Attribute) -> Option<Symbol> {
    attr.doc_str()
        .or_else(|| attr.has_name(sym::doc).then(|| attr.value_str()).flatten())
}

/// Helper for warning message analysis.
fn warning_message(warning: &LintWarning) -> String {
    // Use rumdl as the stable fallback when a warning omits its rule name.
    let rule = warning.rule_name.as_deref().unwrap_or("rumdl");

    // Include the Markdown location because the Rust span covers the whole block.
    format!(
        "{rule} in Rust doc comment at line {}, column {}: {}",
        warning.line, warning.column, warning.message
    )
}

/// Emit the span lint with suggestion diagnostic.
fn emit_span_lint_with_suggestion(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: String,
    help: &'static str,
    suggestion: Option<String>,
) {
    // Use rustc's native diagnostic decorator to keep diagnostics consistent with this suite.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _configured_message = diag.primary_message(message);
            if let Some(suggestion) = suggestion {
                let _configured_suggestion =
                    diag.span_suggestion(span, help, suggestion, Applicability::MachineApplicable);
            } else {
                let _configured_help = diag.help(help);
            }
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
