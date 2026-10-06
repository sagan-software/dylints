#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for Serde-specific lints.
//!
//! The helpers resolve derived Serde implementations through rustc's trait
//! impl index and macro expansion data, read Serde helper attributes from HIR,
//! and classify field types the same way `serde_derive` does.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::{LitKind, MetaItemInner, MetaItemKind, MetaItemLit};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    Attribute, Expr, ExprKind, FieldDef, HirId, Item, ItemKind, Mutability, PrimTy, QPath, Ty,
    TyKind, VariantData,
    def::{DefKind, Res},
};
use rustc_lint::{LateContext, Lint, LintContext as _};
use rustc_middle::ty::{self, fast_reject::SimplifiedType};
use rustc_span::{
    ExpnKind, MacroKind, Span, Symbol,
    def_id::{DefId, LocalDefId},
    sym,
};

/// Derive facts that Serde lints need for one local type.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Derives {
    /// Whether `serde_derive` generated a `Serialize` implementation.
    pub has_serialize: bool,
    /// Whether `serde_derive` generated a `Deserialize` implementation.
    pub has_deserialize: bool,
    /// Whether the built-in `Default` derive generated an implementation.
    pub has_default: bool,
}

impl Derives {
    /// Return true when either Serde derive is present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// assert!(!serde_support::Derives::default().has_serde());
    /// ```
    #[must_use]
    pub const fn has_serde(self) -> bool {
        self.has_serialize || self.has_deserialize
    }

    /// Return true when the item only derives serialization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let derives = serde_support::Derives { has_serialize: true, ..Default::default() };
    /// assert!(derives.only_serialize());
    /// ```
    #[must_use]
    pub const fn only_serialize(self) -> bool {
        self.has_serialize && !self.has_deserialize
    }

    /// Return true when the item only derives deserialization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let derives = serde_support::Derives { has_deserialize: true, ..Default::default() };
    /// assert!(derives.only_deserialize());
    /// ```
    #[must_use]
    pub const fn only_deserialize(self) -> bool {
        self.has_deserialize && !self.has_serialize
    }
}

/// Whether a local struct or enum item is a struct or an enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdtKind {
    /// A `struct` item.
    Struct,
    /// An `enum` item.
    Enum,
}

/// One struct or enum item with its Serde derive facts.
#[derive(Debug)]
pub struct SerdeItem<'tcx> {
    /// Local definition of the item.
    pub def_id: LocalDefId,
    /// Whether the item is a struct or an enum.
    pub kind: AdtKind,
    /// Item identifier.
    pub ident: rustc_span::Ident,
    /// Span of the whole item, excluding outer attributes.
    pub span: Span,
    /// Outer attributes attached to the item.
    pub attrs: &'tcx [Attribute],
    /// Derived Serde and `Default` implementations for the item.
    pub derives: Derives,
    /// Struct fields, or empty for enums.
    pub fields: Vec<SerdeField<'tcx>>,
    /// Enum variants, or empty for structs.
    pub variants: Vec<SerdeVariant<'tcx>>,
}

/// One struct or variant field.
#[derive(Debug)]
pub struct SerdeField<'tcx> {
    /// HIR node of the field, used to honor lint levels set on the field.
    pub hir_id: HirId,
    /// Local definition of the field.
    pub def_id: LocalDefId,
    /// Named-field identifier, or `None` for tuple fields.
    pub name: Option<Symbol>,
    /// Field type as written in source.
    pub ty: &'tcx Ty<'tcx>,
    /// Full field span.
    pub span: Span,
    /// Outer attributes attached to the field.
    pub attrs: &'tcx [Attribute],
}

/// One enum variant.
#[derive(Debug)]
pub struct SerdeVariant<'tcx> {
    /// HIR node of the variant, used to honor lint levels set on the variant.
    pub hir_id: HirId,
    /// Variant identifier.
    pub ident: rustc_span::Ident,
    /// Whether this is a unit variant rather than an empty struct or tuple variant.
    pub is_unit: bool,
    /// Full variant span.
    pub span: Span,
    /// Outer attributes attached to the variant.
    pub attrs: &'tcx [Attribute],
    /// Fields declared by the variant.
    pub fields: Vec<SerdeField<'tcx>>,
}

