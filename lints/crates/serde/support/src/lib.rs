#![feature(rustc_private)]
#![doc(hidden)]

//! Shared source and AST helpers for Serde-specific private lints.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_span;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use rustc_ast::{
    Attribute, Crate, FieldDef as AstFieldDef, GenericArg, Item as AstItem,
    ItemKind as AstItemKind, MetaItem, MetaItemInner, Ty, TyKind, UseTree, UseTreeKind,
    visit::{Visitor, walk_item},
};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{EarlyContext, LateContext, Lint, LintContext};
use rustc_span::{BytePos, SourceFile, Span, Symbol, SyntaxContext, sym};

/// A user source file loaded from rustc's source map for text-based matching.
#[derive(Clone, Debug)]
pub struct SourceCandidate {
    /// Full UTF-8 source text read from the local source file.
    pub source: String,
    /// Byte position where the source file starts in rustc's global source map.
    pub start_pos: BytePos,
}

/// A parsed outer attribute and the byte ranges needed to inspect its text.
#[derive(Clone, Copy, Debug)]
pub struct Attr {
    /// Start byte of the full `#[...]` attribute in the source string.
    pub start: usize,
    /// End byte of the full `#[...]` attribute in the source string.
    pub end: usize,
    /// Start byte of the attribute body between `#[` and `]`.
    body_start: usize,
    /// End byte of the attribute body between `#[` and `]`.
    body_end: usize,
    /// End byte of the leading attribute name inside the body.
    name_end: usize,
}

/// The source item forms that Serde derive lints care about.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemKind {
    /// A `struct` item.
    Struct,
    /// An `enum` item.
    Enum,
}

/// Source-level description of a struct or enum item.
#[derive(Clone, Debug)]
pub struct ItemInfo {
    /// Whether this item was parsed as a struct or enum.
    pub kind: ItemKind,
    /// Identifier text for the parsed item.
    pub name: String,
    /// Outer attributes directly attached to the item.
    pub attrs: Vec<Attr>,
    /// Serde derive facts recovered from item attributes.
    pub derives: Derives,
    /// Struct fields, or empty for enum items.
    pub fields: Vec<FieldInfo>,
    /// Enum variants, or empty for struct items.
    pub variants: Vec<VariantInfo>,
}

/// Source-level description of a named field.
#[derive(Clone, Debug)]
pub struct FieldInfo {
    /// Field identifier without raw-identifier prefix.
    pub name: String,
    /// Raw type tokens for the field.
    pub ty: String,
    /// Start byte of the field segment in the source string.
    pub start: usize,
    /// End byte of the field segment in the source string.
    pub end: usize,
    /// Outer attributes directly attached to the field.
    pub attrs: Vec<Attr>,
}

/// Source-level description of an enum variant.
#[derive(Clone, Debug)]
pub struct VariantInfo {
    /// Variant identifier.
    pub name: String,
    /// Outer attributes directly attached to the variant.
    pub attrs: Vec<Attr>,
    /// Named fields parsed from the variant body.
    pub fields: Vec<FieldInfo>,
}

/// Serde derive flags recovered from attributes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Derives {
    /// Whether the item derives or aliases `Serialize`.
    pub has_serialize: bool,
    /// Whether the item derives or aliases `Deserialize`.
    pub has_deserialize: bool,
    /// Whether the item directly derives `Default`.
    pub has_default: bool,
}

impl Derives {
    /// Return true when either Serde derive is present.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = serde_support::Derives::has_serde(value);
    /// };
    /// ```
    pub const fn has_serde(self) -> bool {
        self.has_serialize || self.has_deserialize
    }

    /// Return true when the item only derives serialization.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = serde_support::Derives::only_serialize(value);
    /// };
    /// ```
    pub const fn only_serialize(self) -> bool {
        self.has_serialize && !self.has_deserialize
    }

    /// Return true when the item only derives deserialization.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = serde_support::Derives::only_deserialize(value);
    /// };
    /// ```
    pub const fn only_deserialize(self) -> bool {
        self.has_deserialize && !self.has_serialize
    }
}

/// String literal value parsed from a Serde attribute.
#[derive(Clone, Debug)]
pub struct SerdeValue {
    /// Unescaped string value without surrounding quotes.
    pub value: String,
    /// Start byte of the literal including the opening quote.
    pub literal_start: usize,
    /// End byte of the literal including the closing quote.
    pub literal_end: usize,
}

/// AST-level Serde facts collected from a crate.
#[derive(Debug)]
pub struct AstSerdeCrate<'ast> {
    /// AST items that can be checked by Serde lints.
    pub items: Vec<AstItemInfo<'ast>>,
    /// Type aliases and imports that affect Serde type matching.
    pub type_facts: AstTypeFacts,
}

/// AST-level description of a struct or enum item.
#[derive(Debug)]
pub struct AstItemInfo<'ast> {
    /// Whether this AST item is a struct or enum.
    pub kind: ItemKind,
    /// Identifier text for the item.
    pub name: String,
    /// Outer attributes attached to the item.
    pub attrs: &'ast [Attribute],
    /// Serde derive facts recovered from AST attributes and source fallback.
    pub derives: Derives,
    /// Struct fields, or empty for enum items.
    pub fields: Vec<AstFieldInfo<'ast>>,
    /// Enum variants, or empty for struct items.
    pub variants: Vec<AstVariantInfo<'ast>>,
}

/// AST-level description of a field.
#[derive(Debug)]
pub struct AstFieldInfo<'ast> {
    /// Named-field identifier, or `None` for tuple fields.
    pub name: Option<String>,
    /// Field type node.
    pub ty: &'ast Ty,
    /// Field span used for diagnostics.
    pub span: Span,
    /// Outer attributes attached to the field.
    pub attrs: &'ast [Attribute],
}

/// AST-level description of an enum variant.
#[derive(Debug)]
pub struct AstVariantInfo<'ast> {
    /// Variant identifier.
    pub name: String,
    /// Whether this is a unit variant rather than an empty struct variant.
    pub is_unit: bool,
    /// Full variant span used for diagnostics.
    pub span: Span,
    /// Outer attributes attached to the variant.
    pub attrs: &'ast [Attribute],
    /// Fields declared by the variant.
    pub fields: Vec<AstFieldInfo<'ast>>,
}

/// Import and type-alias facts used to resolve Serde-relevant AST types.
#[derive(Clone, Debug, Default)]
pub struct AstTypeFacts {
    /// Aliases that resolve to `&str`.
    str_ref_aliases: BTreeSet<String>,
    /// Aliases that resolve to `&[u8]`.
    bytes_ref_aliases: BTreeSet<String>,
    /// Aliases that resolve to `Cow`.
    cow_aliases: BTreeSet<String>,
    /// Locally imported names that refer to `Cow`.
    cow_import_names: BTreeSet<String>,
    /// Single-segment aliases for imported or type-aliased paths.
    path_aliases: BTreeMap<String, String>,
}

/// Serialization and deserialization values from a directional Serde attribute.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SerdeDirectionalValue {
    /// Value used while serializing.
    pub serialize: Option<String>,
    /// Value used while deserializing.
    pub deserialize: Option<String>,
}

/// One semantically resolved Serde trait method call.
#[derive(Clone, Copy, Debug)]
pub struct SerdeMethodCall<'hir> {
    /// Span of the method identifier.
    pub method_span: Span,
    /// Method receiver expression.
    pub receiver: &'hir Expr<'hir>,
    /// Explicit method arguments.
    pub arguments: &'hir [Expr<'hir>],
}

