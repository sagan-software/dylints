#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for Schemars-specific private lints.
//!
//! The helpers resolve Schemars definitions through rustc metadata and inspect
//! attributes in source order. They keep each lint focused on one public API
//! contract while avoiding source-text guesses for type-aware checks.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::{
    Crate,
    ast::{self, AttrArgs, AttrKind, AttrStyle, Attribute, MetaItem, MetaItemInner, MetaItemKind},
    token::TokenKind,
    tokenstream::TokenTree,
    visit::{Visitor, walk_item},
};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{EarlyContext, LateContext, LintContext as _};
use rustc_middle::ty;
use rustc_span::{
    Span,
    def_id::{DefId, LocalDefId},
};

use dylint_linting as _;

/// One redundant Schemars attribute and its optional safe deletion.
#[derive(Debug)]
pub struct RedundantSerdeAttribute {
    /// Source span used for the lint diagnostic.
    pub diagnostic_span: Span,
    /// Disjoint source spans deleted by an exact machine-applicable fix.
    pub deletion_spans: Option<Vec<Span>>,
}

/// Return spans of Schemars attributes that duplicate matching Serde entries.
///
/// This keeps the existing result shape for callers that need diagnostic spans.
/// Use [`redundant_serde_attributes`] when a lint also needs a safe source edit.
#[must_use]
pub fn redundant_serde_attribute_spans(
    cx: &EarlyContext<'_>,
    krate: &Crate,
    keys: &[&str],
) -> Vec<Span> {
    let mut spans = Vec::new();
    spans.extend(
        redundant_serde_attributes(cx, krate, keys)
            .into_iter()
            .map(|attribute| attribute.diagnostic_span),
    );
    spans
}

/// Return Schemars attributes that duplicate matching Serde entries.
///
/// Attribute keys match exactly, so `rename` does not match `rename_all`.
/// String values compare after Rust unescapes them. The visitor preserves source
/// order and descends into local items, variants, fields, and loaded modules.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, krate| {
///     let _ = schemars_support::redundant_serde_attributes(cx, krate, &["rename"]);
/// };
/// ```
pub fn redundant_serde_attributes(
    cx: &EarlyContext<'_>,
    krate: &Crate,
    keys: &[&str],
) -> Vec<RedundantSerdeAttribute> {
    let keys: Vec<_> = keys
        .iter()
        .filter_map(|key| key.parse::<SerdeAttributeKey>().ok())
        .collect();
    // Walk every item shape so local declarations receive the same check as module items.
    let mut visitor = RedundantSerdeAttributeVisitor {
        cx,
        keys: &keys,
        attributes: Vec::new(),
    };
    for item in &krate.items {
        visitor.visit_item(item);
    }

    visitor.attributes
}

/// The closed set of Serde metadata keys supported by the lint family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SerdeAttributeKey {
    /// Select a Serde default.
    Default,
    /// Reject unknown fields.
    DenyUnknownFields,
    /// Rename a value.
    Rename,
    /// Rename values by a rule.
    RenameAll,
    /// Skip a value in all formats.
    Skip,
    /// Skip serialization.
    SkipSerializing,
    /// Skip deserialization.
    SkipDeserializing,
    /// Skip serialization when a function returns true.
    SkipSerializingIf,
    /// Select an internally tagged representation.
    Tag,
    /// Use a transparent representation.
    Transparent,
}

/// Error returned when a metadata key is outside the supported set.
#[derive(Debug)]
struct UnknownSerdeAttributeKey;

impl std::str::FromStr for SerdeAttributeKey {
    type Err = UnknownSerdeAttributeKey;

    /// Parse one exact Serde metadata key into the supported vocabulary.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "default" => Ok(Self::Default),
            "deny_unknown_fields" => Ok(Self::DenyUnknownFields),
            "rename" => Ok(Self::Rename),
            "rename_all" => Ok(Self::RenameAll),
            "skip" => Ok(Self::Skip),
            "skip_serializing" => Ok(Self::SkipSerializing),
            "skip_deserializing" => Ok(Self::SkipDeserializing),
            "skip_serializing_if" => Ok(Self::SkipSerializingIf),
            "tag" => Ok(Self::Tag),
            "transparent" => Ok(Self::Transparent),
            _ => Err(UnknownSerdeAttributeKey),
        }
    }
}

