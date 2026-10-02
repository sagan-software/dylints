#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for clap-specific private lints.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use std::{fs::File, io::Read, str::FromStr};

use rustc_ast::{
    Attribute, Crate, FieldDef, GenericArg, Item, ItemKind, LitKind, MetaItem, MetaItemInner, Ty,
    TyKind, Variant,
    visit::{Visitor, walk_item},
};
use rustc_hir::{Expr, ExprKind, Node, def::Res};
use rustc_lint::{EarlyContext, LateContext, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, Symbol, sym};

/// One Clap derive macro recognized by the derive reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClapDerive {
    /// `Parser`.
    Parser,
    /// `Args`.
    Args,
    /// `Subcommand`.
    Subcommand,
    /// `ValueEnum`.
    ValueEnum,
}

impl FromStr for ClapDerive {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Parse the closed external spelling directly into its semantic enum.
        if value == "Parser" {
            Ok(Self::Parser)
        } else if value == "Args" {
            Ok(Self::Args)
        } else if value == "Subcommand" {
            Ok(Self::Subcommand)
        } else if value == "ValueEnum" {
            Ok(Self::ValueEnum)
        } else {
            // Reject every spelling outside the public derive vocabulary.
            Err(())
        }
    }
}

/// The distinct Clap derive macros applied to one item.
#[derive(Clone, Debug, Default)]
pub struct ClapDerives {
    /// Present derive macros, with duplicates removed.
    derives: Vec<ClapDerive>,
}

impl ClapDerives {
    /// Return whether an item participates in Clap's derive API.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = clap_support::ClapDerives::has_clap(value);
    /// };
    /// ```
    pub const fn has_clap(&self) -> bool {
        !self.derives.is_empty()
    }

    /// Return whether fields on this item define command-line arguments.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = clap_support::ClapDerives::has_arg_fields(value);
    /// };
    /// ```
    pub fn has_arg_fields(&self) -> bool {
        self.has_derive(ClapDerive::Parser)
            || self.has_derive(ClapDerive::Args)
            || self.has_derive(ClapDerive::Subcommand)
    }

    /// Record a derive once.
    fn insert(&mut self, derive: ClapDerive) {
        if !self.has_derive(derive) {
            self.derives.push(derive);
        }
    }

    /// Return whether one derive is present.
    fn has_derive(&self, derive: ClapDerive) -> bool {
        self.derives.contains(&derive)
    }
}

/// One cfg-active struct or enum using a Clap derive macro.
#[derive(Debug)]
pub struct ClapAstItem<'ast> {
    /// Derive macros applied to the item.
    pub derives: ClapDerives,
    /// Outer attributes applied to the item.
    pub attrs: &'ast [Attribute],
    /// Direct struct fields, or an empty slice for enums.
    pub fields: Vec<ClapAstField<'ast>>,
    /// Enum variants, or an empty slice for structs.
    pub variants: Vec<ClapAstVariant<'ast>>,
}

/// One field inside a Clap-derived item.
#[derive(Debug)]
pub struct ClapAstField<'ast> {
    /// Field type used by Clap's type-driven inference.
    pub ty: &'ast Ty,
    /// Full field span used for diagnostics.
    pub span: Span,
    /// Outer attributes applied to the field.
    pub attrs: &'ast [Attribute],
}

/// One variant inside a Clap-derived enum.
#[derive(Debug)]
pub struct ClapAstVariant<'ast> {
    /// Full variant span used for diagnostics.
    pub span: Span,
    /// Outer attributes applied to the variant.
    pub attrs: &'ast [Attribute],
    /// Fields declared by the variant.
    pub fields: Vec<ClapAstField<'ast>>,
}

/// Clap's documented syntactic field-type classifications.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClapFieldType {
    /// A `bool` flag.
    Bool,
    /// An `Option<T>` argument.
    Option,
    /// An `Option<Option<T>>` argument with an optional value.
    OptionOption,
    /// A `Vec<T>` repeated argument.
    Vec,
    /// An `Option<Vec<T>>` repeated argument.
    OptionVec,
    /// A `Vec<Vec<T>>` argument grouped by occurrence.
    VecVec,
    /// An `Option<Vec<Vec<T>>>` argument grouped by occurrence.
    OptionVecVec,
    /// Any field type without one of Clap's recognized wrappers.
    Value,
}

