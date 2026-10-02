#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to require substantive documentation for public APIs.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, FieldDef, ForeignItem, ImplItem, ImplItemImplKind, ImplItemKind, Item, ItemKind,
    TraitItem, TraitItemKind, Variant,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, Symbol, def_id::CRATE_DEF_ID, def_id::LocalDefId, sym};

/// Minimum prose words required for an ordinary public API item.
const ITEM_MINIMUM_WORDS: usize = 20;

/// Minimum prose words required for a crate or public module.
const MODULE_MINIMUM_WORDS: usize = 40;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSUFFICIENT_PUBLIC_DOCUMENTATION,
    Warn,
    "public API documentation is too short",
    InsufficientPublicDocumentation
}

/// Public API definition class used to select a threshold and useful guidance.
#[derive(Clone, Copy, Debug)]
enum ApiKind {
    /// The compiled crate root.
    Crate,
    /// An externally reachable module.
    Module,
    /// A free function or associated function.
    Callable(&'static str),
    /// A type, trait, macro, constant, or other named API item.
    Item,
    /// A public field, enum variant, or associated value.
    Value(&'static str),
}

impl ApiKind {
    /// Return the minimum prose-word count for this API definition.
    const fn minimum_words(self) -> usize {
        match self {
            Self::Crate | Self::Module => MODULE_MINIMUM_WORDS,
            Self::Callable(_) | Self::Item | Self::Value(_) => ITEM_MINIMUM_WORDS,
        }
    }

    /// Return the definition label used by the diagnostic.
    const fn label(self) -> &'static str {
        match self {
            Self::Crate => "crate",
            Self::Module => "module",
            Self::Callable(label) | Self::Value(label) => label,
            Self::Item => "public item",
        }
    }

    /// Return guidance that asks for contract details instead of extra filler words.
    const fn guidance(self) -> &'static str {
        // Keep the guidance aligned with the API category selected by the diagnostic.
        match self {
            Self::Crate | Self::Module => {
                "describe responsibilities, key types, invariants, and how callers should navigate the API; add concrete information instead of filler"
            }
            Self::Callable(_) => {
                "describe purpose, caller expectations, return meaning, side effects, and errors or panics when applicable; add concrete information instead of filler"
            }
            Self::Item => {
                "describe the API role, invariants, valid states, and how callers construct or use it; add concrete information instead of filler"
            }
            Self::Value(_) => {
                "describe the semantic meaning, units or constraints, and when this value occurs; add concrete information instead of filler"
            }
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for InsufficientPublicDocumentation {
    /// Check crate-level documentation for this lint.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        check_definition(
            cx,
            CRATE_DEF_ID,
            cx.tcx.def_span(CRATE_DEF_ID),
            ApiKind::Crate,
        );
    }

    /// Check free-standing items and modules for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let kind = match item.kind {
            ItemKind::Mod(..) => ApiKind::Module,
            ItemKind::Fn { .. } => ApiKind::Callable("function"),
            // Re-exports and impl blocks do not define independent API documentation.
            ItemKind::Use(..) | ItemKind::Impl(..) | ItemKind::GlobalAsm { .. } => return,
            ItemKind::ExternCrate(..)
            | ItemKind::Static(..)
            | ItemKind::Const(..)
            | ItemKind::Macro(..)
            | ItemKind::ForeignMod { .. }
            | ItemKind::TyAlias(..)
            | ItemKind::Enum(..)
            | ItemKind::Struct(..)
            | ItemKind::Union(..)
            | ItemKind::Trait { .. }
            | ItemKind::TraitAlias(..) => ApiKind::Item,
        };

        check_definition(cx, item.owner_id.def_id, item.span, kind);
    }

    /// Check associated items declared by public traits for this lint.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        let kind = match item.kind {
            TraitItemKind::Fn(..) => ApiKind::Callable("trait method"),
            TraitItemKind::Const(..) | TraitItemKind::Type(..) => ApiKind::Value("trait item"),
        };

        check_definition(cx, item.owner_id.def_id, item.span, kind);
    }

    /// Check associated items declared by externally reachable inherent impls.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Trait implementations inherit the trait declaration's public documentation contract.
        if !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. }) {
            return;
        }

        let kind = match item.kind {
            ImplItemKind::Fn(..) => ApiKind::Callable("method"),
            ImplItemKind::Const(..) | ImplItemKind::Type(..) => ApiKind::Value("associated item"),
        };

        check_definition(cx, item.owner_id.def_id, item.span, kind);
    }

    /// Check public foreign declarations for this lint.
    fn check_foreign_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ForeignItem<'tcx>) {
        check_definition(cx, item.owner_id.def_id, item.span, ApiKind::Item);
    }

    /// Check externally reachable fields for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        check_definition(cx, field.def_id, field.span, ApiKind::Value("field"));
    }

    /// Check externally reachable enum variants for this lint.
    fn check_variant(&mut self, cx: &LateContext<'tcx>, variant: &'tcx Variant<'tcx>) {
        check_definition(cx, variant.def_id, variant.span, ApiKind::Value("variant"));
    }
}