/// Build the Serde view of a struct or enum item, or `None` for other items.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, item| {
///     let _ = serde_support::serde_item(cx, item);
/// };
/// ```
#[must_use]
pub fn serde_item<'tcx>(cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) -> Option<SerdeItem<'tcx>> {
    // Separate struct fields from enum variants before resolving derives.
    let (kind, ident, fields, variants) = if let ItemKind::Struct(ident, _, ref data) = item.kind {
        (AdtKind::Struct, ident, fields(cx, data), Vec::new())
    } else if let ItemKind::Enum(ident, _, ref definition) = item.kind {
        let variants = definition
            .variants
            .iter()
            .map(|variant| SerdeVariant {
                hir_id: variant.hir_id,
                ident: variant.ident,
                is_unit: matches!(variant.data, VariantData::Unit(..)),
                span: variant.span,
                attrs: cx.tcx.hir_attrs(variant.hir_id),
                fields: fields(cx, &variant.data),
            })
            .collect();
        (AdtKind::Enum, ident, Vec::new(), variants)
    } else {
        return None;
    };
    let def_id = item.owner_id.def_id;

    Some(SerdeItem {
        def_id,
        kind,
        ident,
        span: item.span,
        attrs: cx.tcx.hir_attrs(item.hir_id()),
        derives: derives(cx, def_id),
        fields,
        variants,
    })
}

/// Convert HIR fields into Serde field records.
fn fields<'tcx>(cx: &LateContext<'tcx>, data: &'tcx VariantData<'tcx>) -> Vec<SerdeField<'tcx>> {
    data.fields()
        .iter()
        .map(|field| field_info(cx, field))
        .collect()
}

/// Convert one HIR field into a Serde field record.
fn field_info<'tcx>(cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) -> SerdeField<'tcx> {
    // Tuple fields use their index as identifier, so only named fields keep a name.
    let name = field.ident.name.as_str();
    let is_named = !name.starts_with(|first: char| first.is_ascii_digit());
    SerdeField {
        hir_id: field.hir_id,
        def_id: field.def_id,
        name: is_named.then_some(field.ident.name),
        ty: field.ty,
        span: field.span,
        attrs: cx.tcx.hir_attrs(field.hir_id),
    }
}

/// Return every field of an item, flattening enum variant fields.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |item| {
///     let _ = serde_support::all_fields(item).count();
/// };
/// ```
pub fn all_fields<'item, 'tcx>(
    item: &'item SerdeItem<'tcx>,
) -> impl Iterator<Item = &'item SerdeField<'tcx>> {
    item.fields.iter().chain(
        item.variants
            .iter()
            .flat_map(|variant| variant.fields.iter()),
    )
}

/// Return the derived Serde and `Default` implementations of a local type.
///
/// Only implementations generated by a derive macro count, so hand-written
/// impls and lookalike traits from other crates do not.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, def_id| {
///     let _ = serde_support::derives(cx, def_id);
/// };
/// ```
#[must_use]
pub fn derives(cx: &LateContext<'_>, adt: LocalDefId) -> Derives {
    let mut derives = Derives::default();
    let key = SimplifiedType::Adt(adt.to_def_id());

    // Consider only the three relevant traits, then look up impls for this exact type.
    for &trait_def_id in cx.tcx.all_local_trait_impls(()).keys() {
        let Some(tracked) = derived_trait(cx, trait_def_id) else {
            continue;
        };
        let impls = cx
            .tcx
            .trait_impls_of(trait_def_id)
            .non_blanket_impls()
            .get(&key);
        let is_derived = impls.is_some_and(|impls| has_derived_impl(cx, impls));
        match tracked {
            DerivedTrait::Serialize => derives.has_serialize |= is_derived,
            DerivedTrait::Deserialize => derives.has_deserialize |= is_derived,
            DerivedTrait::Default => derives.has_default |= is_derived,
        }
    }

    derives
}