impl Attr {
    /// Return the full attribute source including delimiters.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value, source| {
    ///     let _ = serde_support::Attr::source(value, source);
    /// };
    /// ```
    pub fn source<'source>(&self, source: &'source str) -> Option<&'source str> {
        source.get(self.start..self.end)
    }

    /// Return the attribute body between `#[` and `]`.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value, source| {
    ///     let _ = serde_support::Attr::body(value, source);
    /// };
    /// ```
    pub fn body<'source>(&self, source: &'source str) -> Option<&'source str> {
        source.get(self.body_start..self.body_end)
    }

    /// Return the leading attribute name before any arguments.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value, source| {
    ///     let _ = serde_support::Attr::name(value, source);
    /// };
    /// ```
    pub fn name<'source>(&self, source: &'source str) -> Option<&'source str> {
        source.get(self.body_start..self.name_end)
    }

    /// Return the parenthesized argument text for attributes such as `serde(...)`.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value, source| {
    ///     let _ = serde_support::Attr::args(value, source);
    /// };
    /// ```
    pub fn args<'source>(&self, source: &'source str) -> Option<&'source str> {
        let body = self.body(source)?;
        let name = self.name(source)?;
        let after_name = body.get(name.len()..)?.trim_start();

        after_name.strip_prefix('(')?.trim_end().strip_suffix(')')
    }
}

/// Collect AST-backed Serde facts for all active items in a crate.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, krate| {
///     let _ = serde_support::serde_ast_crate(cx, krate);
/// };
/// ```
pub fn serde_ast_crate<'ast>(cx: &EarlyContext<'_>, krate: &'ast Crate) -> AstSerdeCrate<'ast> {
    // Recover source-level derives before visiting cfg-active AST items.
    let source_derives = source_type_derives(cx);
    let mut collector = AstSerdeCollector {
        cx,
        source_derives: &source_derives,
        items: Vec::new(),
        type_facts: AstTypeFacts::default(),
    };

    // Collect item and type facts in source order through rustc's visitor.
    for item in &krate.items {
        collector.visit_item(item);
    }

    collector.type_facts.resolve_alias_chains();

    AstSerdeCrate {
        items: collector.items,
        type_facts: collector.type_facts,
    }
}

/// Visitor state for collecting Serde derive and type-alias facts from AST items.
struct AstSerdeCollector<'ast, 'cx> {
    /// Lint context used for source-map fallback checks.
    cx: &'cx EarlyContext<'cx>,
    /// Source-derived Serde facts keyed by item name.
    source_derives: &'cx BTreeMap<String, Derives>,
    /// Collected lintable items.
    items: Vec<AstItemInfo<'ast>>,
    /// Import and alias facts collected while visiting items.
    type_facts: AstTypeFacts,
}

impl std::fmt::Debug for AstSerdeCollector<'_, '_> {
    /// Helper for fmt analysis.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstSerdeCollector")
            .field("source_derives", self.source_derives)
            .field("items", &self.items)
            .field("type_facts", &self.type_facts)
            .finish_non_exhaustive()
    }
}

impl<'ast> Visitor<'ast> for AstSerdeCollector<'ast, '_> {
    /// Helper for visit item analysis.
    fn visit_item(&mut self, item: &'ast AstItem) {
        // Collect aliases and imports from cfg-active AST items before lint matching uses them.
        match &item.kind {
            AstItemKind::Use(use_tree) => self.type_facts.record_use_tree(use_tree, &[]),
            AstItemKind::TyAlias(alias) => {
                if let Some(ty) = alias.ty.as_deref() {
                    self.type_facts.record_type_alias(alias.ident.name, ty);
                }
            }
            AstItemKind::Struct(..) | AstItemKind::Enum(..) => self.record_aggregate_item(item),
            AstItemKind::ExternCrate(..)
            | AstItemKind::Static(_)
            | AstItemKind::Const(_)
            | AstItemKind::ConstBlock(_)
            | AstItemKind::Fn(_)
            | AstItemKind::Mod(..)
            | AstItemKind::ForeignMod(_)
            | AstItemKind::GlobalAsm(_)
            | AstItemKind::Union(..)
            | AstItemKind::Trait(_)
            | AstItemKind::TraitAlias(_)
            | AstItemKind::Impl(_)
            | AstItemKind::MacCall(_)
            | AstItemKind::MacroDef(..)
            | AstItemKind::Delegation(_)
            | AstItemKind::DelegationMac(_) => {}
        }

        walk_item(self, item);
    }
}

impl<'ast> AstSerdeCollector<'ast, '_> {
    /// Record a struct or enum and the field facts needed by item-level checks.
    fn record_aggregate_item(&mut self, item: &'ast AstItem) {
        // Store struct metadata without constructing variant facts.
        if let AstItemKind::Struct(ident, _, data) = &item.kind {
            let name = ident.name.to_ident_string();
            self.items.push(AstItemInfo {
                kind: ItemKind::Struct,
                derives: self.derives_for_item(&name, &item.attrs),
                name,
                attrs: &item.attrs,
                fields: ast_fields(data.fields()),
                variants: Vec::new(),
            });
        }
        // Derive enum variant facts before recording the aggregate item.
        if let AstItemKind::Enum(ident, _, enum_def) = &item.kind {
            let name = ident.name.to_ident_string();
            let variants = enum_def
                .variants
                .iter()
                .map(|variant| AstVariantInfo {
                    name: variant.ident.name.to_ident_string(),
                    is_unit: matches!(variant.data, rustc_ast::VariantData::Unit(..)),
                    span: variant.span,
                    attrs: &variant.attrs,
                    fields: ast_fields(variant.data.fields()),
                })
                .collect();
            self.items.push(AstItemInfo {
                kind: ItemKind::Enum,
                derives: self.derives_for_item(&name, &item.attrs),
                name,
                attrs: &item.attrs,
                fields: Vec::new(),
                variants,
            });
        }
    }

    /// Prefer source-derived Serde derive facts when rustc's early AST metadata is incomplete.
    fn derives_for_item(&self, name: &str, attrs: &[Attribute]) -> Derives {
        self.source_derives
            .get(name)
            .copied()
            .unwrap_or_else(|| derives_from_ast_attrs(self.cx, attrs))
    }
}

/// Recover item derive facts from source text as a fallback for early-lint AST gaps.
fn source_type_derives(cx: &EarlyContext<'_>) -> BTreeMap<String, Derives> {
    // Merge cfg-active source candidates into one name-indexed derive map.
    let mut derives_by_type = BTreeMap::new();

    for candidate in loaded_rust_sources(cx) {
        // Retain only types that participate in Serde behavior.
        for item in parse_items(&candidate.source) {
            if item.derives.has_serde() {
                let _ = derives_by_type.insert(item.name, item.derives);
            }
        }
    }

    derives_by_type
}

/// Convert rustc AST fields into compact field records for lint matching.
fn ast_fields(fields: &[AstFieldDef]) -> Vec<AstFieldInfo<'_>> {
    fields
        .iter()
        .map(|field| AstFieldInfo {
            name: field.ident.map(|ident| {
                ident
                    .name
                    .to_ident_string()
                    .trim_start_matches("r#")
                    .to_owned()
            }),
            ty: &field.ty,
            span: field.span,
            attrs: &field.attrs,
        })
        .collect()
}

impl AstTypeFacts {
    /// Record a type alias when it resolves to a Serde-relevant borrowed type.
    fn record_type_alias(&mut self, alias: Symbol, ty: &Ty) {
        // Normalize raw identifiers before storing any alias facts.
        let alias = alias.to_ident_string();

        // Record each directly recognized borrowed representation independently.
        if ast_ty_is_borrowed_str(ty, self) {
            let _ = self.str_ref_aliases.insert(alias.clone());
        }
        if ast_ty_is_borrowed_bytes(ty, self) {
            let _ = self.bytes_ref_aliases.insert(alias.clone());
        }
        if ast_ty_is_borrowable_cow(ty, self) {
            let _ = self.cow_aliases.insert(alias.clone());
        }
        if let Some(target) = ast_ty_path_last_name(ty) {
            drop(self.path_aliases.insert(alias, target.to_owned()));
        }
    }