/// Visit source items and retain their redundant Schemars attributes.
struct RedundantSerdeAttributeVisitor<'cx, 'keys> {
    /// Compiler context used to read source spans and snippets.
    cx: &'cx EarlyContext<'cx>,
    /// Exact keys owned by the lint that invokes this helper.
    keys: &'keys [SerdeAttributeKey],
    /// Matches in source visitation order.
    attributes: Vec<RedundantSerdeAttribute>,
}

impl<'ast> Visitor<'ast> for RedundantSerdeAttributeVisitor<'_, '_> {
    /// Check this item's attributes, then recurse through nested item positions.
    fn visit_item(&mut self, item: &'ast ast::Item) {
        self.attributes
            .extend(redundant_attrs_for_item(self.cx, &item.attrs, self.keys));
        // The compiler walker descends into modules, impls, functions, and block-local items.
        walk_item(self, item);
    }

    /// Check an enum variant before visiting its fields.
    fn visit_variant(&mut self, variant: &'ast ast::Variant) {
        self.attributes
            .extend(redundant_attrs_for_item(self.cx, &variant.attrs, self.keys));
        rustc_ast::visit::walk_variant(self, variant);
    }

    /// Check one struct, union, or variant field.
    fn visit_field_def(&mut self, field: &'ast ast::FieldDef) {
        self.attributes
            .extend(redundant_attrs_for_item(self.cx, &field.attrs, self.keys));
        rustc_ast::visit::walk_field_def(self, field);
    }
}

/// Find matching entries among attributes attached to one item, field, or variant.
fn redundant_attrs_for_item(
    cx: &EarlyContext<'_>,
    attrs: &[Attribute],
    keys: &[SerdeAttributeKey],
) -> Vec<RedundantSerdeAttribute> {
    // Parse each outer attribute once and keep its root metadata beside its source span.
    let parsed: Vec<_> = attrs
        .iter()
        .filter(|attr| matches!(attr.style, AttrStyle::Outer))
        .filter_map(|attr| attr.meta().map(|meta| (attr, meta)))
        .collect();
    // Collect Serde entries from every sibling Serde attribute on this syntax node.
    let serde_entries: Vec<_> = parsed
        .iter()
        .filter(|(_, meta)| meta_name(meta) == Some("serde"))
        .filter_map(|(_, meta)| meta.meta_item_list())
        .flat_map(|items| items.iter().filter_map(MetaItemInner::meta_item))
        .collect();

    let mut redundant = Vec::new();
    for (attr, meta) in &parsed {
        // Only the exact `schemars` namespace can supply the redundant entries.
        if meta_name(meta) != Some("schemars") {
            continue;
        }
        let Some(items) = meta.meta_item_list() else {
            continue;
        };
        let matching_entries: Vec<_> = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let entry = item.meta_item()?;
                let key = meta_name(entry)?.parse::<SerdeAttributeKey>().ok()?;
                let matches_serde = serde_entries
                    .iter()
                    .any(|serde| is_entry_equivalent(key, entry, serde));
                matches_serde.then_some((index, entry, keys.contains(&key)))
            })
            .collect();
        // Do not emit matches owned by another lint in the family.
        if !matching_entries.iter().any(|(_, _, is_target)| *is_target) {
            continue;
        }

        // Multiple redundant entries would give separate lint passes overlapping comma edits.
        let deletion_spans = match matching_entries.as_slice() {
            [(index, entry, _)] => safe_deletion_spans(cx, attr, items, *index, entry.span),
            _ => None,
        };
        redundant.push(RedundantSerdeAttribute {
            diagnostic_span: attr.span,
            deletion_spans,
        });
    }

    redundant
}

/// Return one top-level metadata key only when its path has no qualification or arguments.
fn meta_name(meta: &MetaItem) -> Option<&str> {
    let [segment] = &meta.path.segments[..] else {
        return None;
    };
    segment
        .args
        .is_none()
        .then_some(segment.ident.name.as_str())
}