/// Return true when a derive macro generated one of the implementations.
fn has_derived_impl(cx: &LateContext<'_>, impls: &[DefId]) -> bool {
    impls
        .iter()
        .any(|&impl_def_id| derive_macro(cx, impl_def_id).is_some())
}

/// Trait whose derived implementation Serde lints track.
#[derive(Clone, Copy, Debug)]
enum DerivedTrait {
    /// `serde::Serialize`.
    Serialize,
    /// `serde::Deserialize`.
    Deserialize,
    /// `core::default::Default`.
    Default,
}

/// Classify a trait definition as one of the tracked traits.
fn derived_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> Option<DerivedTrait> {
    if cx.tcx.is_diagnostic_item(sym::Default, trait_def_id) {
        return Some(DerivedTrait::Default);
    }
    if is_serde_trait(cx, trait_def_id, "Serialize") {
        return Some(DerivedTrait::Serialize);
    }
    is_serde_trait(cx, trait_def_id, "Deserialize").then_some(DerivedTrait::Deserialize)
}

/// Return the derive macro that generated an implementation, if any.
///
/// The expansion chain is walked outward, so a derive used inside another
/// macro still resolves to the derive macro's definition.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, impl_def_id| {
///     let _ = serde_support::derive_macro(cx, impl_def_id);
/// };
/// ```
#[must_use]
pub fn derive_macro(cx: &LateContext<'_>, impl_def_id: DefId) -> Option<DefId> {
    let mut expansion = cx.tcx.def_span(impl_def_id).ctxt().outer_expn_data();
    loop {
        if matches!(expansion.kind, ExpnKind::Macro(MacroKind::Derive, _)) {
            return expansion.macro_def_id;
        }
        if !expansion.call_site.from_expansion() {
            return None;
        }
        expansion = expansion.call_site.ctxt().outer_expn_data();
    }
}

/// Return the local ADT an implementation's self type names, if any.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, impl_def_id| {
///     let _ = serde_support::impl_self_local_adt(cx, impl_def_id);
/// };
/// ```
#[must_use]
pub fn impl_self_local_adt(cx: &LateContext<'_>, impl_def_id: DefId) -> Option<LocalDefId> {
    if !matches!(cx.tcx.def_kind(impl_def_id), DefKind::Impl { .. }) {
        return None;
    }
    let self_ty = cx
        .tcx
        .type_of(impl_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    let ty::Adt(adt, _) = self_ty.kind() else {
        return None;
    };
    adt.did().as_local()
}

/// Return true when a definition belongs to Serde's facade or core crate.
fn is_serde_def(cx: &LateContext<'_>, def_id: DefId) -> bool {
    matches!(
        cx.tcx.crate_name(def_id.krate).as_str(),
        "serde" | "serde_core"
    )
}

/// Return the attributes named `namespace`, such as `serde` or `strum`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attrs| {
///     let _ = serde_support::namespace_attrs(attrs, "serde").count();
/// };
/// ```
pub fn namespace_attrs<'attr>(
    attrs: &'attr [Attribute],
    namespace: &str,
) -> impl Iterator<Item = &'attr Attribute> {
    let namespace = Symbol::intern(namespace);
    attrs.iter().filter(move |attr| attr.has_name(namespace))
}

/// Return the structured entries named `key` inside one attribute.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attr| {
///     let _ = serde_support::attr_entries(attr, "rename");
/// };
/// ```
pub fn attr_entries<'key>(
    attr: &Attribute,
    key: &'key str,
) -> impl Iterator<Item = MetaItemInner> + 'key {
    attr.meta_item_list()
        .unwrap_or_default()
        .into_iter()
        .filter(move |entry| entry_key(entry) == Some(key))
}