    /// Record `use` paths that make `Cow` visible through local names.
    fn record_use_tree(&mut self, tree: &UseTree, parent: &[String]) {
        // Extend the parent path before classifying the current use-tree node.
        let path = parent
            .iter()
            .cloned()
            .chain(path_segment_names(&tree.prefix))
            .collect::<Vec<_>>();

        // Record direct `Cow` imports and recurse through nested groups.
        match &tree.kind {
            UseTreeKind::Simple(rename) => {
                if is_std_cow_path(&path) {
                    let import_name = rename
                        .map(|ident| ident.name.to_ident_string())
                        .or_else(|| path.last().cloned());
                    if let Some(import_name) = import_name {
                        let _ = self.cow_import_names.insert(import_name);
                    }
                }
            }
            UseTreeKind::Nested { items, .. } => {
                for (nested, _) in items {
                    self.record_use_tree(nested, &path);
                }
            }
            UseTreeKind::Glob(_) => {}
        }
    }

    /// Expand aliases that point at other aliases already known to be relevant.
    fn resolve_alias_chains(&mut self) {
        // Snapshot aliases because successful resolutions extend the target sets.
        let aliases = self.path_aliases.clone();
        // Resolve each borrowed representation against the same alias graph.
        for alias in aliases.keys() {
            if self.alias_targets(alias, &self.str_ref_aliases) {
                let _ = self.str_ref_aliases.insert(alias.clone());
            }
            if self.alias_targets(alias, &self.bytes_ref_aliases) {
                let _ = self.bytes_ref_aliases.insert(alias.clone());
            }
            if self.alias_targets(alias, &self.cow_aliases) {
                let _ = self.cow_aliases.insert(alias.clone());
            }
        }
    }

    /// Return true when an alias chain reaches one of the target names.
    fn alias_targets(&self, alias: &str, targets: &BTreeSet<String>) -> bool {
        // Track visited names so cyclic alias graphs terminate.
        let mut current = alias;
        let mut seen = BTreeSet::new();

        while let Some(next) = self.path_aliases.get(current) {
            // A target match wins before cycle detection advances the chain.
            if targets.contains(next) {
                return true;
            }
            if !seen.insert(current.to_owned()) {
                return false;
            }
            // Follow only the recorded final path segment for the next alias.
            current = next;
        }

        false
    }

    /// Return true when a local type name resolves to one of the target names.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value, name, targets| {
    ///     let _ = serde_support::AstTypeFacts::resolves_to_name(value, name, targets);
    /// };
    /// ```
    pub fn resolves_to_name(&self, name: &str, targets: &BTreeSet<String>) -> bool {
        targets.contains(name) || self.alias_targets(name, targets)
    }
}

/// Recover Serde derives from rustc's structured attribute representation.
fn derives_from_ast_attrs(cx: &EarlyContext<'_>, attrs: &[Attribute]) -> Derives {
    // Merge every derive attribute into one set of semantic flags.
    let mut derives = Derives::default();

    for attr in attrs.iter().filter(|attr| attr.has_name(sym::derive)) {
        // Ignore attributes whose structured arguments were not retained.
        let Some(args) = attr.meta_item_list() else {
            continue;
        };

        // Match only final derive path segments from structured metadata.
        for inner in args {
            let Some(meta) = inner.meta_item() else {
                continue;
            };
            let Some(name) = meta_path_last_name(meta) else {
                continue;
            };
            derives.has_serialize |= name == "Serialize";
            derives.has_deserialize |= name == "Deserialize";
            derives.has_default |= name == "Default";
        }

        // Supplement thin early metadata from this attribute's bounded source.
        record_derive_source(cx, attr, &mut derives);
    }

    derives
}

/// Update derive flags from the original derive attribute source when
/// structured metadata is thin.
fn record_derive_source(cx: &EarlyContext<'_>, attr: &Attribute, derives: &mut Derives) {
    // Bound fallback parsing to the exact derive attribute span.
    let Ok(source) = cx.sess().source_map().span_to_snippet(attr.span) else {
        return;
    };
    let Some(args_start) = source.find('(') else {
        return;
    };
    let Some(args_end) = source.rfind(')') else {
        return;
    };
    if args_end <= args_start {
        return;
    }

    // rustc's structured derive meta is not populated for every early-lint path, so derive
    // argument spelling remains the narrow fallback while item selection stays cfg-filtered.
    let Some(args) = source.get(args_start + 1..args_end) else {
        return;
    };
    // Accept qualified and unqualified spellings while retaining a closed derive set.
    for token in path_tokens(args) {
        derives.has_serialize |= token == "Serialize" || token.ends_with("::Serialize");
        derives.has_deserialize |= token == "Deserialize" || token.ends_with("::Deserialize");
        derives.has_default |= token == "Default" || token.ends_with("::Default");
    }
}

/// Find a Serde AST attribute containing the attribute key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attrs, key| {
///     let _ = serde_support::ast_serde_attr(cx, attrs, key);
/// };
/// ```
pub fn ast_serde_attr<'attr>(
    cx: &EarlyContext<'_>,
    attrs: &'attr [Attribute],
    key: &str,
) -> Option<&'attr Attribute> {
    attrs
        .iter()
        .find(|attr| attr.has_name(Symbol::intern("serde")) && ast_attr_has_word(cx, attr, key))
}

/// Return a Serde attribute containing exactly one structured entry with this key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attr, key| {
///     let _ = serde_support::ast_attr_is_single_entry(cx, attr, key);
/// };
/// ```
pub fn ast_attr_is_single_entry(cx: &EarlyContext<'_>, attr: &Attribute, key: &str) -> bool {
    if has_single_structured_entry(attr, key) {
        return true;
    }

    // Early rustc metadata can omit the complete helper list, so use only the bounded
    // attribute source when structured metadata did not prove the single-entry shape.
    let Ok(source) = cx.sess().source_map().span_to_snippet(attr.span) else {
        return false;
    };
    let Some(arguments_start) = source.find('(') else {
        return false;
    };
    let Some(arguments_end) = source.rfind(')') else {
        return false;
    };
    let Some(arguments) = source.get(arguments_start + 1..arguments_end) else {
        return false;
    };
    has_one_top_level_entry(arguments) && word_tokens(arguments).any(|token| token == key)
}

/// Return true when AST attributes contain a Serde key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attrs, key| {
///     let _ = serde_support::ast_has_serde_attr(cx, attrs, key);
/// };
/// ```
pub fn ast_has_serde_attr(cx: &EarlyContext<'_>, attrs: &[Attribute], key: &str) -> bool {
    ast_serde_attr(cx, attrs, key).is_some()
}

/// Parse a Serde value with optional serialization and deserialization directions.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attrs, key| {
///     let _ = serde_support::ast_serde_directional_value(cx, attrs, key);
/// };
/// ```
pub fn ast_serde_directional_value(
    cx: &EarlyContext<'_>,
    attrs: &[Attribute],
    key: &str,
) -> Option<SerdeDirectionalValue> {
    let attr = ast_serde_attr(cx, attrs, key)?;
    let source = cx.sess().source_map().span_to_snippet(attr.span).ok()?;
    directional_value(&source, key)
}

/// Return true when an AST attribute contains a word-like key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attr, key| {
///     let _ = serde_support::ast_attr_has_word(cx, attr, key);
/// };
/// ```
pub fn ast_attr_has_word(cx: &EarlyContext<'_>, attr: &Attribute, key: &str) -> bool {
    if has_structured_entry(attr, key) {
        return true;
    }

    attr_source_has_word(cx, attr, key)
}

/// Return whether rustc's structured metadata holds exactly one entry named `key`.
fn has_single_structured_entry(attr: &Attribute, key: &str) -> bool {
    // Rustc omits the list when the attribute arguments are not plain meta items.
    let Some(arguments) = attr.meta_item_list() else {
        return false;
    };
    let [argument] = arguments.as_slice() else {
        return false;
    };
    meta_inner_key(argument) == Some(key)
}

/// Return whether rustc's structured metadata holds any entry named `key`.
fn has_structured_entry(attr: &Attribute, key: &str) -> bool {
    attr.meta_item_list()
        .is_some_and(|args| args.iter().any(|inner| meta_inner_key(inner) == Some(key)))
}