/// Compare one supported Schemars key using its Serde value grammar.
fn is_entry_equivalent(key: SerdeAttributeKey, left: &MetaItem, right: &MetaItem) -> bool {
    let Some(left_name) = meta_name(left) else {
        return false;
    };
    let Ok(left_key) = left_name.parse::<SerdeAttributeKey>() else {
        return false;
    };
    if left_key != key {
        return false;
    }

    let Some(right_name) = meta_name(right) else {
        return false;
    };
    let Ok(right_key) = right_name.parse::<SerdeAttributeKey>() else {
        return false;
    };
    if right_key != key {
        return false;
    }

    match key {
        SerdeAttributeKey::Rename | SerdeAttributeKey::RenameAll => rename_values(left)
            .zip(rename_values(right))
            .is_some_and(|(left, right)| left == right),
        SerdeAttributeKey::Default => match (&left.kind, &right.kind) {
            (MetaItemKind::Word, MetaItemKind::Word) => true,
            (MetaItemKind::NameValue(_), MetaItemKind::NameValue(_)) => {
                is_string_values_equal(left, right)
            }
            _ => false,
        },
        SerdeAttributeKey::DenyUnknownFields
        | SerdeAttributeKey::Skip
        | SerdeAttributeKey::SkipSerializing
        | SerdeAttributeKey::SkipDeserializing
        | SerdeAttributeKey::Transparent => matches!(
            (&left.kind, &right.kind),
            (MetaItemKind::Word, MetaItemKind::Word)
        ),
        SerdeAttributeKey::SkipSerializingIf | SerdeAttributeKey::Tag => {
            is_string_values_equal(left, right)
        }
    }
}

/// Parse direct or direction-specific names into decoded serialize and
/// deserialize values.
fn rename_values(
    meta: &MetaItem,
) -> Option<(Option<rustc_span::Symbol>, Option<rustc_span::Symbol>)> {
    if matches!(meta.kind, MetaItemKind::NameValue(_)) {
        let value = string_value(meta)?;
        return Some((Some(value), Some(value)));
    }

    let items = meta.meta_item_list()?;
    if items.is_empty() {
        return None;
    }

    let mut serialize = None;
    let mut deserialize = None;
    for item in items {
        let nested = item.meta_item()?;
        let value = string_value(nested)?;
        match meta_name(nested)? {
            "serialize" => {
                if serialize.replace(value).is_some() {
                    return None;
                }
            }
            "deserialize" => {
                if deserialize.replace(value).is_some() {
                    return None;
                }
            }
            _ => return None,
        }
    }

    (serialize.is_some() || deserialize.is_some()).then_some((serialize, deserialize))
}

/// Return a string literal's decoded value when it has no unsupported suffix.
fn string_value(meta: &MetaItem) -> Option<rustc_span::Symbol> {
    let MetaItemKind::NameValue(literal) = &meta.kind else {
        return None;
    };
    literal
        .suffix
        .is_none()
        .then(|| literal.value_as_str())
        .flatten()
}

/// Compare string values after Rust processes escapes and raw delimiters.
fn is_string_values_equal(left: &MetaItem, right: &MetaItem) -> bool {
    string_value(left)
        .zip(string_value(right))
        .is_some_and(|(left, right)| left == right)
}

/// Choose an exact deletion that does not remove comments or another attribute entry.
fn safe_deletion_spans(
    cx: &EarlyContext<'_>,
    attr: &Attribute,
    items: &[MetaItemInner],
    index: usize,
    entry_span: Span,
) -> Option<Vec<Span>> {
    // A comment inside the target entry cannot be preserved by deleting its full source span.
    if snippet_has_comment(cx, attr, entry_span)? {
        return None;
    }

    let attr_has_comment = snippet_has_comment(cx, attr, attr.span)?;
    if items.len() == 1 && !attr_has_comment {
        // Keep the existing whole-attribute fix when it cannot discard a comment.
        let source_map = cx.sess().source_map();
        return Some(vec![source_map.span_extend_while_whitespace(attr.span)]);
    }
    if items.len() == 1 {
        // Remove just the entry when comments elsewhere in the attribute must remain.
        return Some(vec![entry_span]);
    }

    let comma = adjacent_comma_span(attr, items, index, entry_span)?;
    // The entry and comma are separate spans so comments and intervening whitespace survive.
    let mut spans = vec![entry_span, comma];
    spans.sort_by_key(|span| span.lo());
    Some(spans)
}