/// Return true when an attribute holds an entry named `key`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attr| {
///     let _ = serde_support::attr_has_entry(attr, "rename");
/// };
/// ```
#[must_use]
pub fn attr_has_entry(attr: &Attribute, key: &str) -> bool {
    attr_entries(attr, key).next().is_some()
}

/// Return the first `serde` attribute holding an entry named `key`.
///
/// Only structured entry names match, so a string value equal to `key` does not.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attrs| {
///     let _ = serde_support::serde_attr(attrs, "default");
/// };
/// ```
#[must_use]
pub fn serde_attr<'attr>(attrs: &'attr [Attribute], key: &str) -> Option<&'attr Attribute> {
    namespace_attrs(attrs, "serde").find(|attr| attr_has_entry(attr, key))
}

/// Return true when a `serde` attribute holds an entry named `key`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attrs| {
///     let _ = serde_support::has_serde_attr(attrs, "default");
/// };
/// ```
#[must_use]
pub fn has_serde_attr(attrs: &[Attribute], key: &str) -> bool {
    serde_attr(attrs, key).is_some()
}

/// Return true when `key` is a bare word entry, such as `#[serde(default)]`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attrs| {
///     let _ = serde_support::has_serde_word(attrs, "default");
/// };
/// ```
#[must_use]
pub fn has_serde_word(attrs: &[Attribute], key: &str) -> bool {
    namespace_has_word(attrs, "serde", key)
}

/// Return true when an attribute in `namespace` holds `key` as a bare word.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attrs| {
///     let _ = serde_support::namespace_has_word(attrs, "strum", "disabled");
/// };
/// ```
#[must_use]
pub fn namespace_has_word(attrs: &[Attribute], namespace: &str, key: &str) -> bool {
    namespace_attrs(attrs, namespace).any(|attr| {
        attr_entries(attr, key).any(|entry| {
            entry
                .meta_item()
                .is_some_and(|meta| matches!(meta.kind, MetaItemKind::Word))
        })
    })
}

/// Return every string assigned to `key` in attributes of `namespace`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attrs| {
///     let _ = serde_support::namespace_string_values(attrs, "serde", "alias");
/// };
/// ```
pub fn namespace_string_values<'attr>(
    attrs: &'attr [Attribute],
    namespace: &'attr str,
    key: &'attr str,
) -> impl Iterator<Item = String> + 'attr {
    namespace_attrs(attrs, namespace)
        .flat_map(move |attr| attr_entries(attr, key))
        .filter_map(|entry| entry_string(&entry))
}

/// Return true when an attribute holds exactly one entry and it is named `key`.
///
/// Deleting such an attribute removes only that entry.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attr| {
///     let _ = serde_support::attr_is_single_entry(attr, "borrow");
/// };
/// ```
#[must_use]
pub fn attr_is_single_entry(attr: &Attribute, key: &str) -> bool {
    matches!(attr.meta_item_list().as_deref(), Some([entry]) if entry_key(entry) == Some(key))
}

/// Serialization and deserialization values of one directional Serde entry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SerdeDirectionalValue {
    /// Value used while serializing.
    pub serialize: Option<String>,
    /// Value used while deserializing.
    pub deserialize: Option<String>,
}

/// Read a directional Serde entry from `serde` attributes.
///
/// The entry is either `key = "..."` or
/// `key(serialize = "...", deserialize = "...")`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |attrs| {
///     let _ = serde_support::serde_directional_value(attrs, "rename");
/// };
/// ```
#[must_use]
pub fn serde_directional_value(attrs: &[Attribute], key: &str) -> Option<SerdeDirectionalValue> {
    let entry = namespace_attrs(attrs, "serde")
        .flat_map(|attr| attr_entries(attr, key))
        .next()?;

    // A shared value applies to both directions; a list names each direction.
    if let Some(value) = entry_string(&entry) {
        return Some(SerdeDirectionalValue {
            serialize: Some(value.clone()),
            deserialize: Some(value),
        });
    }
    let nested = entry.meta_item_list()?;
    let direction = |name: &str| {
        nested
            .iter()
            .filter(|inner| entry_key(inner) == Some(name))
            .find_map(entry_string)
    };
    Some(SerdeDirectionalValue {
        serialize: direction("serialize"),
        deserialize: direction("deserialize"),
    })
}

