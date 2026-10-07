#![feature(rustc_private)]

//! A lint to require doctest examples on functions and methods.
//!
//! It parses normalized rustdoc attributes, respects public or all-function
//! scope, and recognizes Rust code blocks inside the level-one `# Examples`
//! section. The diagnostic identifies the definition whose documentation needs
//! an executable example.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, ImplItem, ImplItemImplKind, ImplItemKind, Item, ItemKind, TraitItem, TraitItemKind,
    attrs::AttributeKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, def_id::LocalDefId, sym};
use serde::Deserialize;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub MISSING_DOCTEST_EXAMPLES,
    Warn,
    "function or method documentation has no doctest example",
    MissingDoctestExamples,
    MissingDoctestExamples::default()
}

/// Function and method scope checked by the lint.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Scope {
    /// Check only functions and methods reachable from another crate.
    #[default]
    Public,
    /// Check all functions and methods.
    All,
}

/// Configuration read from the lint's `dylint.toml` table.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Select which functions and methods require examples.
    scope: Scope,
}

/// Lint pass that retains the configured function and method scope.
#[derive(Debug)]
struct MissingDoctestExamples {
    /// Active scope for this compilation.
    config: Config,
}

impl Default for MissingDoctestExamples {
    /// Construct the default pass from the target workspace configuration.
    fn default() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for MissingDoctestExamples {
    /// Check free functions for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if !matches!(item.kind, ItemKind::Fn { .. }) {
            return;
        }

        check_definition(cx, self.config.scope, item.owner_id.def_id, item.span);
    }

    /// Check trait method declarations for this lint.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        if !matches!(item.kind, TraitItemKind::Fn(..)) {
            return;
        }

        check_definition(cx, self.config.scope, item.owner_id.def_id, item.span);
    }

    /// Check inherent methods for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Trait implementations inherit their API docs from the trait declaration.
        if !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
            || !matches!(item.kind, ImplItemKind::Fn(..))
        {
            return;
        }

        check_definition(cx, self.config.scope, item.owner_id.def_id, item.span);
    }
}

/// Check one function or method definition against the configured scope.
fn check_definition(cx: &LateContext<'_>, scope: Scope, local_def_id: LocalDefId, span: Span) {
    // Generated code has no editable docs, and public scope follows effective visibility.
    if span.from_expansion()
        || matches!(scope, Scope::Public) && !cx.effective_visibilities.is_exported(local_def_id)
    {
        return;
    }

    // Rustdoc omits hidden items, and nobody calls `main` or a test function from a doctest.
    if is_doc_hidden(cx, local_def_id) {
        return;
    }
    if is_entry_function(cx, local_def_id) {
        return;
    }
    if is_test_function(cx, local_def_id) {
        return;
    }

    // Parse normalized attributes so all supported doc-comment forms behave alike.
    let hir_id = cx.tcx.local_def_id_to_hir_id(local_def_id);
    // Keep attribute order because Markdown section boundaries depend on it.
    let docs = cx
        .tcx
        .hir_attrs(hir_id)
        .iter()
        .filter_map(|attr| {
            attr.doc_str()
                .or_else(|| attr.has_name(sym::doc).then(|| attr.value_str()).flatten())
        })
        .map(|text| text.as_str().to_owned())
        .collect::<Vec<_>>()
        .join("\n");
    if !has_doctest_example(&docs) {
        emit_missing_example(cx, span);
    }
}

/// Return whether a definition or one of its ancestors is `#[doc(hidden)]`.
fn is_doc_hidden(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    // Rustdoc omits hidden items and everything inside them, including a hidden crate root.
    let mut current = Some(local_def_id.to_def_id());
    while let Some(ancestor) = current {
        // Check every ancestor because a hidden module hides all descendants.
        if cx.tcx.is_doc_hidden(ancestor) {
            return true;
        }
        current = cx.tcx.opt_parent(ancestor);
    }
    false
}

/// Return whether a definition is the crate entry function.
fn is_entry_function(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    cx.tcx
        .entry_fn(())
        .is_some_and(|(entry, _)| entry == local_def_id.to_def_id())
}