impl ClapFieldType {
    /// Return the action Clap infers for this exact field-type spelling.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = clap_support::ClapFieldType::inferred_action(value);
    /// };
    /// ```
    pub const fn inferred_action(self) -> &'static str {
        match self {
            Self::Bool => "SetTrue",
            Self::Vec | Self::OptionVec | Self::VecVec | Self::OptionVecVec => "Append",
            Self::Option | Self::OptionOption | Self::Value => "Set",
        }
    }

    /// Return whether the type groups values by argument occurrence.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = clap_support::ClapFieldType::has_grouped_occurrences(value);
    /// };
    /// ```
    pub const fn has_grouped_occurrences(self) -> bool {
        matches!(self, Self::VecVec | Self::OptionVecVec)
    }
}

/// Collect all cfg-active items using Clap's derive API.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, krate| {
///     let _ = clap_support::clap_ast_items(cx, krate);
/// };
/// ```
pub fn clap_ast_items<'ast>(cx: &EarlyContext<'_>, krate: &'ast Crate) -> Vec<ClapAstItem<'ast>> {
    // Walk cfg-active items in source order and retain only Clap-derived definitions.
    let mut collector = ClapAstCollector {
        cx,
        items: Vec::new(),
    };

    for item in &krate.items {
        collector.visit_item(item);
    }

    collector.items
}

/// Visit each argument field on a Clap-derived item.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |item| {
///     let _ = clap_support::clap_arg_fields(item);
/// };
/// ```
pub fn clap_arg_fields<'ast, 'item>(
    item: &'item ClapAstItem<'ast>,
) -> impl Iterator<Item = &'item ClapAstField<'ast>> {
    item.fields.iter().chain(
        item.variants
            .iter()
            .flat_map(|variant| variant.fields.iter()),
    )
}

/// Visitor that retains only source items using Clap derives.
struct ClapAstCollector<'ast, 'cx> {
    /// Lint context used for source-map fallbacks.
    cx: &'cx EarlyContext<'cx>,
    /// Collected Clap-derived items.
    items: Vec<ClapAstItem<'ast>>,
}

impl std::fmt::Debug for ClapAstCollector<'_, '_> {
    /// Format collector state without exposing the lint context.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClapAstCollector")
            .field("items", &self.items)
            .finish_non_exhaustive()
    }
}

impl<'ast> Visitor<'ast> for ClapAstCollector<'ast, '_> {
    /// Collect one cfg-active struct or enum before walking nested items.
    fn visit_item(&mut self, item: &'ast Item) {
        // Prefer structured derive metadata and fall back to original source when expansion consumed it.
        let mut derives = clap_derives(self.cx, &item.attrs);
        if !derives.has_clap() {
            derives = source_clap_derives(self.cx, item.span);
        }
        if !derives.has_clap() && item_has_clap_helper(item) {
            // Custom derives are expanded before this pass and consume the `derive`
            // entry. Their registered helper attributes remain on the source item.
            derives.insert(ClapDerive::Parser);
        }
        if derives.has_clap() {
            // Store the field and variant shape needed by downstream lint rules.
            if let ItemKind::Struct(_, _, data) = &item.kind {
                self.items.push(ClapAstItem {
                    derives,
                    attrs: &item.attrs,
                    fields: ast_fields(data.fields()),
                    variants: Vec::new(),
                });
            } else if let ItemKind::Enum(_, _, enum_definition) = &item.kind {
                self.items.push(ClapAstItem {
                    derives,
                    attrs: &item.attrs,
                    fields: Vec::new(),
                    variants: enum_definition.variants.iter().map(ast_variant).collect(),
                });
            }
        }

        // Continue into nested modules after recording the current item.
        walk_item(self, item);
    }
}

/// Return whether an item retains a helper attribute registered by Clap derive.
fn item_has_clap_helper(item: &Item) -> bool {
    // Item-level helpers apply before helpers attached to fields or variants.
    if has_clap_helper_attr(&item.attrs) {
        return true;
    }

    // Inspect only aggregate shapes that can contain derive helper attributes.
    if let ItemKind::Struct(_, _, data) = &item.kind {
        data.fields()
            .iter()
            .any(|field| has_clap_helper_attr(&field.attrs))
    } else if let ItemKind::Enum(_, _, enum_definition) = &item.kind {
        enum_definition.variants.iter().any(|variant| {
            has_clap_helper_attr(&variant.attrs)
                || variant
                    .data
                    .fields()
                    .iter()
                    .any(|field| has_clap_helper_attr(&field.attrs))
        })
    } else {
        false
    }
}