/// Return the string literal of a `key = "..."` entry.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |entry| {
///     let _ = serde_support::entry_literal(entry);
/// };
/// ```
#[must_use]
pub fn entry_literal(entry: &MetaItemInner) -> Option<&MetaItemLit> {
    let MetaItemKind::NameValue(literal) = &entry.meta_item()?.kind else {
        return None;
    };
    matches!(literal.kind, LitKind::Str(..)).then_some(literal)
}

/// Return the unescaped string of a `key = "..."` entry.
fn entry_string(entry: &MetaItemInner) -> Option<String> {
    let LitKind::Str(value, _) = entry_literal(entry)?.kind else {
        return None;
    };
    Some(value.to_string())
}

/// Return the single-segment name of a meta entry.
fn entry_key(entry: &MetaItemInner) -> Option<&str> {
    let meta = entry.meta_item()?;
    let [segment] = meta.path.segments.as_slice() else {
        return None;
    };
    Some(segment.ident.name.as_str())
}

/// Return true when the attribute span covers a plain `#[serde(...)]` in user source.
///
/// Deleting the span is then exact. An attribute produced by `cfg_attr` or a
/// macro spans other text, so callers fall back to help.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attr| {
///     let _ = serde_support::is_plain_source_attr(cx, attr, "serde");
/// };
/// ```
#[must_use]
pub fn is_plain_source_attr(cx: &LateContext<'_>, attr: &Attribute, namespace: &str) -> bool {
    let span = attr.span();
    !span.from_expansion()
        && cx
            .sess()
            .source_map()
            .span_to_snippet(span)
            .is_ok_and(|source| {
                source
                    .strip_prefix("#[")
                    .and_then(|rest| rest.trim_start().strip_prefix(namespace))
                    .is_some_and(|rest| rest.trim_start().starts_with('('))
            })
}

/// Return true for the field types that `serde_derive` borrows without `#[serde(borrow)]`.
///
/// Serde decides this from the written type: `&str`, `&[u8]`, or `Option` of
/// either. A type alias for `&str` is not borrowed implicitly.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |ty| {
///     let _ = serde_support::ty_is_implicitly_borrowed(ty);
/// };
/// ```
#[must_use]
pub fn ty_is_implicitly_borrowed(ty: &Ty<'_>) -> bool {
    is_borrowed_str_or_bytes(ty) || option_argument(ty).is_some_and(is_borrowed_str_or_bytes)
}

/// Return true for a written `&str` or `&[u8]`.
fn is_borrowed_str_or_bytes(ty: &Ty<'_>) -> bool {
    let TyKind::Ref(_, referent) = ty.kind else {
        return false;
    };
    if referent.mutbl != Mutability::Not {
        return false;
    }
    is_primitive(referent.ty, PrimTy::Str) || is_u8_slice(referent.ty)
}

/// Return true for a written `[u8]`.
fn is_u8_slice(ty: &Ty<'_>) -> bool {
    matches!(ty.kind, TyKind::Slice(element) if is_primitive(element, PrimTy::Uint(rustc_ast::UintTy::U8)))
}

/// Return true for a single-segment path that resolves to `primitive`.
fn is_primitive(ty: &Ty<'_>, primitive: PrimTy) -> bool {
    matches!(
        ty.kind,
        TyKind::Path(QPath::Resolved(None, path)) if is_single_segment_res(path, Res::PrimTy(primitive))
    )
}

/// Return true for a one-segment path that resolves to `res`.
fn is_single_segment_res(path: &rustc_hir::Path<'_>, res: Res) -> bool {
    let is_single = path.segments.len() == 1;
    is_single && path.res == res
}