/// Search one attribute's source arguments for a key without accepting substrings.
fn attr_source_has_word(cx: &EarlyContext<'_>, attr: &Attribute, key: &str) -> bool {
    let Ok(source) = cx.sess().source_map().span_to_snippet(attr.span) else {
        return false;
    };
    let Some(args_start) = source.find('(') else {
        return false;
    };
    let Some(args_end) = source.rfind(')') else {
        return false;
    };
    if args_end <= args_start {
        return false;
    }

    // Serde derive-helper attributes are not always exposed through structured meta here, so
    // keep fallback parsing scoped to this one attribute's argument list.
    source
        .get(args_start + 1..args_end)
        .is_some_and(|args| word_tokens(args).any(|token| token == key))
}

/// Return whether a bounded attribute argument list contains no top-level comma.
fn has_one_top_level_entry(arguments: &str) -> bool {
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

/// Parse `key = "..."` or `key(serialize = "...", deserialize = "...")`.
fn directional_value(source: &str, key: &str) -> Option<SerdeDirectionalValue> {
    // Locate one whole-word key and advance to its value delimiter.
    let key_start = find_word(source, key)?;
    let cursor = skip_whitespace(source, key_start + key.len());

    // Distinguish the shared value form from directional nested assignments.
    match source.get(cursor..)?.chars().next()? {
        '=' => {
            let literal = parse_string_literal(source, skip_whitespace(source, cursor + 1))?;
            Some(SerdeDirectionalValue {
                serialize: Some(literal.value.clone()),
                deserialize: Some(literal.value),
            })
        }
        '(' => {
            let end = find_matching_delimiter(source, cursor, '(', ')')?;
            let args = source.get(cursor + 1..end)?;
            Some(SerdeDirectionalValue {
                serialize: assigned_string(args, "serialize"),
                deserialize: assigned_string(args, "deserialize"),
            })
        }
        _ => None,
    }
}

/// Parse one string assignment from a bounded attribute argument list.
fn assigned_string(source: &str, key: &str) -> Option<String> {
    // Require a whole-word key followed by an assignment delimiter.
    let key_start = find_word(source, key)?;
    let cursor = skip_whitespace(source, key_start + key.len());
    if source.get(cursor..)?.chars().next()? != '=' {
        return None;
    }

    parse_string_literal(source, skip_whitespace(source, cursor + 1)).map(|literal| literal.value)
}

/// Resolve a method call to one method on a Serde trait.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, trait_path, method_name| {
///     let _ = serde_support::serde_method_call(cx, expr, trait_path, method_name);
/// };
/// ```
pub fn serde_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    trait_path: &str,
    method_name: &str,
) -> Option<SerdeMethodCall<'hir>> {
    // Require a resolved method call with the target method name.
    let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    if cx.tcx.item_name(def_id).as_str() != method_name {
        return None;
    }
    // Accept both Serde crate names used by supported releases.
    let crate_symbol = cx.tcx.crate_name(def_id.krate);
    let crate_name = crate_symbol.as_str();
    if !matches!(crate_name, "serde" | "serde_core") {
        return None;
    }
    // Compare the resolved trait and method suffix rather than receiver spelling.
    let trait_name = trait_path.rsplit("::").next()?;
    if !cx
        .tcx
        .def_path_str(def_id)
        .ends_with(&format!("{trait_name}::{method_name}"))
    {
        return None;
    }

    // Preserve the receiver, arguments, and precise method span for downstream lints.
    Some(SerdeMethodCall {
        method_span: segment.ident.span,
        receiver,
        arguments,
    })
}

/// Return true when an expression calls the standard `ToString::to_string`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = serde_support::is_to_string_call(cx, expr);
/// };
/// ```
pub fn is_to_string_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Reject other expression shapes and method names before trait resolution.
    let ExprKind::MethodCall(segment, ..) = expr.kind else {
        return false;
    };
    if segment.ident.name.as_str() != "to_string" {
        return false;
    }
    // Resolve the associated trait instead of accepting an inherent lookalike.
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    let Some(trait_id) = cx.tcx.trait_of_assoc(def_id) else {
        return false;
    };

    cx.tcx.crate_name(trait_id.krate).as_str() == "alloc"
        && cx.tcx.def_path_str(trait_id).ends_with("ToString")
}

/// Return every field from an AST item, flattening enum variant fields.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |item| {
///     let _ = serde_support::ast_all_fields(item);
/// };
/// ```
pub fn ast_all_fields<'ast, 'item>(
    item: &'item AstItemInfo<'ast>,
) -> impl Iterator<Item = &'item AstFieldInfo<'ast>> {
    item.fields.iter().chain(
        item.variants
            .iter()
            .flat_map(|variant| variant.fields.iter()),
    )
}

/// Return true for borrowed types that Serde can deserialize without `#[serde(borrow)]`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |ty, facts| {
///     let _ = serde_support::ast_ty_is_implicitly_borrowed(ty, facts);
/// };
/// ```
pub fn ast_ty_is_implicitly_borrowed(ty: &Ty, facts: &AstTypeFacts) -> bool {
    ast_ty_is_borrowed_str(ty, facts) || ast_ty_is_borrowed_bytes(ty, facts)
}

/// Return true when a type is or aliases `&str`.
fn ast_ty_is_borrowed_str(ty: &Ty, facts: &AstTypeFacts) -> bool {
    // Accept a direct shared string reference first.
    if let TyKind::Ref(_, mut_ty) = &ty.kind {
        return matches!(mut_ty.ty.kind, TyKind::Path(_, ref path) if path_is_str(path));
    }
    // Resolve known aliases and peel parentheses recursively.
    if let TyKind::Path(_, path) = &ty.kind {
        return ast_ty_path_last_name_from_path(path)
            .is_some_and(|name| facts.resolves_to_name(name, &facts.str_ref_aliases));
    }
    if let TyKind::Paren(inner) = &ty.kind {
        return ast_ty_is_borrowed_str(inner, facts);
    }
    false
}

/// Return true when a type is or aliases `&[u8]`.
fn ast_ty_is_borrowed_bytes(ty: &Ty, facts: &AstTypeFacts) -> bool {
    // Accept a direct shared byte-slice reference first.
    if let TyKind::Ref(_, mut_ty) = &ty.kind {
        return matches!(mut_ty.ty.kind, TyKind::Slice(ref inner) if ty_is_u8(inner));
    }
    // Resolve known aliases and peel parentheses recursively.
    if let TyKind::Path(_, path) = &ty.kind {
        return ast_ty_path_last_name_from_path(path)
            .is_some_and(|name| facts.resolves_to_name(name, &facts.bytes_ref_aliases));
    }
    if let TyKind::Paren(inner) = &ty.kind {
        return ast_ty_is_borrowed_bytes(inner, facts);
    }
    false
}

/// Return true when a type is `Cow<'_, str>` or `Cow<'_, [u8]>`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |ty, facts| {
///     let _ = serde_support::ast_ty_is_borrowable_cow(ty, facts);
/// };
/// ```
pub fn ast_ty_is_borrowable_cow(ty: &Ty, facts: &AstTypeFacts) -> bool {
    // Check direct Cow paths before consulting source aliases.
    if let TyKind::Path(_, path) = &ty.kind {
        if path_is_cow(path, facts) {
            return cow_args_are_borrowable(path);
        }
        return ast_ty_path_last_name_from_path(path)
            .is_some_and(|name| facts.resolves_to_name(name, &facts.cow_aliases));
    }
    // Preserve the same classification through parenthesized types.
    if let TyKind::Paren(inner) = &ty.kind {
        return ast_ty_is_borrowable_cow(inner, facts);
    }
    false
}

/// Return the last path segment of a type path, ignoring parentheses.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |ty| {
///     let _ = serde_support::ast_ty_path_last_name(ty);
/// };
/// ```
pub fn ast_ty_path_last_name(ty: &Ty) -> Option<&str> {
    // Return a path segment directly or recurse through parentheses.
    if let TyKind::Path(_, path) = &ty.kind {
        return ast_ty_path_last_name_from_path(path);
    }
    if let TyKind::Paren(inner) = &ty.kind {
        return ast_ty_path_last_name(inner);
    }
    None
}