/// Return whether attributes include one of Clap derive's helper namespaces.
fn has_clap_helper_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        ["command", "arg", "group", "value", "clap"]
            .into_iter()
            .any(|name| attr.has_name(Symbol::intern(name)))
    })
}

/// Recover Clap derive flags from one item's structured attributes and source.
fn clap_derives(cx: &EarlyContext<'_>, attrs: &[Attribute]) -> ClapDerives {
    // Deduplicate recognized derive kinds while examining every derive attribute.
    let mut derives = ClapDerives::default();

    for attr in attrs.iter().filter(|attr| attr.has_name(sym::derive)) {
        // Use structured meta items when rustc retained the derive arguments.
        if let Some(arguments) = attr.meta_item_list() {
            for argument in arguments {
                let Some(meta) = argument.meta_item() else {
                    continue;
                };
                record_derive_name(meta_path_last_name(meta), &mut derives);
            }
        }

        let Ok(source) = cx.sess().source_map().span_to_snippet(attr.span) else {
            continue;
        };
        // Scan source tokens as a fallback for custom derive expansion shapes.
        for token in
            source.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        {
            record_derive_name(Some(token), &mut derives);
        }
    }

    derives
}

/// Recover derives from the original source after macro expansion consumed them.
fn source_clap_derives(cx: &EarlyContext<'_>, item_span: Span) -> ClapDerives {
    // Recover only the source segment immediately preceding the expanded item.
    let mut derives = ClapDerives::default();
    let Some(segment) = source_segment_before_span(cx, item_span) else {
        return derives;
    };
    let Some(derive_start) = segment.rfind("#[derive") else {
        return derives;
    };
    // Isolate the nearest derive attribute to avoid unrelated earlier tokens.
    let Some(derive_source) = segment.get(derive_start..) else {
        return derives;
    };

    // Normalize path punctuation and record each recognized final segment once.
    for token in derive_source
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
    {
        record_derive_name(Some(token), &mut derives);
    }

    derives
}

/// Record one final derive path segment.
fn record_derive_name(name: Option<&str>, derives: &mut ClapDerives) {
    let Some(name) = name else {
        return;
    };

    if let Ok(derive) = name.parse() {
        derives.insert(derive);
    }
}

/// Convert rustc AST fields into compact records.
fn ast_fields(fields: &[FieldDef]) -> Vec<ClapAstField<'_>> {
    fields
        .iter()
        .map(|field| ClapAstField {
            ty: &field.ty,
            span: field.span,
            attrs: &field.attrs,
        })
        .collect()
}

/// Convert one rustc AST enum variant into a compact record.
fn ast_variant(variant: &Variant) -> ClapAstVariant<'_> {
    ClapAstVariant {
        span: variant.span,
        attrs: &variant.attrs,
        fields: ast_fields(variant.data.fields()),
    }
}

/// Find a named Clap derive-helper attribute entry.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attrs, attribute_name, key| {
///     let _ = clap_support::ast_clap_attr(cx, attrs, attribute_name, key);
/// };
/// ```
pub fn ast_clap_attr<'attr>(
    cx: &EarlyContext<'_>,
    attrs: &'attr [Attribute],
    attribute_name: &str,
    key: &str,
) -> Option<&'attr Attribute> {
    attrs.iter().find(|attr| {
        attr.has_name(Symbol::intern(attribute_name)) && ast_attr_has_word(cx, attr, key)
    })
}

/// Find a Clap helper attribute containing exactly one named entry.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attrs, attribute_name, key| {
///     let _ = clap_support::ast_clap_attr_single_entry(cx, attrs, attribute_name, key);
/// };
/// ```
pub fn ast_clap_attr_single_entry<'attr>(
    cx: &EarlyContext<'_>,
    attrs: &'attr [Attribute],
    attribute_name: &str,
    key: &str,
) -> Option<&'attr Attribute> {
    let attr = attrs
        .iter()
        .find(|attr| attr.has_name(Symbol::intern(attribute_name)))?;
    if let Some(arguments) = attr.meta_item_list()
        && let [argument] = arguments.as_slice()
        && meta_inner_key(argument) == Some(key)
    {
        return Some(attr);
    }
    let source = cx.sess().source_map().span_to_snippet(attr.span).ok()?;
    (has_one_top_level_entry(&source) && attr_entry_source(&source, key).is_some()).then_some(attr)
}