/// Return the type argument of a written `Option<T>`.
fn option_argument<'tcx>(ty: &'tcx Ty<'tcx>) -> Option<&'tcx Ty<'tcx>> {
    let TyKind::Path(QPath::Resolved(None, path)) = ty.kind else {
        return None;
    };
    let segment = path.segments.last()?;
    if segment.ident.name != sym::Option {
        return None;
    }
    match segment.args?.args {
        [rustc_hir::GenericArg::Type(argument)] => Some(argument.as_unambig_ty()),
        _ => None,
    }
}

/// Return true for a written `Cow<'a, str>` or `Cow<'a, [u8]>`.
///
/// The path must resolve to the standard `Cow`.
/// `serde_derive` borrows these only with `#[serde(borrow)]`, and it recognizes
/// them by the final path segment `Cow`, so a renamed import does not qualify.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, ty| {
///     let _ = serde_support::ty_is_borrowable_cow(cx, ty);
/// };
/// ```
#[must_use]
pub fn ty_is_borrowable_cow(cx: &LateContext<'_>, ty: &Ty<'_>) -> bool {
    let TyKind::Path(QPath::Resolved(None, path)) = ty.kind else {
        return false;
    };
    let Res::Def(_, def_id) = path.res else {
        return false;
    };
    let Some(segment) = path.segments.last() else {
        return false;
    };
    if segment.ident.name != sym::Cow || !cx.tcx.is_diagnostic_item(sym::Cow, def_id) {
        return false;
    }
    // Serde requires exactly a lifetime and a `str` or `[u8]` target, and `'static` cannot borrow.
    match segment.args.map(|args| args.args) {
        Some(
            [
                rustc_hir::GenericArg::Lifetime(lifetime),
                rustc_hir::GenericArg::Type(target),
            ],
        ) => {
            !lifetime.is_static()
                && (is_primitive(target.as_unambig_ty(), PrimTy::Str)
                    || is_u8_slice(target.as_unambig_ty()))
        }
        _ => false,
    }
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

/// Resolve a method call to `method_name` on the Serde trait `trait_name`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = serde_support::serde_method_call(cx, expr, "Deserializer", "deserialize_any");
/// };
/// ```
#[must_use]
pub fn serde_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    trait_name: &str,
    method_name: &str,
) -> Option<SerdeMethodCall<'hir>> {
    // Require a method call whose resolved definition is the named Serde trait method.
    let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    is_serde_trait_method(cx, def_id, trait_name, method_name).then_some(SerdeMethodCall {
        method_span: segment.ident.span,
        receiver,
        arguments,
    })
}

/// Return true when `def_id` is the method `method_name` of the Serde trait `trait_name`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, def_id| {
///     let _ = serde_support::is_serde_trait_method(cx, def_id, "Deserializer", "deserialize_any");
/// };
/// ```
#[must_use]
pub fn is_serde_trait_method(
    cx: &LateContext<'_>,
    def_id: DefId,
    trait_name: &str,
    method_name: &str,
) -> bool {
    cx.tcx.trait_of_assoc(def_id).is_some_and(|trait_def_id| {
        cx.tcx.item_name(def_id).as_str() == method_name
            && is_serde_trait(cx, trait_def_id, trait_name)
    })
}

/// Return true when `hir_id` lies in a method of a Serde trait implementation.
///
/// The implemented trait must be the Serde trait named `trait_name`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, hir_id| {
///     let _ = serde_support::is_in_serde_trait_impl(cx, hir_id, "Deserializer");
/// };
/// ```
#[must_use]
pub fn is_in_serde_trait_impl(cx: &LateContext<'_>, hir_id: HirId, trait_name: &str) -> bool {
    let owner = cx.tcx.hir_get_parent_item(hir_id).to_def_id();
    cx.tcx
        .trait_impl_of_assoc(owner)
        .is_some_and(|impl_def_id| {
            is_serde_trait(cx, cx.tcx.impl_trait_id(impl_def_id), trait_name)
        })
}