/// Return true when a type path resolves to any target through known aliases.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |ty, facts, targets| {
///     let _ = serde_support::ast_ty_resolves_to_any_name(ty, facts, targets);
/// };
/// ```
pub fn ast_ty_resolves_to_any_name(
    ty: &Ty,
    facts: &AstTypeFacts,
    targets: &BTreeSet<String>,
) -> bool {
    ast_ty_path_last_name(ty).is_some_and(|name| facts.resolves_to_name(name, targets))
}

/// Return the final identifier from an AST path.
fn ast_ty_path_last_name_from_path(path: &rustc_ast::Path) -> Option<&str> {
    path.segments
        .iter()
        .next_back()
        .map(|segment| segment.ident.name.as_str())
}

/// Return true when a path names `Cow` directly or through a local import.
fn path_is_cow(path: &rustc_ast::Path, facts: &AstTypeFacts) -> bool {
    let names = path_segment_names(path).collect::<Vec<_>>();
    is_std_cow_path(&names)
        || (names.len() == 1
            && names
                .last()
                .is_some_and(|name| facts.cow_import_names.contains(name)))
}

/// Return true when `Cow` generic arguments can borrow from deserializer input.
fn cow_args_are_borrowable(path: &rustc_ast::Path) -> bool {
    // Require generic arguments on the final `Cow` path segment.
    let Some(args) = path
        .segments
        .last()
        .and_then(|segment| segment.args.as_deref())
    else {
        return false;
    };
    let rustc_ast::GenericArgs::AngleBracketed(args) = args else {
        return false;
    };

    // Borrowing needs both a non-static lifetime and a supported target type.
    let mut saw_non_static_lifetime = false;
    let mut saw_borrowable_target = false;

    // Inspect only ordinary lifetime and type arguments from the angle brackets.
    for arg in &args.args {
        let rustc_ast::AngleBracketedArg::Arg(arg) = arg else {
            continue;
        };
        match arg {
            GenericArg::Lifetime(lifetime) => {
                saw_non_static_lifetime |= lifetime.ident.name != Symbol::intern("'static");
            }
            GenericArg::Type(ty) => {
                saw_borrowable_target |= ty_is_str(ty) || ty_is_u8_slice(ty);
            }
            GenericArg::Const(_) => {}
        }
    }

    saw_non_static_lifetime && saw_borrowable_target
}

/// Return true when a type node spells `str`, ignoring parentheses.
fn ty_is_str(ty: &Ty) -> bool {
    // Match the direct path or recurse through parentheses.
    if let TyKind::Path(_, path) = &ty.kind {
        return path_is_str(path);
    }
    if let TyKind::Paren(inner) = &ty.kind {
        return ty_is_str(inner);
    }
    false
}

/// Return true when a type node spells `[u8]`, ignoring parentheses.
fn ty_is_u8_slice(ty: &Ty) -> bool {
    // Match the direct slice or recurse through parentheses.
    if let TyKind::Slice(inner) = &ty.kind {
        return ty_is_u8(inner);
    }
    if let TyKind::Paren(inner) = &ty.kind {
        return ty_is_u8_slice(inner);
    }
    false
}

/// Return true when a type node spells `u8`, ignoring parentheses.
fn ty_is_u8(ty: &Ty) -> bool {
    // Match the direct path or recurse through parentheses.
    if let TyKind::Path(_, path) = &ty.kind {
        return ast_ty_path_last_name_from_path(path) == Some("u8");
    }
    if let TyKind::Paren(inner) = &ty.kind {
        return ty_is_u8(inner);
    }
    false
}

/// Return true when a path's final segment is `str`.
fn path_is_str(path: &rustc_ast::Path) -> bool {
    ast_ty_path_last_name_from_path(path) == Some("str")
}

/// Return true for canonical `std::borrow::Cow` or `alloc::borrow::Cow` paths.
fn is_std_cow_path(names: &[String]) -> bool {
    matches!(
        names,
        [krate, borrow, cow]
            if (krate == "std" || krate == "alloc")
                && borrow == "borrow"
                && cow == "Cow"
    )
}

/// Yield non-empty path segments as owned strings.
fn path_segment_names(path: &rustc_ast::Path) -> impl Iterator<Item = String> + '_ {
    path.segments.iter().filter_map(|segment| {
        let name = segment.ident.name.as_str();
        (!name.is_empty()).then(|| name.to_owned())
    })
}

/// Return a meta-list entry's final path segment.
fn meta_inner_key(inner: &MetaItemInner) -> Option<&str> {
    let meta = inner.meta_item()?;
    meta_path_last_name(meta)
}

/// Return a meta item's final path segment.
fn meta_path_last_name(meta: &MetaItem) -> Option<&str> {
    meta.path
        .segments
        .iter()
        .next_back()
        .map(|segment| segment.ident.name.as_str())
}

/// Load local Rust source files that belong to the linted crate.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = serde_support::loaded_rust_sources(cx);
/// };
/// ```
pub fn loaded_rust_sources(cx: &EarlyContext<'_>) -> Vec<SourceCandidate> {
    // Borrow the source map once and deduplicate physical paths during traversal.
    let files = cx.sess().source_map().files();
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    // Keep local readable Rust sources and their source-map offsets.
    for source_file in files.iter() {
        let Some(path) = source_file_path(source_file) else {
            continue;
        };
        if !seen.insert(path.clone()) || !is_rust_file(&path) || is_dependency_source(&path) {
            continue;
        }

        // Virtual or missing source-map entries cannot participate in fallback parsing.
        let Some(source) = read_file(&path) else {
            continue;
        };
        candidates.push(SourceCandidate {
            source,
            start_pos: source_file.start_pos,
        });
    }

    drop(files);

    candidates
}

/// Convert a rustc source-map entry into a local filesystem path.
fn source_file_path(source_file: &SourceFile) -> Option<PathBuf> {
    source_file.name.clone().into_local_path()
}

/// Return true when a path has an `.rs` extension.
fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rs")
}

/// Return true when a source path points at dependencies or compiler sources.
fn is_dependency_source(path: &Path) -> bool {
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        let name = component.as_os_str();
        if name == ".cargo"
            && components
                .peek()
                .is_some_and(|next| matches!(next.as_os_str().to_str(), Some("registry" | "git")))
        {
            return true;
        }
        if name == "rustc" {
            return true;
        }
    }
    false
}

/// Read a source file, returning `None` when rustc reported a virtual or missing path.
fn read_file(path: &Path) -> Option<String> {
    let mut source = String::new();

    // Source-file scanning keeps derive-helper attributes visible before macro expansion.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut source).ok()?;

    Some(source)
}

/// Parse top-level struct and enum items from source text.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source| {
///     let _ = serde_support::parse_items(source);
/// };
/// ```
pub fn parse_items(source: &str) -> Vec<ItemInfo> {
    // Parse attributes once so item discovery can attach adjacent ranges cheaply.
    let attrs = attr_ranges(source, None);
    let mut items = Vec::new();
    let mut cursor = 0;

    // Advance monotonically across top-level struct and enum keywords.
    while let Some((keyword_start, kind, keyword)) = next_item_keyword(source, cursor) {
        // Invalid item headers are skipped without stalling at the same keyword.
        let Some((name, after_name)) = item_name_after_keyword(source, keyword_start, keyword)
        else {
            cursor = keyword_start + keyword.len();
            continue;
        };
        // Derive flags come only from attributes directly preceding this item.
        let attrs = attrs_before_item(source, &attrs, keyword_start);
        let derives = derives_from_attrs(source, &attrs);
        let Some((body, item_end)) = item_body(source, after_name, kind) else {
            // Preserve bodyless declarations while leaving their field sets empty.
            items.push(ItemInfo {
                kind,
                name,
                attrs,
                derives,
                fields: Vec::new(),
                variants: Vec::new(),
            });
            cursor = after_name;
            continue;
        };

        // Structs own fields directly; enums own variant records instead.
        let (fields, variants) = match kind {
            ItemKind::Struct => (parse_fields(source, body.0 + 1, body.1), Vec::new()),
            ItemKind::Enum => {
                let variants = parse_variants(source, body.0 + 1, body.1);
                (Vec::new(), variants)
            }
        };

        // Commit the complete item only after its body has been parsed.
        items.push(ItemInfo {
            kind,
            name,
            attrs,
            derives,
            fields,
            variants,
        });
        cursor = item_end;
    }

    items
}