/// Return whether a named Clap derive-helper attribute contains a key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attrs, attribute_name, key| {
///     let _ = clap_support::ast_has_clap_attr(cx, attrs, attribute_name, key);
/// };
/// ```
pub fn ast_has_clap_attr(
    cx: &EarlyContext<'_>,
    attrs: &[Attribute],
    attribute_name: &str,
    key: &str,
) -> bool {
    ast_clap_attr(cx, attrs, attribute_name, key).is_some()
}

/// Return the source for one named entry in a Clap derive-helper attribute.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attrs, attribute_name, key| {
///     let _ = clap_support::ast_clap_attr_entry_source(cx, attrs, attribute_name, key);
/// };
/// ```
pub fn ast_clap_attr_entry_source(
    cx: &EarlyContext<'_>,
    attrs: &[Attribute],
    attribute_name: &str,
    key: &str,
) -> Option<String> {
    let attr = ast_clap_attr(cx, attrs, attribute_name, key)?;

    if let Some(arguments) = attr.meta_item_list() {
        for argument in arguments {
            let Some(meta) = argument.meta_item() else {
                continue;
            };
            if meta_path_last_name(meta) == Some(key)
                && let Ok(source) = cx.sess().source_map().span_to_snippet(meta.span)
            {
                return Some(source);
            }
        }
    }

    // Derive helper metadata can be thin during early linting. Keep the fallback
    // bounded to the matching attribute and return only the matching entry.
    let source = cx.sess().source_map().span_to_snippet(attr.span).ok()?;
    attr_entry_source(&source, key).map(str::to_owned)
}

/// Return whether one set of attributes still includes a `doc` attribute.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attrs| {
///     let _ = clap_support::ast_has_doc(attrs);
/// };
/// ```
pub fn ast_has_doc(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.has_name(sym::doc))
}

/// Return whether original source before a target contains a doc comment.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, span| {
///     let _ = clap_support::source_has_doc_before(cx, span);
/// };
/// ```
pub fn source_has_doc_before(cx: &EarlyContext<'_>, span: Span) -> bool {
    source_segment_before_span(cx, span).is_some_and(|source| {
        source
            .as_bytes()
            .windows(3)
            .any(|window| matches!(window, [b'/', b'/', b'/'] | [b'/', b'*', b'*']))
            || source.contains("#[doc")
    })
}

/// Classify a field type using Clap's documented exact syntactic inference.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |ty| {
///     let _ = clap_support::clap_field_type(ty);
/// };
/// ```
pub fn clap_field_type(ty: &Ty) -> ClapFieldType {
    // Classify only Clap's documented Option and Vec wrapper combinations.
    let Some((outer_name, outer_type)) = one_type_wrapper(ty) else {
        return if exact_type_name(ty) == Some("bool") {
            ClapFieldType::Bool
        } else {
            ClapFieldType::Value
        };
    };

    match outer_name {
        "Option" => match one_type_wrapper(outer_type) {
            Some(("Option", _)) => ClapFieldType::OptionOption,
            Some(("Vec", vector_type)) => match one_type_wrapper(vector_type) {
                Some(("Vec", _)) => ClapFieldType::OptionVecVec,
                _ => ClapFieldType::OptionVec,
            },
            _ => ClapFieldType::Option,
        },
        "Vec" => match one_type_wrapper(outer_type) {
            Some(("Vec", _)) => ClapFieldType::VecVec,
            _ => ClapFieldType::Vec,
        },
        _ => ClapFieldType::Value,
    }
}

/// Return the type passed to Clap's inferred `value_parser!` call.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |ty| {
///     let _ = clap_support::clap_value_parser_type(ty);
/// };
/// ```
pub fn clap_value_parser_type(mut ty: &Ty) -> &Ty {
    // Peel every inferred Option or Vec wrapper in outer-to-inner order.
    loop {
        let Some((name, inner)) = one_type_wrapper(ty) else {
            return ty;
        };
        if matches!(name, "Option" | "Vec") {
            ty = inner;
        } else {
            // Stop at the first wrapper that Clap does not infer through.
            return ty;
        }
    }
}