/// Return true when `trait_def_id` is the Serde trait named `trait_name`.
fn is_serde_trait(cx: &LateContext<'_>, trait_def_id: DefId, trait_name: &str) -> bool {
    is_serde_def(cx, trait_def_id) && cx.tcx.item_name(trait_def_id).as_str() == trait_name
}

/// Return true when an expression calls the standard `ToString::to_string`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = serde_support::is_to_string_call(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_to_string_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    cx.typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .and_then(|def_id| cx.tcx.trait_of_assoc(def_id))
        .is_some_and(|trait_id| {
            cx.tcx
                .is_diagnostic_item(Symbol::intern("ToString"), trait_id)
        })
}

/// Emit a lint on a HIR node so `allow` and `expect` attributes on that node apply.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, lint, hir_id, span| {
///     serde_support::emit_lint(cx, lint, hir_id, span, "message", serde_support::Help::text("help"));
/// };
/// ```
pub fn emit_lint(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    hir_id: HirId,
    span: Span,
    message: impl Into<String>,
    help: Help,
) {
    let message = message.into();
    cx.tcx.emit_node_span_lint(
        lint,
        hir_id,
        span,
        DiagDecorator(move |diag| {
            let _configured = diag.primary_message(message);
            match help {
                Help::Text(text) => {
                    let _configured = diag.help(text);
                }
                Help::Rewrite { text, parts } => {
                    let _configured =
                        diag.multipart_suggestion(text, parts, Applicability::MachineApplicable);
                }
            }
        }),
    );
}

/// Help attached to a Serde diagnostic.
#[derive(Clone, Debug)]
pub enum Help {
    /// Help text without a rewrite.
    Text(String),
    /// An exact rewrite that `cargo fix` may apply.
    Rewrite {
        /// Help text describing the rewrite.
        text: String,
        /// Source spans and their replacements.
        parts: Vec<(Span, String)>,
    },
}

impl Help {
    /// Return help text without a rewrite.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _ = serde_support::Help::text("remove it");
    /// ```
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Return a deletion of `attr` when its only entry is `key` and it comes from plain
    /// source, and help text otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |cx, attr| {
    ///     let _ = serde_support::Help::attr_deletion(cx, attr, "borrow", "remove it");
    /// };
    /// ```
    #[must_use]
    pub fn attr_deletion(
        cx: &LateContext<'_>,
        attr: &Attribute,
        key: &str,
        text: impl Into<String>,
    ) -> Self {
        let text = text.into();
        if is_deletable_attr(cx, attr, key) {
            Self::Rewrite {
                text,
                parts: vec![(attr_deletion_span(cx, attr), String::new())],
            }
        } else {
            Self::Text(text)
        }
    }
}

/// Return true when deleting `attr` removes exactly the entry `key`.
///
/// The attribute must hold only that entry and be a plain `#[serde(...)]` in
/// user source rather than the output of `cfg_attr` or a macro.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attr| {
///     let _ = serde_support::is_deletable_attr(cx, attr, "borrow");
/// };
/// ```
#[must_use]
pub fn is_deletable_attr(cx: &LateContext<'_>, attr: &Attribute, key: &str) -> bool {
    attr_is_single_entry(attr, key) && is_plain_source_attr(cx, attr, "serde")
}

/// Return the span that deletes an attribute together with the whitespace after it.
///
/// Removing the trailing whitespace keeps the next token at the attribute's
/// position, so no blank line or stray space remains.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, attr| {
///     let _ = serde_support::attr_deletion_span(cx, attr);
/// };
/// ```
#[must_use]
pub fn attr_deletion_span(cx: &LateContext<'_>, attr: &Attribute) -> Span {
    cx.sess()
        .source_map()
        .span_extend_while_whitespace(attr.span())
}