/// Check one public API definition against its prose-word threshold.
fn check_definition(cx: &LateContext<'_>, def_id: LocalDefId, span: Span, kind: ApiKind) {
    // Generated definitions have no editable source, while effective visibility excludes public
    // syntax hidden behind private ancestry.
    let is_skipped_source = dylint_support::is_internal_support_crate(
        cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
    ) || span.from_expansion()
        || is_doc_hidden(cx, def_id);
    let is_unexported_item =
        def_id != CRATE_DEF_ID && !cx.effective_visibilities.is_exported(def_id);
    if is_skipped_source || is_unexported_item {
        return;
    }

    // Missing documentation is owned by rustc's `missing_docs`; this lint evaluates present docs.
    let hir_id = cx.tcx.local_def_id_to_hir_id(def_id);
    let docs = normalized_docs(cx.tcx.hir_attrs(hir_id));
    if docs.trim().is_empty() {
        return;
    }

    let word_count = prose_word_count(&docs);
    let minimum_words = kind.minimum_words();
    if word_count >= minimum_words {
        return;
    }

    emit_short_documentation(cx, span, kind, word_count, minimum_words);
}

/// Return whether a definition or one of its ancestors is `#[doc(hidden)]`.
fn is_doc_hidden(cx: &LateContext<'_>, def_id: LocalDefId) -> bool {
    // Rustdoc omits hidden items and everything inside them, including a hidden crate root.
    let mut current = Some(def_id.to_def_id());
    while let Some(ancestor) = current {
        // Stop once the first hidden ancestor makes this definition invisible to rustdoc.
        if cx.tcx.is_doc_hidden(ancestor) {
            return true;
        }
        current = cx.tcx.opt_parent(ancestor);
    }
    false
}

/// Join rustc's normalized documentation attributes into Markdown source.
fn normalized_docs(attrs: &[Attribute]) -> String {
    // Preserve attribute order because Markdown block boundaries depend on it.
    let mut docs = String::new();
    // Concatenate only documentation attributes so unrelated attributes stay invisible.
    for attr in attrs {
        let Some(text) = doc_attr_text(attr) else {
            continue;
        };

        if !docs.is_empty() {
            docs.push('\n');
        }
        docs.push_str(text.as_str());
    }
    docs
}

/// Return the text carried by one normalized documentation attribute.
fn doc_attr_text(attr: &Attribute) -> Option<Symbol> {
    attr.doc_str()
        .or_else(|| attr.has_name(sym::doc).then(|| attr.value_str()).flatten())
}

/// Count prose words while excluding fenced, indented, and inline code.
fn prose_word_count(markdown: &str) -> usize {
    // Code examples demonstrate use but do not replace prose that explains the API contract.
    let mut code_block_depth = 0_usize;
    let mut word_count = 0_usize;
    // Track fenced blocks while preserving prose events in their original order.
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_block_depth = code_block_depth.saturating_sub(1),
            Event::Text(text) if code_block_depth == 0 => word_count += text_word_count(&text),
            Event::Start(_)
            | Event::End(_)
            | Event::Text(_)
            | Event::Code(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::Rule
            | Event::TaskListMarker(_) => {}
        }
    }
    word_count
}

/// Count whitespace-delimited tokens that contain at least one alphanumeric character.
fn text_word_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count()
}

/// Emit a short-documentation diagnostic with definition-specific writing guidance.
fn emit_short_documentation(
    cx: &LateContext<'_>,
    span: Span,
    kind: ApiKind,
    word_count: usize,
    minimum_words: usize,
) {
    // Build one stable message from the selected API category and measured prose count.
    let label = kind.label();
    let message =
        format!("{label} documentation has {word_count} prose words; minimum is {minimum_words}");

    // Keep the guidance actionable while leaving the source unchanged.
    cx.emit_span_lint(
        INSUFFICIENT_PUBLIC_DOCUMENTATION,
        span,
        DiagDecorator(move |diag| {
            let _configured_message = diag.primary_message(message);
            let _configured_help = diag.help(kind.guidance());
        }),
    );
}

/// Run the public API documentation UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::prose_word_count;

    /// Confirm code examples cannot satisfy the prose threshold by themselves.
    #[test]
    fn excludes_code_from_word_count() {
        let docs = "Read a value.\n\n```rust\nlet many_code_words = still_not_prose();\n```";

        assert_eq!(prose_word_count(docs), 3);
    }
}