/// Return source with insignificant whitespace removed.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source| {
///     let _ = clap_support::compact_source(source);
/// };
/// ```
pub fn compact_source(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// Return source for an AST type.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, ty| {
///     let _ = clap_support::ast_type_source(cx, ty);
/// };
/// ```
pub fn ast_type_source(cx: &EarlyContext<'_>, ty: &Ty) -> Option<String> {
    cx.sess().source_map().span_to_snippet(ty.span).ok()
}

/// Return whether a field is handled specially rather than as a regular argument.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attrs| {
///     let _ = clap_support::ast_is_special_clap_field(cx, attrs);
/// };
/// ```
pub fn ast_is_special_clap_field(cx: &EarlyContext<'_>, attrs: &[Attribute]) -> bool {
    // Check command-level field roles before ordinary argument helpers.
    ["flatten", "subcommand"]
        .into_iter()
        .any(|key| ast_has_clap_attr(cx, attrs, "command", key))
        || ["skip", "from_global"]
            .into_iter()
            .any(|key| ast_has_clap_attr(cx, attrs, "arg", key))
        // Group skipping also removes a field from ordinary argument handling.
        || ast_has_clap_attr(cx, attrs, "group", "skip")
}

/// Return a single exact wrapper name and its type argument.
fn one_type_wrapper(ty: &Ty) -> Option<(&str, &Ty)> {
    // Accept one unqualified path with exactly one type argument.
    let TyKind::Path(_, path) = &ty.kind else {
        return None;
    };
    let [segment] = path.segments.as_slice() else {
        return None;
    };
    // Require angle brackets with one type argument and no lifetime or const arguments.
    let arguments = segment.args.as_deref()?;
    let rustc_ast::GenericArgs::AngleBracketed(arguments) = arguments else {
        return None;
    };
    let [rustc_ast::AngleBracketedArg::Arg(GenericArg::Type(inner))] = arguments.args.as_slice()
    else {
        return None;
    };

    Some((segment.ident.name.as_str(), inner))
}

/// Return the name of one unqualified type without generic arguments.
fn exact_type_name(ty: &Ty) -> Option<&str> {
    // Reject qualified or generic paths before returning the sole segment.
    let TyKind::Path(_, path) = &ty.kind else {
        return None;
    };
    let [segment] = path.segments.as_slice() else {
        return None;
    };
    segment.args.is_none().then(|| segment.ident.name.as_str())
}

/// Return true when an AST attribute contains a word-like key.
fn ast_attr_has_word(cx: &EarlyContext<'_>, attr: &Attribute, key: &str) -> bool {
    // Prefer structured metadata when rustc preserved the helper arguments.
    if let Some(arguments) = attr.meta_item_list()
        && arguments
            .iter()
            .any(|argument| meta_inner_key(argument) == Some(key))
    {
        return true;
    }

    let Ok(source) = cx.sess().source_map().span_to_snippet(attr.span) else {
        return false;
    };
    attr_entry_source(&source, key).is_some()
}

/// Return a meta-list entry's final path segment.
fn meta_inner_key(inner: &MetaItemInner) -> Option<&str> {
    meta_path_last_name(inner.meta_item()?)
}

/// Return a meta item's final path segment.
fn meta_path_last_name(meta: &MetaItem) -> Option<&str> {
    meta.path
        .segments
        .iter()
        .next_back()
        .map(|segment| segment.ident.name.as_str())
}

/// Find one top-level attribute entry by exact key.
fn attr_entry_source<'source>(source: &'source str, key: &str) -> Option<&'source str> {
    // Isolate the attribute argument text before scanning individual entries.
    let arguments_start = source.find('(')? + 1;
    let arguments_end = source.rfind(')')?;
    let arguments = source.get(arguments_start..arguments_end)?;
    let mut entry_start = 0;
    let mut delimiter_depth = 0_u32;
    // Track quoted strings separately so commas inside them remain data.
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in arguments.char_indices() {
        // Consume escapes and the closing quote without changing group depth.
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        // Split commas only when no nested delimiter is open.
        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => delimiter_depth = delimiter_depth.saturating_add(1),
            ')' | ']' | '}' => delimiter_depth = delimiter_depth.saturating_sub(1),
            ',' if delimiter_depth == 0 => {
                let entry = arguments.get(entry_start..index)?.trim();
                if entry_has_key(entry, key) {
                    return Some(entry);
                }
                entry_start = index + character.len_utf8();
            }
            _ => {}
        }
    }

    // Check the final entry because it has no trailing comma.
    let entry = arguments.get(entry_start..)?.trim();
    entry_has_key(entry, key).then_some(entry)
}