/// Return whether a source span contains comments between its compiler tokens.
fn snippet_has_comment(cx: &EarlyContext<'_>, attr: &Attribute, span: Span) -> Option<bool> {
    let AttrKind::Normal(normal) = &attr.kind else {
        return None;
    };
    let token_trees = normal
        .tokens
        .as_ref()?
        .to_attr_token_stream()
        .to_token_trees();
    let mut tokens = Vec::new();
    for tree in &token_trees {
        collect_token_spans(tree, &mut tokens);
    }
    tokens.sort_by_key(|(token_span, _)| (token_span.lo(), token_span.hi()));

    let mut cursor = span.lo();
    for (token_span, is_doc_comment) in tokens {
        if token_span.hi() <= span.lo() || token_span.lo() >= span.hi() {
            continue;
        }
        // A partial token overlap or synthetic span cannot support a safe source edit.
        if token_span.from_expansion()
            || token_span.lo() < span.lo()
            || token_span.hi() > span.hi()
            || token_span.lo() < cursor
        {
            return None;
        }
        if source_gap_has_comment(cx, span.with_lo(cursor).with_hi(token_span.lo()))? {
            return Some(true);
        }
        if is_doc_comment {
            return Some(true);
        }
        cursor = token_span.hi();
    }

    source_gap_has_comment(cx, span.with_lo(cursor).with_hi(span.hi()))
}

/// Record token spans, including delimiter tokens and doc comments.
fn collect_token_spans(tree: &TokenTree, spans: &mut Vec<(Span, bool)>) {
    match tree {
        TokenTree::Token(token, _) => {
            spans.push((token.span, matches!(token.kind, TokenKind::DocComment(..))));
        }
        TokenTree::Delimited(delim_span, _, _, nested) => {
            spans.push((delim_span.open, false));
            for nested_tree in nested.iter() {
                collect_token_spans(nested_tree, spans);
            }
            spans.push((delim_span.close, false));
        }
    }
}

/// Return whether one source gap contains a Rust line or block comment.
fn source_gap_has_comment(cx: &EarlyContext<'_>, gap: Span) -> Option<bool> {
    if gap.is_empty() {
        return Some(false);
    }
    let snippet = cx.sess().source_map().span_to_snippet(gap).ok()?;
    Some(snippet.contains("//") || snippet.contains("/*"))
}

/// Find a top-level comma adjacent to one list entry, outside nested value lists.
fn adjacent_comma_span(
    attr: &Attribute,
    items: &[MetaItemInner],
    index: usize,
    entry_span: Span,
) -> Option<Span> {
    let AttrKind::Normal(normal) = &attr.kind else {
        return None;
    };
    let Some(AttrArgs::Delimited(args)) = normal.item.args.unparsed_ref() else {
        return None;
    };
    let comma_spans: Vec<_> = args
        .tokens
        .iter()
        .filter_map(|tree| match tree {
            TokenTree::Token(token, _) if token.kind == TokenKind::Comma => Some(token.span),
            TokenTree::Token(..) | TokenTree::Delimited(..) => None,
        })
        .collect();

    // Prefer the following separator, including a legal trailing comma.
    if let Some(comma) = comma_spans
        .iter()
        .find(|comma| entry_span.hi() <= comma.lo())
    {
        return Some(*comma);
    }

    // The final entry without a trailing comma must remove its preceding separator.
    let previous_span = meta_inner_span(items.get(index.checked_sub(1)?)?);
    comma_spans
        .iter()
        .rev()
        .find(|comma| previous_span.hi() <= comma.lo() && comma.hi() <= entry_span.lo())
        .copied()
}

/// Return the source span of one argument in a delimited attribute list.
const fn meta_inner_span(item: &MetaItemInner) -> Span {
    match item {
        MetaItemInner::MetaItem(meta) => meta.span,
        MetaItemInner::Lit(literal) => literal.span,
    }
}

/// Declare one redundant Schemars-over-Serde attribute lint.
#[macro_export]
macro_rules! declare_redundant_serde_attribute_lint {
    ($lint:ident, $pass:ident, [$($key:literal),+ $(,)?], $description:literal) => {
        dylint_support::documented_early_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl rustc_lint::EarlyLintPass for $pass {
            /// Check item, variant, and field attributes in the crate AST.
            fn check_crate(&mut self, cx: &rustc_lint::EarlyContext<'_>, krate: &rustc_ast::Crate) {
                for attribute in $crate::redundant_serde_attributes(cx, krate, &[$($key),+]) {
                    let span = attribute.diagnostic_span;
                    let deletion_spans = attribute.deletion_spans;
                    cx.emit_span_lint(
                        $lint,
                        span,
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let diagnostic = diagnostic
                                .primary_message("this Schemars attribute duplicates Serde");
                            if span.from_expansion() {
                                let _configured_help =
                                    diagnostic.help("remove the redundant Schemars attribute");
                            } else if let Some(deletion_spans) = deletion_spans {
                                if deletion_spans.len() == 1 {
                                    let _configured_suggestion = diagnostic.span_suggestion(
                                        deletion_spans[0],
                                        "remove the redundant Schemars attribute",
                                        String::new(),
                                        rustc_errors::Applicability::MachineApplicable,
                                    );
                                } else {
                                    let parts = deletion_spans
                                        .into_iter()
                                        .map(|span| (span, String::new()))
                                        .collect();
                                    let _configured_suggestion = diagnostic.multipart_suggestion(
                                        "remove the redundant Schemars entry",
                                        parts,
                                        rustc_errors::Applicability::MachineApplicable,
                                    );
                                }
                            } else {
                                let _configured_help =
                                    diagnostic.help("remove only the matching Schemars entry");
                            }
                        }),
                    );
                }
            }
        }

        /// Run the UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Return whether a trait definition is Schemars's `JsonSchema`.