/// Return whether a function is a `#[test]` function in a `--test` build.
fn is_test_function(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
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

/// Return whether `# Examples` contains a non-empty Rust doctest block.
fn has_doctest_example(markdown: &str) -> bool {
    // Let the state machine own heading and code-block transitions separately.
    let mut state = DoctestState::default();
    Parser::new(markdown).any(|event| state.observe(event).is_some())
}

/// Parser state for the active level-one heading and Rust code block.
#[derive(Default)]
struct DoctestState {
    /// Text collected from the current level-one heading.
    heading_text: String,
    /// Whether the parser is currently inside a level-one heading.
    in_h1: bool,
    /// Whether the current level-one section is named `Examples`.
    in_examples: bool,
    /// Whether the parser is currently inside an accepted Rust code block.
    in_rust_block: bool,
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "unrelated Markdown events cannot affect doctest section state"
)]
impl DoctestState {
    /// Consume one Markdown event and return evidence of a qualifying code block.
    fn observe(&mut self, event: Event<'_>) -> Option<()> {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) | Event::Code(text) => self.text(&text),
            _ => None,
        }
    }

    /// Process a start tag that can change section or code-block state.
    fn start(&mut self, tag: Tag<'_>) -> Option<()> {
        // A new level-one heading resets the previous section before collecting text.
        match tag {
            Tag::Heading {
                level: HeadingLevel::H1,
                ..
            } => {
                self.heading_text.clear();
                self.in_h1 = true;
                self.in_examples = false;
                None
            }
            Tag::CodeBlock(kind) if self.in_examples => {
                // Only code blocks inside `# Examples` can satisfy this lint.
                self.in_rust_block = is_rust_doctest(&kind);
                None
            }
            _ => None,
        }
    }

    /// Process an end tag that can close a heading or code block.
    fn end(&mut self, tag: TagEnd) -> Option<()> {
        // Finalize the heading only after all inline text has been collected.
        match tag {
            TagEnd::Heading(HeadingLevel::H1) => {
                self.in_examples = self.heading_text.trim() == "Examples";
                self.in_h1 = false;
                None
            }
            TagEnd::CodeBlock => {
                // A closing fence ends the active Rust example candidate.
                self.in_rust_block = false;
                None
            }
            _ => None,
        }
    }

    /// Append heading text or return evidence from a non-empty Rust block.
    fn text(&mut self, text: &str) -> Option<()> {
        if self.in_h1 {
            self.heading_text.push_str(text);
        }
        (!self.in_h1 && self.in_rust_block && !text.trim().is_empty()).then_some(())
    }
}

/// Return whether rustdoc treats the code block as Rust.
fn is_rust_doctest(kind: &CodeBlockKind<'_>) -> bool {
    // Indented blocks default to Rust; fenced blocks must use only rustdoc attributes.
    match kind {
        CodeBlockKind::Indented => true,
        CodeBlockKind::Fenced(info) => {
            let attributes = info
                .split([',', ' ', '\t'])
                .filter(|attribute| !attribute.is_empty());
            attributes.into_iter().all(is_rust_doctest_attribute)
        }
    }
}

/// Return whether an info-string attribute belongs to rustdoc's Rust grammar.
fn is_rust_doctest_attribute(attribute: &str) -> bool {
    // Keep the accepted vocabulary aligned with stable rustdoc code-block attributes.
    matches!(
        attribute,
        "rust"
            | "no_run"
            | "compile_fail"
            | "should_panic"
            | "ignore"
            | "edition2015"
            | "edition2018"
            | "edition2021"
            | "edition2024"
            | "standalone_crate"
    ) || attribute.starts_with("ignore-")
}

/// Emit the missing-example diagnostic.
fn emit_missing_example(cx: &LateContext<'_>, span: Span) {
    // Point at the definition because its doc attributes can use several source forms.
    cx.emit_span_lint(
        MISSING_DOCTEST_EXAMPLES,
        span,
        DiagDecorator(|diag| {
            let _configured_message =
                diag.primary_message("function or method documentation has no doctest example");
            let _configured_help =
                diag.help("add `# Examples` with a non-empty Rust code block to this item's docs");
        }),
    );
}

/// Run the default public-scope UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui/default");
}

/// Run the configured all-functions UI fixture.
#[test]
fn ui_all_functions() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui/all_functions")
        .dylint_toml("[missing-doctest-examples]\nscope = \"all\"\n")
        .run();
}

#[cfg(test)]
mod tests {
    use super::has_doctest_example;

    /// Accept ordinary and attributed Rust doctest blocks.
    #[test]
    fn accepts_rust_doctest_blocks() {
        // Cover the default fence, rustdoc attributes, and indented Rust blocks.
        assert!(has_doctest_example("# Examples\n\n```\ncall();\n```"));
        assert!(has_doctest_example(
            "# Examples\n\n```rust,no_run\ncall();\n```"
        ));
        assert!(has_doctest_example("# Examples\n\n    call();"));
    }

    /// Reject missing, non-Rust, and empty examples.
    #[test]
    fn rejects_non_doctest_examples() {
        // Distinguish documentation text, another language, and an empty Rust fence.
        assert!(!has_doctest_example("Call `run` to begin."));
        assert!(!has_doctest_example("# Examples\n\n```text\ncall();\n```"));
        assert!(!has_doctest_example("# Examples\n\n```rust\n\n```"));
    }

    /// Keep the example inside the `# Examples` section boundary.
    #[test]
    fn stops_at_the_next_level_one_heading() {
        // A later section and a level-two heading cannot supply the required example.
        assert!(!has_doctest_example(
            "# Examples\n\n# Safety\n\n```\ncall();\n```"
        ));
        assert!(!has_doctest_example("## Examples\n\n```\ncall();\n```"));
    }
}