/// Return whether an attribute argument list contains no top-level comma.
fn has_one_top_level_entry(source: &str) -> bool {
    let Some(arguments_start) = source.find('(') else {
        return false;
    };
    let Some(arguments_end) = source.rfind(')') else {
        return false;
    };
    let Some(arguments) = source.get(arguments_start + 1..arguments_end) else {
        return false;
    };
    let mut delimiter_depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    for character in arguments.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => delimiter_depth = delimiter_depth.saturating_add(1),
            ')' | ']' | '}' => delimiter_depth = delimiter_depth.saturating_sub(1),
            ',' if delimiter_depth == 0 => return false,
            _ => {}
        }
    }
    !arguments.trim().is_empty()
}

/// Return whether an attribute entry starts with an exact key.
fn entry_has_key(entry: &str, key: &str) -> bool {
    let entry = entry.trim_start();
    let Some(rest) = entry.strip_prefix(key) else {
        return false;
    };
    rest.chars()
        .next()
        .is_some_and(|character| !character.is_ascii_alphanumeric() && character != '_')
        || rest.is_empty()
}

/// Load source preceding a span, bounded by the previous structural delimiter.
fn source_segment_before_span(cx: &EarlyContext<'_>, span: Span) -> Option<String> {
    // Resolve the source file and convert the global span to a local byte offset.
    let files = cx.sess().source_map().files();
    let source_file = files.iter().find(|source_file| {
        span.lo() >= source_file.start_pos && span.lo() <= source_file.end_position()
    })?;
    let path = source_file.name.clone().into_local_path()?;
    let local_offset = span.lo().0.checked_sub(source_file.start_pos.0)? as usize;
    drop(files);

    // Read the file after releasing the source-map lock.
    let mut source = String::new();
    let mut file = File::open(path).ok()?;
    let _bytes_read = file.read_to_string(&mut source).ok()?;
    let prefix = source.get(..local_offset)?;
    // Bound the fallback at the nearest structural delimiter before the span.
    let boundary = prefix
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            matches!(character, '{' | '}' | ';' | ',').then_some(index + character.len_utf8())
        })
        .unwrap_or(0);

    prefix.get(boundary..).map(str::to_owned)
}

/// A clap builder type that owns fluent configuration methods.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderType {
    /// `clap::Arg`.
    Arg,
    /// `clap::Command`.
    Command,
}

impl BuilderType {
    /// Return the resolved clap type name.
    const fn item_name(self) -> &'static str {
        match self {
            Self::Arg => "Arg",
            Self::Command => "Command",
        }
    }
}

/// One resolved method in a fluent clap builder chain.
#[derive(Clone, Copy, Debug)]
pub struct BuilderCall<'hir> {
    /// Resolved method name.
    pub method: Symbol,
    /// User-facing method-name span.
    pub span: Span,
    /// Explicit arguments, excluding the receiver.
    pub args: &'hir [Expr<'hir>],
}

/// Return whether this expression is the outermost method in its fluent chain.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = clap_support::is_outermost_builder_call(cx, expr);
/// };
/// ```
pub fn is_outermost_builder_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    !cx.tcx
        .hir_parent_iter(expr.hir_id)
        .next()
        .is_some_and(|(_, node)| {
            matches!(
                node,
                Node::Expr(parent)
                    if matches!(
                        parent.kind,
                        ExprKind::MethodCall(_, receiver, _, _)
                            if receiver.hir_id == expr.hir_id
                    )
            )
        })
}

/// Collect resolved clap builder calls from the constructor outward.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, builder_type| {
///     let _ = clap_support::builder_calls(cx, expr, builder_type);
/// };
/// ```
pub fn builder_calls<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    builder_type: BuilderType,
) -> Vec<BuilderCall<'tcx>> {
    // Accumulate calls from the chain's outer expression toward its constructor.
    let mut calls = Vec::new();
    let mut current = expr;

    // Walk receivers inward, retaining only methods resolved on the target clap type.
    while let ExprKind::MethodCall(segment, receiver, args, _) = current.kind {
        if !is_builder_method(cx, current, receiver, builder_type) {
            break;
        }
        calls.push(BuilderCall {
            method: segment.ident.name,
            span: segment.ident.span,
            args,
        });
        current = receiver;
    }

    // Restore constructor-to-outer order for policy checks.
    calls.reverse();
    calls
}