/// Find the next struct or enum keyword after a byte cursor.
fn next_item_keyword(source: &str, cursor: usize) -> Option<(usize, ItemKind, &'static str)> {
    let struct_at = find_keyword(source, cursor, "struct");
    let enum_at = find_keyword(source, cursor, "enum");

    match (struct_at, enum_at) {
        (Some(struct_at), Some(enum_at)) if struct_at <= enum_at => {
            Some((struct_at, ItemKind::Struct, "struct"))
        }
        (Some(_) | None, Some(enum_at)) => Some((enum_at, ItemKind::Enum, "enum")),
        (Some(struct_at), None) => Some((struct_at, ItemKind::Struct, "struct")),
        (None, None) => None,
    }
}

/// Find a standalone keyword that is not part of a larger identifier.
fn find_keyword(source: &str, cursor: usize, keyword: &str) -> Option<usize> {
    // Search forward while rejecting matches embedded in identifiers.
    let mut search = cursor;

    while let Some(relative) = source.get(search..)?.find(keyword) {
        // Inspect both lexical boundaries before accepting the spelling.
        let index = search + relative;
        let before = source.get(..index)?.chars().next_back();
        let after = source.get(index + keyword.len()..)?.chars().next();
        if is_word_boundary(before) && is_word_boundary(after) {
            return Some(index);
        }
        // Resume after the rejected spelling to guarantee forward progress.
        search = index + keyword.len();
    }

    None
}

/// Parse an item name immediately following a struct or enum keyword.
fn item_name_after_keyword(
    source: &str,
    keyword_start: usize,
    keyword: &str,
) -> Option<(String, usize)> {
    // Skip whitespace after the confirmed keyword before scanning the identifier.
    let after_keyword = keyword_start + keyword.len();
    let after_ws = skip_whitespace(source, after_keyword);
    let name_source = source.get(after_ws..)?;
    let name_end = name_source
        .char_indices()
        .find_map(|(index, ch)| (!ident_char(ch)).then_some(after_ws + index))
        .unwrap_or(source.len());
    // Reject an empty identifier instead of returning the keyword boundary.
    let name = source.get(after_ws..name_end)?.trim();

    (!name.is_empty()).then(|| (name.to_owned(), name_end))
}

/// Find the brace body for a braced item and return its range plus item end.
fn item_body(source: &str, after_name: usize, kind: ItemKind) -> Option<((usize, usize), usize)> {
    // Scan past generics and clauses until the item's body shape is known.
    let mut cursor = after_name;

    while cursor < source.len() {
        // Return braced bodies and reject supported bodyless item forms.
        let ch = source.get(cursor..)?.chars().next()?;
        match ch {
            '{' => {
                let end = find_matching_delimiter(source, cursor, '{', '}')?;
                return Some(((cursor, end), end + 1));
            }
            ';' => return None,
            '(' if kind == ItemKind::Struct => return None,
            _ => cursor += ch.len_utf8(),
        }
    }

    None
}

/// Collect attributes directly adjacent to an item header.
fn attrs_before_item(source: &str, attrs: &[Attr], keyword_start: usize) -> Vec<Attr> {
    // Establish the item line as the initial adjacency boundary.
    let Some(source_before_keyword) = source.get(..keyword_start) else {
        return Vec::new();
    };
    let item_line_start = source_before_keyword
        .rfind('\n')
        .map_or(0, |index| index + 1);

    // Walk attributes backward until non-whitespace separates them from the item.
    let mut cursor = item_line_start;
    let mut item_attrs = Vec::new();
    for attr in attrs.iter().rev() {
        // Ignore later attributes that belong to another source region.
        if attr.end > cursor {
            continue;
        }
        let Some(between_attr_and_item) = source.get(attr.end..cursor) else {
            break;
        };
        if !between_attr_and_item.trim().is_empty() {
            break;
        }

        // Prepend semantically by reversing the backward collection afterward.
        item_attrs.push(*attr);
        cursor = attr.start;
    }
    item_attrs.reverse();

    item_attrs
}

/// Parse named fields from a struct or variant body.
fn parse_fields(source: &str, start: usize, end: usize) -> Vec<FieldInfo> {
    split_top_level_segments(source, start, end)
        .into_iter()
        .filter_map(|(segment_start, segment_end)| parse_field(source, segment_start, segment_end))
        .collect()
}

/// Parse one field segment into name, type text, and attached attributes.
fn parse_field(source: &str, start: usize, end: usize) -> Option<FieldInfo> {
    // Exclude attached attributes before parsing the field declaration itself.
    let attrs = attr_ranges_in(source, start, end, None);
    let raw_field_start = attrs.last().map_or(start, |attr| attr.end);
    let raw_field = source.get(raw_field_start..end)?;
    let field_start = raw_field_start + raw_field.len() - raw_field.trim_start().len();
    let field_source = source.get(field_start..end)?.trim();
    // Empty and comment-only segments do not represent fields.
    if field_source.is_empty() || field_source.as_bytes().starts_with(b"//") {
        return None;
    }

    // Split the first field delimiter and normalize both semantic values.
    let (name, ty) = field_source.split_once(':')?;
    let name = clean_field_name(name)?;
    let ty = ty.trim().trim_end_matches(',').trim().to_owned();

    Some(FieldInfo {
        name,
        ty,
        start: field_start,
        end,
        attrs,
    })
}

/// Strip visibility and raw-identifier syntax from a field name.
fn clean_field_name(raw: &str) -> Option<String> {
    // Remove supported visibility prefixes before normalizing raw identifiers.
    let mut name = raw.trim();

    if let Some(rest) = name.strip_prefix("pub ") {
        name = rest.trim_start();
    } else if let Some(rest) = name.strip_prefix("pub(") {
        name = rest.split_once(')')?.1.trim_start();
    }

    (!name.is_empty()).then(|| name.trim_start_matches("r#").to_owned())
}

/// Parse enum variants from an enum body.
fn parse_variants(source: &str, start: usize, end: usize) -> Vec<VariantInfo> {
    split_top_level_segments(source, start, end)
        .into_iter()
        .filter_map(|(segment_start, segment_end)| {
            parse_variant(source, segment_start, segment_end)
        })
        .collect()
}

/// Parse one enum variant segment.
fn parse_variant(source: &str, start: usize, end: usize) -> Option<VariantInfo> {
    // Skip attached attributes before scanning the variant identifier.
    let attrs = attr_ranges_in(source, start, end, None);
    let variant_start = attrs.last().map_or(start, |attr| attr.end);
    let segment = source.get(variant_start..end)?.trim_start();
    let name_len = segment
        .char_indices()
        .find_map(|(index, ch)| (!ident_char(ch)).then_some(index))
        .unwrap_or(segment.len());
    let name = segment.get(..name_len)?.trim();
    // Reject punctuation-only segments created by trailing commas.
    if name.is_empty() {
        return None;
    }

    // Parse fields only when the variant uses a named-field body.
    let fields = variant_named_fields(source, variant_start, end);

    Some(VariantInfo {
        name: name.trim_start_matches("r#").to_owned(),
        attrs,
        fields,
    })
}

/// Parse named fields from a struct-like enum variant.
fn variant_named_fields(source: &str, start: usize, end: usize) -> Vec<FieldInfo> {
    // Bound every delimiter search to the current variant segment.
    let Some(segment) = source.get(start..end) else {
        return Vec::new();
    };
    let Some(open) = segment.find('{').map(|relative| start + relative) else {
        return Vec::new();
    };
    // Require a balanced closing brace that remains within the variant.
    let Some(close) = find_matching_delimiter(source, open, '{', '}') else {
        return Vec::new();
    };
    if close > end {
        return Vec::new();
    }

    parse_fields(source, open + 1, close)
}