///
/// Resolution is crate-aware, so a local trait with the same name cannot produce
/// a false positive for a lint that targets the external Schemars contract.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, def_id| {
///     let _ = schemars_support::is_json_schema_trait(cx, def_id);
/// };
/// ```
pub fn is_json_schema_trait(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx.crate_name(def_id.krate).as_str() == "schemars"
        && cx.tcx.item_name(def_id).as_str() == "JsonSchema"
}

/// Return whether an expression calls one exact Schemars method.
///
/// The method name and defining crate must both match, which excludes unrelated
/// extension traits and local methods that happen to use the same spelling.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_name| {
///     let _ = schemars_support::is_schemars_method_call(cx, expr, expected_name);
/// };
/// ```
pub fn is_schemars_method_call(cx: &LateContext<'_>, expr: &Expr<'_>, expected_name: &str) -> bool {
    // Reject non-method expressions before requesting type-dependent resolution.
    let ExprKind::MethodCall(segment, ..) = expr.kind else {
        return false;
    };
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    let Some(def_id) = typeck.type_dependent_def_id(expr.hir_id) else {
        return false;
    };

    // Require both the target method name and the defining Schemars crate.
    segment.ident.name.as_str() == expected_name
        && cx.tcx.crate_name(def_id.krate).as_str() == "schemars"
}

/// Return whether a call expression resolves to one exact Schemars associated function.
///
/// Only direct path calls are considered because closures and method calls have
/// different resolution contracts and are handled by separate helpers.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_name| {
///     let _ = schemars_support::is_schemars_function_call(cx, expr, expected_name);
/// };
/// ```
pub fn is_schemars_function_call(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    expected_name: &str,
) -> bool {
    // Resolve only direct call paths because closures and method calls use other contracts.
    let ExprKind::Call(callee, _) = expr.kind else {
        return false;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return false;
    };
    let rustc_hir::def::Res::Def(_, def_id) = cx
        .tcx
        .typeck(callee.hir_id.owner.def_id)
        .qpath_res(path, callee.hir_id)
    else {
        return false;
    };

    // Reject local functions that reuse the same associated-function name.
    cx.tcx.crate_name(def_id.krate).as_str() == "schemars"
        && cx.tcx.item_name(def_id).as_str() == expected_name
}

/// Collect local ADTs with a semantically resolved `JsonSchema` implementation.
///
/// The result contains unique local definition identifiers in traversal order,
/// allowing callers to inspect only application types that implement the target.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = schemars_support::local_json_schema_impls(cx);
/// };
/// ```
pub fn local_json_schema_impls(cx: &LateContext<'_>) -> Vec<LocalDefId> {
    // Accumulate unique local types while walking all local trait implementations.
    let mut implementations = Vec::new();

    // Ignore every trait except Schemars's resolved JsonSchema definition.
    for (&trait_def_id, impl_def_ids) in cx.tcx.all_local_trait_impls(()) {
        if !is_json_schema_trait(cx, trait_def_id) {
            continue;
        }

        // Retain only local ADT self types because external types cannot be lint targets.
        for &impl_def_id in impl_def_ids {
            if !matches!(
                cx.tcx.def_kind(impl_def_id),
                rustc_hir::def::DefKind::Impl { .. }
            ) {
                continue;
            }
            let self_ty = cx
                .tcx
                .type_of(impl_def_id)
                .instantiate_identity()
                .skip_norm_wip();
            let ty::Adt(adt, _) = self_ty.kind() else {
                continue;
            };
            if let Some(local_def_id) = adt.did().as_local()
                && !implementations.contains(&local_def_id)
            {
                implementations.push(local_def_id);
            }
        }
    }

    implementations
}