/// Return a literal boolean argument when the call has exactly one.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |call| {
///     let _ = clap_support::bool_argument(call);
/// };
/// ```
pub const fn bool_argument(call: BuilderCall<'_>) -> Option<bool> {
    // Require one literal boolean and reject computed expressions.
    let [argument] = call.args else {
        return None;
    };
    let ExprKind::Lit(literal) = argument.kind else {
        return None;
    };

    // Preserve only the boolean literal variant after expression-shape validation.
    let LitKind::Bool(value) = literal.node else {
        return None;
    };
    Some(value)
}

/// Return whether an expression resolves to the target clap `ValueHint` variant.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, variant_name| {
///     let _ = clap_support::is_value_hint_variant(cx, expr, variant_name);
/// };
/// ```
pub fn is_value_hint_variant(cx: &LateContext<'_>, expr: &Expr<'_>, variant_name: &str) -> bool {
    // Resolve a path expression before comparing its owning crate and variant name.
    let ExprKind::Path(ref path) = expr.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, expr.hir_id) else {
        return false;
    };

    cx.tcx.crate_name(def_id.krate).as_str() == "clap_builder"
        && cx.tcx.item_name(def_id).as_str() == variant_name
}

/// Return a trigger span when one outermost builder chain lacks a required method.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, builder_type, trigger, is_trigger_true_required, required, is_requirement_true_required| {
///     let _ = clap_support::builder_call_without_requirement(cx, expr, builder_type, trigger, is_trigger_true_required, required, is_requirement_true_required);
/// };
/// ```
pub fn builder_call_without_requirement<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    builder_type: BuilderType,
    trigger: &str,
    is_trigger_true_required: bool,
    required: &str,
    is_requirement_true_required: bool,
) -> Option<Span> {
    // Analyze each fluent chain once at its outermost call.
    if !is_outermost_builder_call(cx, expr) {
        return None;
    }
    let calls = builder_calls(cx, expr, builder_type);
    let trigger_call = calls.iter().copied().find(|call| {
        call.method.as_str() == trigger
            && (!is_trigger_true_required || bool_argument(*call) == Some(true))
    })?;
    // A requirement satisfies the contract only with its requested boolean value.
    let has_requirement = calls.iter().copied().any(|call| {
        call.method.as_str() == required
            && (!is_requirement_true_required || bool_argument(call) == Some(true))
    });

    (!has_requirement).then_some(trigger_call.span)
}

/// Return an `index` span when a clap argument chain also configures an option name.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = clap_support::index_on_option(cx, expr);
/// };
/// ```
pub fn index_on_option<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Span> {
    // Inspect each argument builder chain once from its outermost call.
    if !is_outermost_builder_call(cx, expr) {
        return None;
    }
    let calls = builder_calls(cx, expr, BuilderType::Arg);
    let index = calls.iter().find(|call| call.method.as_str() == "index")?;
    calls
        .iter()
        .any(|call| matches!(call.method.as_str(), "short" | "long"))
        .then_some(index.span)
}

/// Declare one missing-prerequisite clap builder lint.
#[macro_export]
macro_rules! declare_builder_requirement_lint {
    (
        $lint:ident,
        $pass:ident,
        $builder:ident,
        $trigger:literal,
        $trigger_true:literal,
        $required:literal,
        $required_true:literal,
        $description:literal,
        $message:literal,
        $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check an outermost resolved clap builder chain.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(span) = $crate::builder_call_without_requirement(
                    cx,
                    expr,
                    $crate::BuilderType::$builder,
                    $trigger,
                    $trigger_true,
                    $required,
                    $required_true,
                ) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help($help);
                    }),
                );
            }
        }

        /// Run the UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Prove that a resolved method belongs to clap's requested builder type.
fn is_builder_method(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    receiver: &Expr<'_>,
    builder_type: BuilderType,
) -> bool {
    // Resolve the method definition before checking the receiver type.
    let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    if cx.tcx.crate_name(method_def_id.krate).as_str() != "clap_builder" {
        return false;
    }

    // Require the receiver's nominal Clap builder type, not only the method owner.
    let receiver_ty = cx.typeck_results().expr_ty(receiver).peel_refs();
    let ty::Adt(definition, _) = receiver_ty.kind() else {
        return false;
    };

    cx.tcx.crate_name(definition.did().krate).as_str() == "clap_builder"
        && cx.tcx.item_name(definition.did()).as_str() == builder_type.item_name()
}