/// Split comma-separated source segments while respecting nested delimiters and strings.
fn split_top_level_segments(source: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    // Reject invalid byte ranges before initializing parser state.
    if source.get(start..end).is_none() {
        return Vec::new();
    }

    // Track delimiter and string state so commas inside nested syntax stay with their segment.
    let mut segments = Vec::new();
    let mut segment_start = start;
    let mut paren_depth = 0_u32;
    let mut brace_depth = 0_u32;
    let mut bracket_depth = 0_u32;
    let mut angle_depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    let mut cursor = start;

    // Walk UTF-8 characters while all retained positions remain byte offsets.
    while cursor < end {
        let Some(ch) = source.get(cursor..end).and_then(|rest| rest.chars().next()) else {
            break;
        };

        // Strings suppress delimiter meaning and honor escaped quote characters.
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            cursor += ch.len_utf8();
            continue;
        }

        // Update nesting before classifying a comma as a segment boundary.
        if is_top_level_comma_after_update(
            ch,
            &mut paren_depth,
            &mut brace_depth,
            &mut bracket_depth,
            &mut angle_depth,
        ) {
            // Exclude empty comma-separated segments from the result.
            if has_nonblank_text(source, segment_start, cursor) {
                segments.push((segment_start, cursor));
            }
            // Begin the next segment immediately after the top-level comma.
            segment_start = cursor + ch.len_utf8();
        } else if ch == '"' {
            in_string = true;
        }
        cursor += ch.len_utf8();
    }

    // Preserve a final non-empty segment when the range has no trailing comma.
    push_tail_segment(source, segment_start, end, &mut segments);

    segments
}

/// Append the final source segment when it contains non-whitespace text.
fn push_tail_segment(
    source: &str,
    segment_start: usize,
    end: usize,
    segments: &mut Vec<(usize, usize)>,
) {
    if segment_start < end && has_nonblank_text(source, segment_start, end) {
        segments.push((segment_start, end));
    }
}

/// Return whether a source byte range is valid and contains non-whitespace text.
fn has_nonblank_text(source: &str, start: usize, end: usize) -> bool {
    source
        .get(start..end)
        .is_some_and(|segment| !segment.trim().is_empty())
}

/// Update delimiter depths and report whether a character is a top-level comma.
const fn is_top_level_comma_after_update(
    ch: char,
    paren_depth: &mut u32,
    brace_depth: &mut u32,
    bracket_depth: &mut u32,
    angle_depth: &mut u32,
) -> bool {
    match ch {
        '(' => *paren_depth += 1,
        ')' => *paren_depth = paren_depth.saturating_sub(1),
        '{' => *brace_depth += 1,
        '}' => *brace_depth = brace_depth.saturating_sub(1),
        '[' => *bracket_depth += 1,
        ']' => *bracket_depth = bracket_depth.saturating_sub(1),
        '<' => *angle_depth += 1,
        '>' => *angle_depth = angle_depth.saturating_sub(1),
        ',' => {
            return *paren_depth == 0
                && *brace_depth == 0
                && *bracket_depth == 0
                && *angle_depth == 0;
        }
        _ => {}
    }
    false
}

/// Parse attributes from a full source string, optionally filtering by name.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source, name| {
///     let _ = serde_support::attr_ranges(source, name);
/// };
/// ```
pub fn attr_ranges(source: &str, name: Option<&str>) -> Vec<Attr> {
    attr_ranges_in(source, 0, source.len(), name)
}

/// Parse attributes inside a source byte range, optionally filtering by name.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source, start, end, name| {
///     let _ = serde_support::attr_ranges_in(source, start, end, name);
/// };
/// ```
pub fn attr_ranges_in(source: &str, start: usize, end: usize, name: Option<&str>) -> Vec<Attr> {
    // Reject invalid source ranges before searching for attribute openers.
    if source.get(start..end).is_none() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut search_start = start;

    // Find attributes monotonically within the caller's byte range.
    while search_start < end {
        let Some(relative_start) = source
            .get(search_start..end)
            .and_then(|remaining| remaining.find("#["))
        else {
            break;
        };
        // Stop at an unterminated attribute because later openers remain nested text.
        let attr_start = search_start + relative_start;
        let Some(attr_end) = find_attr_end(source, attr_start, end) else {
            break;
        };
        let Some(attr) = build_attr(source, attr_start, attr_end) else {
            // A malformed record still advances past its balanced attribute span.
            search_start = attr_end;
            continue;
        };
        // Apply the optional name filter only after building a complete range record.
        if name.is_none_or(|name| attr.name(source) == Some(name)) {
            ranges.push(attr);
        }
        search_start = attr_end;
    }

    ranges
}

/// Find the end byte of an attribute, accounting for nested brackets and strings.
fn find_attr_end(source: &str, start: usize, limit: usize) -> Option<usize> {
    // Track nested brackets independently from quoted string contents.
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    let mut cursor = start;

    // Stay within the caller's range while advancing by UTF-8 character widths.
    while cursor < limit {
        let ch = source.get(cursor..limit)?.chars().next()?;
        // Escapes and quotes inside strings suppress bracket transitions.
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            cursor += ch.len_utf8();
            continue;
        }

        // Return immediately when the outermost attribute bracket closes.
        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                // Depth zero marks the byte immediately after the complete attribute.
                if depth == 0 {
                    return Some(cursor + ch.len_utf8());
                }
            }
            _ => {}
        }
        cursor += ch.len_utf8();
    }

    None
}

/// Build an attribute range record from a known `#[...]` span.
fn build_attr(source: &str, start: usize, end: usize) -> Option<Attr> {
    // Remove the `#[` opener and final bracket from the attribute body.
    let body_start = start + 2;
    let body_end = end.checked_sub(1)?;
    let body = source.get(body_start..body_end)?;
    // Normalize leading whitespace before scanning a path-like attribute name.
    let trimmed_start = body_start + body.len() - body.trim_start().len();
    let name_source = source.get(trimmed_start..body_end)?;
    let name_len = name_source
        .char_indices()
        .find_map(|(index, ch)| (!(ident_char(ch) || ch == ':')).then_some(index))
        .unwrap_or(body_end - trimmed_start);
    let name_end = trimmed_start + name_len;

    (name_end > trimmed_start).then_some(Attr {
        start,
        end,
        body_start: trimmed_start,
        body_end,
        name_end,
    })
}

/// Recover Serde derive flags from source-level attributes.
fn derives_from_attrs(source: &str, attrs: &[Attr]) -> Derives {
    // Merge every source-level derive attribute into one semantic flag set.
    let mut derives = Derives::default();

    // Filter by resolved attribute name before parsing argument tokens.
    for attr in attrs
        .iter()
        .filter(|attr| attr.name(source) == Some("derive"))
    {
        // Skip malformed derive attributes whose argument range is unavailable.
        let Some(args) = attr.args(source) else {
            continue;
        };

        // Accept qualified derive paths but retain only the supported final names.
        for token in path_tokens(args) {
            derives.has_serialize |= token == "Serialize" || token.ends_with("::Serialize");
            derives.has_deserialize |= token == "Deserialize" || token.ends_with("::Deserialize");
            derives.has_default |= token == "Default" || token.ends_with("::Default");
        }
    }

    derives
}

/// Find a source-level Serde attribute containing the attribute key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source, attrs, key| {
///     let _ = serde_support::serde_attr(source, attrs, key);
/// };
/// ```
pub fn serde_attr<'attr>(source: &str, attrs: &'attr [Attr], key: &str) -> Option<&'attr Attr> {
    attrs
        .iter()
        .find(|attr| attr.name(source) == Some("serde") && serde_attr_has_word(source, attr, key))
}

/// Return true when source-level attributes contain a Serde key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source, attrs, key| {
///     let _ = serde_support::has_serde_attr(source, attrs, key);
/// };
/// ```
pub fn has_serde_attr(source: &str, attrs: &[Attr], key: &str) -> bool {
    serde_attr(source, attrs, key).is_some()
}

/// Return true when a Serde attribute argument contains a word-like key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source, attr, key| {
///     let _ = serde_support::serde_attr_has_word(source, attr, key);
/// };
/// ```
pub fn serde_attr_has_word(source: &str, attr: &Attr, key: &str) -> bool {
    let Some(args) = attr.args(source) else {
        return false;
    };

    word_tokens(args).any(|token| token == key)
}

/// Parse a string literal value assigned to a Serde attribute key.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source, attr, key| {
///     let _ = serde_support::serde_attr_value(source, attr, key);
/// };
/// ```
pub fn serde_attr_value(source: &str, attr: &Attr, key: &str) -> Option<SerdeValue> {
    // Resolve attribute arguments and their absolute source offset together.
    let args = attr.args(source)?;
    let args_start = attr
        .source(source)?
        .find(args)
        .map(|relative| attr.start + relative)?;
    let key_relative = find_word(args, key)?;
    // Advance from the whole-word key through whitespace to its assignment.
    let mut cursor = args_start + key_relative + key.len();
    cursor = skip_whitespace(source, cursor);
    if source.get(cursor..)?.chars().next()? != '=' {
        return None;
    }
    cursor = skip_whitespace(source, cursor + 1);
    parse_string_literal(source, cursor)
}

/// Find a standalone word inside source text.
fn find_word(source: &str, word: &str) -> Option<usize> {
    // Search forward while rejecting occurrences embedded in identifiers.
    let mut cursor = 0;

    while let Some(relative) = source.get(cursor..)?.find(word) {
        // Inspect both lexical boundaries before accepting the occurrence.
        let index = cursor + relative;
        let before = source.get(..index)?.chars().next_back();
        let after = source.get(index + word.len()..)?.chars().next();
        if is_word_boundary(before) && is_word_boundary(after) {
            return Some(index);
        }
        // Resume after the rejected spelling to guarantee progress.
        cursor = index + word.len();
    }

    None
}

/// Parse a simple quoted string literal and return its value and literal range.
fn parse_string_literal(source: &str, start: usize) -> Option<SerdeValue> {
    // Require a quoted literal at the caller-supplied byte boundary.
    if source.get(start..)?.chars().next()? != '"' {
        return None;
    }

    // Accumulate the decoded simple string while retaining source byte offsets.
    let mut value = String::new();
    let mut escaped = false;
    let mut cursor = start + 1;
    // Walk until an unescaped closing quote completes the literal.
    while cursor < source.len() {
        let ch = source.get(cursor..)?.chars().next()?;
        if escaped {
            // Preserve the escaped character without interpreting escape sequences further.
            value.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            // Return the decoded value and half-open literal source range together.
            return Some(SerdeValue {
                value,
                literal_start: start,
                literal_end: cursor + 1,
            });
        } else {
            value.push(ch);
        }
        cursor += ch.len_utf8();
    }

    None
}

/// Split derive argument source into path-like tokens.
fn path_tokens(source: &str) -> impl Iterator<Item = String> + '_ {
    source
        .split(',')
        .map(str::trim)
        .map(|token| token.trim_start_matches("r#").to_owned())
}

/// Split type source into identifier and path tokens.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source| {
///     let _ = serde_support::type_tokens(source);
/// };
/// ```
pub fn type_tokens(source: &str) -> impl Iterator<Item = &str> {
    source
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
        .filter(|token| !token.is_empty())
}

/// Split source into identifier-shaped word tokens.
fn word_tokens(source: &str) -> impl Iterator<Item = &str> {
    source
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|token| !token.is_empty())
}

/// Find the matching closing delimiter while ignoring delimiters inside strings.
fn find_matching_delimiter(
    source: &str,
    open: usize,
    open_ch: char,
    close_ch: char,
) -> Option<usize> {
    // Track nesting and string state from the caller-verified opening position.
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    let mut cursor = open;

    // Advance over UTF-8 characters while retaining byte offsets.
    while cursor < source.len() {
        let ch = source.get(cursor..)?.chars().next()?;
        // Ignore delimiters inside strings and honor escaped quote characters.
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            cursor += ch.len_utf8();
            continue;
        }

        // Enter strings or update the delimiter depth.
        if ch == '"' {
            in_string = true;
        } else if ch == open_ch {
            depth += 1;
        } else if ch == close_ch {
            // The matching closer is the one that returns nesting to zero.
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(cursor);
            }
        }
        // Move by the current character width to keep every cursor on a UTF-8 boundary.
        cursor += ch.len_utf8();
    }

    None
}

/// Advance a byte cursor past ASCII or Unicode whitespace.
fn skip_whitespace(source: &str, mut cursor: usize) -> usize {
    // Advance from a valid byte boundary until the first non-whitespace character.
    while cursor < source.len() {
        let Some(ch) = source.get(cursor..).and_then(|rest| rest.chars().next()) else {
            break;
        };
        if !ch.is_whitespace() {
            break;
        }
        // Preserve UTF-8 boundaries while skipping Unicode whitespace.
        cursor += ch.len_utf8();
    }

    cursor
}

/// Return true for characters accepted in the source parser's identifier fragments.
const fn ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// Return whether a neighboring character, or the source edge, ends an identifier.
fn is_word_boundary(neighbor: Option<char>) -> bool {
    neighbor.is_none_or(|ch| !ident_char(ch))
}

/// Return every source-level field from a struct or enum item.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |item| {
///     let _ = serde_support::all_fields(item);
/// };
/// ```
pub fn all_fields(item: &ItemInfo) -> impl Iterator<Item = &FieldInfo> {
    item.fields.iter().chain(
        item.variants
            .iter()
            .flat_map(|variant| variant.fields.iter()),
    )
}

/// Expand an attribute deletion range to the full line when the attribute is standalone.
///
/// Return `None` when either offset is not a valid UTF-8 boundary in `source`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source, start, end| {
///     let _ = serde_support::attr_deletion_range(source, start, end);
/// };
/// ```
pub fn attr_deletion_range(source: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    // Validate both offsets and retain the exact attribute slice before line expansion.
    let before_attr = source.get(..start)?;
    let after_attr = source.get(end..)?;
    let attr_source = source.get(start..end)?;
    let line_start = before_attr.rfind('\n').map_or(0, |index| index + 1);
    let line_end = after_attr
        .find('\n')
        .map_or(end, |relative| end + relative + 1);
    let line_source = source.get(line_start..line_end)?;

    // Remove an entire standalone attribute line; inline attributes only lose the attribute.
    if line_source.trim() == attr_source.trim() {
        Some((line_start, line_end))
    } else {
        Some((start, end))
    }
}

/// Convert source-relative byte offsets into a rustc span.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |start_pos, start, end| {
///     let _ = serde_support::range_span(start_pos, start, end);
/// };
/// ```
pub fn range_span(start_pos: BytePos, start: usize, end: usize) -> Span {
    let lo = start_pos + byte_pos(start);
    let hi = start_pos + byte_pos(end);

    Span::new(lo, hi, SyntaxContext::root(), None)
}

/// Convert a `usize` source offset into rustc's 32-bit byte position.
fn byte_pos(offset: usize) -> BytePos {
    BytePos(u32::try_from(offset).expect("source file offset exceeds rustc BytePos range"))
}

/// Emit a span lint with a help message and no machine-applicable rewrite.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, lint, span, message, help| {
///     let _ = serde_support::emit_span_lint_with_help(cx, lint, span, message, help);
/// };
/// ```
pub fn emit_span_lint_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // The lints using this helper need a human choice rather than an exact local rewrite.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _configured_diagnostic = diag.primary_message(message).help(help);
        }),
    );
}

/// Emit a span lint with a local source replacement suggestion.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, lint, span, message, help, suggestion: String, applicability| {
///     let _ = serde_support::emit_span_lint_with_suggestion(cx, lint, span, message, help, suggestion, applicability);
/// };
/// ```
pub fn emit_span_lint_with_suggestion(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
    suggestion: impl Into<String>,
    applicability: Applicability,
) {
    let suggestion = suggestion.into();

    // Callers pass spans that cover one source attribute or literal, so rustc can patch locally.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _configured_diagnostic = diag.primary_message(message).span_suggestion(
                span,
                help,
                suggestion,
                applicability,
            );
        }),
    );
}
