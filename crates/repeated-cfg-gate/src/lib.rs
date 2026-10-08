#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "Syn's non-exhaustive syntax enums require a forward-compatible fallback"
)]
#![warn(unused_extern_crates)]

//! A lint to check for repeated `cfg` predicates across a crate.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use proc_macro2::Span as ProcMacroSpan;
use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext, def_id::LOCAL_CRATE};
use syn::{
    Attribute, Expr, ExprLit, ForeignItem, ImplItem, Item, ItemConst, ItemEnum, ItemExternCrate,
    ItemFn, ItemForeignMod, ItemImpl, ItemMacro, ItemMod, ItemStatic, ItemStruct, ItemTrait,
    ItemTraitAlias, ItemType, ItemUnion, ItemUse, Lit, Meta, Path as SynPath, Token, TraitItem,
    punctuated::Punctuated,
    spanned::Spanned as _,
    visit::{self, Visit},
};

/// Number of occurrences allowed before a predicate becomes a crate-wide warning.
const MAX_REPEATED_GATES: usize = 3;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub REPEATED_CFG_GATE,
    Warn,
    "the same cfg predicate is repeated across a crate",
    RepeatedCfgGate
}

impl EarlyLintPass for RepeatedCfgGate {
    /// Check every source file in the crate for repeated direct `cfg` predicates.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _krate: &Crate) {
        // Analyze source text because cfg-disabled items are absent from the parsed compiler AST.
        // Emit one diagnostic per predicate after the complete crate has been collected.
        for (predicate, occurrences) in repeated_gates(cx) {
            if occurrences.len() <= MAX_REPEATED_GATES {
                continue;
            }

            let Some(first) = occurrences.first() else {
                continue;
            };
            emit_repeated_gate_lint(cx, first.span, &predicate, occurrences.len());
        }
    }
}

/// One source occurrence of a normalized `cfg` predicate.
struct GateOccurrence {
    /// The complete attribute span used for the diagnostic location.
    span: Span,
}

/// A source file loaded for the target crate.
struct SourceCandidate {
    /// Byte position where this source file starts in rustc's source map.
    start_pos: BytePos,
    /// Source text used for syntax-aware cfg and test-region analysis.
    source: Arc<String>,
}

/// Collects repeated predicates while tracking test-only item regions.
#[derive(Default)]
struct CfgGateCollector {
    /// Predicate occurrences as source locations within one source file.
    gates: BTreeMap<CfgPredicate, Vec<SourceLocation>>,
    /// Number of enclosing test-only items currently being visited.
    test_only_depth: usize,
}

/// A normalized structured `cfg` predicate used as the repetition key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CfgPredicate {
    /// A bare predicate such as `unix`.
    Word(String),
    /// A name-value predicate such as `feature = "abc"`.
    NameValue {
        /// Predicate name.
        name: String,
        /// String-valued predicate argument.
        value: String,
    },
    /// A nested predicate list such as `all(unix, feature = "abc")`.
    List {
        /// Predicate name.
        name: String,
        /// Nested predicate arguments.
        children: Vec<Self>,
    },
}

/// One source-relative line and column span from `syn`.
#[derive(Clone, Copy)]
struct SourceLocation {
    /// One-based starting line.
    start_line: usize,
    /// Zero-based starting byte column.
    start_column: usize,
    /// One-based ending line.
    end_line: usize,
    /// Zero-based ending byte column.
    end_column: usize,
}

impl std::fmt::Display for CfgPredicate {
    /// Render the normalized predicate in diagnostic-friendly Rust syntax.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Render the predicate by shape so diagnostics preserve the structured key.
        match self {
            Self::Word(word) => formatter.write_str(word),
            Self::NameValue { name, value } => format_name_value(formatter, name, value),
            Self::List { name, children } => format_predicate_list(formatter, name, children),
        }
    }
}

/// Render one name-value predicate without changing its normalized spelling.
fn format_name_value(
    formatter: &mut std::fmt::Formatter<'_>,
    name: &str,
    value: &str,
) -> std::fmt::Result {
    // Keep the name and value separate while emitting valid cfg syntax.
    formatter.write_str(name)?;
    formatter.write_str(" = \"")?;
    formatter.write_str(value)?;
    formatter.write_str("\"")
}

/// Render one nested predicate list with canonical separators.
fn format_predicate_list(
    formatter: &mut std::fmt::Formatter<'_>,
    name: &str,
    children: &[CfgPredicate],
) -> std::fmt::Result {
    // Open the predicate list before rendering each normalized child.
    formatter.write_str(name)?;
    formatter.write_str("(")?;
    // Separate children with the canonical comma-and-space spelling.
    for (index, child) in children.iter().enumerate() {
        if index != 0 {
            formatter.write_str(", ")?;
        }
        std::fmt::Display::fmt(child, formatter)?;
    }
    formatter.write_str(")")
}

impl<'ast> Visit<'ast> for CfgGateCollector {
    /// Record top-level and nested item attributes.
    fn visit_item(&mut self, item: &'ast Item) {
        let attrs = item_attributes(item);
        let item_is_test_only = is_item_test_only(attrs);
        // Enter the test-only scope before recording this item's attributes.
        self.enter_test_only(item_is_test_only);
        // Visit child items while the enclosing test-only state is active.
        self.visit_attrs(attrs);
        visit::visit_item(self, item);
        // Restore the enclosing scope after all children have been visited.
        self.leave_test_only(item_is_test_only);
    }

    /// Record implementation-item attributes.
    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        let attrs = impl_item_attributes(item);
        let item_is_test_only = is_item_test_only(attrs);
        // Enter the test-only scope before recording this item's attributes.
        self.enter_test_only(item_is_test_only);
        // Visit child items while the enclosing test-only state is active.
        self.visit_attrs(attrs);
        visit::visit_impl_item(self, item);
        // Restore the enclosing scope after all children have been visited.
        self.leave_test_only(item_is_test_only);
    }

    /// Record trait-item attributes.
    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        let attrs = trait_item_attributes(item);
        let item_is_test_only = is_item_test_only(attrs);
        // Enter the test-only scope before recording this item's attributes.
        self.enter_test_only(item_is_test_only);
        // Visit child items while the enclosing test-only state is active.
        self.visit_attrs(attrs);
        visit::visit_trait_item(self, item);
        // Restore the enclosing scope after all children have been visited.
        self.leave_test_only(item_is_test_only);
    }

    /// Record foreign-item attributes.
    fn visit_foreign_item(&mut self, item: &'ast ForeignItem) {
        let attrs = foreign_item_attributes(item);
        let item_is_test_only = is_item_test_only(attrs);
        // Enter the test-only scope before recording this item's attributes.
        self.enter_test_only(item_is_test_only);
        // Visit child items while the enclosing test-only state is active.
        self.visit_attrs(attrs);
        visit::visit_foreign_item(self, item);
        // Restore the enclosing scope after all children have been visited.
        self.leave_test_only(item_is_test_only);
    }

    /// Record enum-variant attributes.
    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        let item_is_test_only = is_item_test_only(&variant.attrs);
        // Exclude this variant and its fields when the variant is test-only.
        self.enter_test_only(item_is_test_only);
        self.visit_attrs(&variant.attrs);
        visit::visit_variant(self, variant);
        self.leave_test_only(item_is_test_only);
    }

    /// Record struct and tuple-field attributes.
    fn visit_field(&mut self, field: &'ast syn::Field) {
        let item_is_test_only = is_item_test_only(&field.attrs);
        // Exclude this field from a test-only enclosing region when required.
        self.enter_test_only(item_is_test_only);
        self.visit_attrs(&field.attrs);
        visit::visit_field(self, field);
        self.leave_test_only(item_is_test_only);
    }
}

impl CfgGateCollector {
    /// Record direct `cfg` attributes unless the current item is test-only.
    fn visit_attrs(&mut self, attrs: &[Attribute]) {
        // A test-only ancestor excludes every nested gate from the count.
        if self.test_only_depth != 0 {
            return;
        }

        // Record only direct cfg attributes; cfg_attr remains outside this lint's profile.
        for attr in attrs {
            let Some(predicate) = cfg_predicate(attr) else {
                continue;
            };
            self.gates
                .entry(predicate)
                .or_default()
                .push(attribute_location(attr.span()));
        }
    }

    /// Enter a test-only item before visiting its attributes and children.
    const fn enter_test_only(&mut self, item_is_test_only: bool) {
        if item_is_test_only {
            self.test_only_depth += 1;
        }
    }

    /// Restore the enclosing test-only depth after visiting an item or nested AST node.
    const fn leave_test_only(&mut self, item_is_test_only: bool) {
        if item_is_test_only {
            self.test_only_depth -= 1;
        }
    }
}

/// Collect repeated predicates from all local Rust source files loaded for the crate.
fn repeated_gates(cx: &EarlyContext<'_>) -> BTreeMap<CfgPredicate, Vec<GateOccurrence>> {
    let mut gates = BTreeMap::new();

    // Parse each loaded source file so cfg-disabled items remain visible to the lint.
    for candidate in loaded_rust_source_files(cx) {
        let Ok(file) = syn::parse_file(&candidate.source) else {
            continue;
        };
        // Collect test-region state and normalized predicates within this source file.
        let mut collector = CfgGateCollector::default();
        collector.visit_file(&file);

        if collector.gates.is_empty() {
            continue;
        }

        // Index line starts once instead of rescanning the source for each occurrence.
        let lines = SourceLines::new(&candidate.source);

        // Merge this file's occurrences into the crate-wide predicate map.
        for (predicate, ranges) in collector.gates {
            let occurrences: &mut Vec<GateOccurrence> = gates.entry(predicate).or_default();
            occurrences.extend(ranges.into_iter().filter_map(|range| {
                source_span(candidate.start_pos, &lines, range).map(|span| GateOccurrence { span })
            }));
        }
    }

    gates
}

/// Locate readable local Rust files loaded for the target crate.
fn loaded_rust_source_files(cx: &EarlyContext<'_>) -> Vec<SourceCandidate> {
    // Resolve source-map paths against the package that owns the crate root.
    let source_map = cx.sess().source_map();
    let files = source_map.files();
    // Anchor package discovery at the source file rustc compiled as the crate root.
    let Some(crate_root) = cx
        .sess()
        .local_crate_source_file()
        .and_then(rustc_span::RealFileName::into_local_path)
        .and_then(|path| {
            if path.is_absolute() {
                Some(path)
            } else {
                std::env::current_dir()
                    .ok()
                    .map(|directory| directory.join(path))
            }
        })
        // Stop at the nearest manifest so sibling workspace packages remain excluded.
        .and_then(|path| {
            path.parent()?
                .ancestors()
                .find(|directory| directory.join("Cargo.toml").is_file())
                .map(Path::to_path_buf)
        })
    else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for source_file in files.iter() {
        // Files imported from dependency metadata belong to other crates.
        if source_file.cnum != LOCAL_CRATE {
            continue;
        }

        // De-duplicate source-map entries under the package root.
        let Some(path) = local_source_file_path(source_file, &crate_root) else {
            continue;
        };
        if !seen.insert(path.clone()) || !is_rust_source_path(&path) {
            continue;
        }

        // Share rustc's retained source; an attribute-free file cannot contain cfg gates.
        let Some(source) = source_file
            .src
            .as_ref()
            .filter(|source| source.contains('#'))
        else {
            continue;
        };
        candidates.push(SourceCandidate {
            start_pos: source_file.start_pos,
            source: Arc::clone(source),
        });
    }

    // Release the source-map guard before callers parse and retain source text.
    drop(files);

    candidates
}

/// Resolve one source-map file to a local path inside the target crate.
fn local_source_file_path(source_file: &SourceFile, crate_root: &Path) -> Option<PathBuf> {
    let path = source_file.name.clone().into_local_path()?;
    crate_local_path(path, crate_root)
}

/// Retain a path only when it belongs to the target crate tree.
fn crate_local_path(path: PathBuf, crate_root: &Path) -> Option<PathBuf> {
    // Normalize both paths before checking containment so parent segments cannot escape.
    let crate_root = normalize_path(crate_root);
    let path = if path.is_absolute() {
        path
    } else {
        // Resolve relative source-map names under the package root.
        crate_root.join(path)
    };
    let path = normalize_path(&path);
    path.starts_with(&crate_root).then_some(path)
}

/// Normalize lexical `.` and `..` components without requiring a file to exist.
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    // Preserve roots while folding lexical current- and parent-directory components.
    // Keep the normalized path independent of filesystem existence.
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            Component::Normal(component) => normalized.push(component),
            Component::Prefix(_) | Component::RootDir => normalized.push(component.as_os_str()),
        }
    }

    normalized
}

/// Return whether a source-map path has a Rust source extension.
fn is_rust_source_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| matches!(extension.to_str(), Some("rs" | "in")))
}

/// Return the attributes attached to a parsed Rust item.
fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(ItemConst { attrs, .. })
        | Item::Enum(ItemEnum { attrs, .. })
        | Item::ExternCrate(ItemExternCrate { attrs, .. })
        | Item::Fn(ItemFn { attrs, .. })
        | Item::ForeignMod(ItemForeignMod { attrs, .. })
        | Item::Impl(ItemImpl { attrs, .. })
        | Item::Macro(ItemMacro { attrs, .. })
        | Item::Mod(ItemMod { attrs, .. })
        | Item::Static(ItemStatic { attrs, .. })
        | Item::Struct(ItemStruct { attrs, .. })
        | Item::Trait(ItemTrait { attrs, .. })
        | Item::TraitAlias(ItemTraitAlias { attrs, .. })
        | Item::Type(ItemType { attrs, .. })
        | Item::Union(ItemUnion { attrs, .. })
        | Item::Use(ItemUse { attrs, .. }) => attrs,
        _ => &[],
    }
}

/// Return the attributes attached to a parsed implementation item.
fn impl_item_attributes(item: &ImplItem) -> &[Attribute] {
    match item {
        ImplItem::Const(item) => &item.attrs,
        ImplItem::Fn(item) => &item.attrs,
        ImplItem::Macro(item) => &item.attrs,
        ImplItem::Type(item) => &item.attrs,
        _ => &[],
    }
}

/// Return the attributes attached to a parsed trait item.
fn trait_item_attributes(item: &TraitItem) -> &[Attribute] {
    match item {
        TraitItem::Const(item) => &item.attrs,
        TraitItem::Fn(item) => &item.attrs,
        TraitItem::Macro(item) => &item.attrs,
        TraitItem::Type(item) => &item.attrs,
        _ => &[],
    }
}

/// Return the attributes attached to a parsed foreign item.
fn foreign_item_attributes(item: &ForeignItem) -> &[Attribute] {
    match item {
        ForeignItem::Fn(item) => &item.attrs,
        ForeignItem::Macro(item) => &item.attrs,
        ForeignItem::Static(item) => &item.attrs,
        ForeignItem::Type(item) => &item.attrs,
        _ => &[],
    }
}

/// Build a rustc span from a source-relative line and column location.
fn source_span(
    start_pos: BytePos,
    source: &SourceLines<'_>,
    location: SourceLocation,
) -> Option<Span> {
    // Convert both endpoints before constructing the diagnostic span.
    source
        .offset(location.start_line, location.start_column)
        .zip(source.offset(location.end_line, location.end_column))
        // Reject malformed or reversed source locations before applying the file offset.
        .filter(|(start, end)| start <= end)
        // Guard integer conversion and source-map offset addition independently.
        .and_then(|(start, end)| u32::try_from(start).ok().zip(u32::try_from(end).ok()))
        .and_then(|(start, end)| {
            start_pos
                .0
                .checked_add(start)
                .zip(start_pos.0.checked_add(end))
                .map(|(lo, hi)| Span::new(BytePos(lo), BytePos(hi), SyntaxContext::root(), None))
        })
}

/// Byte offsets for the start of each physical source line.
struct SourceLines<'source> {
    /// Original source used to validate UTF-8 boundaries.
    source: &'source str,
    /// One entry per line, including an empty trailing line.
    starts: Vec<usize>,
}

impl<'source> SourceLines<'source> {
    /// Build one line index for all diagnostic locations in a file.
    fn new(source: &'source str) -> Self {
        let starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(offset, _)| offset + 1))
            .collect();
        Self { source, starts }
    }

    /// Convert a one-based line and zero-based byte column into a source offset.
    fn offset(&self, line: usize, column: usize) -> Option<usize> {
        let start = *self.starts.get(line.checked_sub(1)?)?;
        // Exclude the newline, while retaining the original CRLF column semantics.
        let end = self
            .starts
            .get(line)
            .map_or(self.source.len(), |next| next - 1);
        let offset = start.checked_add(column)?;
        (offset <= end && self.source.is_char_boundary(offset)).then_some(offset)
    }
}

/// Convert a `syn` span into a source-relative line and column location.
fn attribute_location(span: ProcMacroSpan) -> SourceLocation {
    let start = span.start();
    let end = span.end();
    SourceLocation {
        start_line: start.line,
        start_column: start.column,
        end_line: end.line,
        end_column: end.column,
    }
}

/// Return whether an attribute list makes its item test-only.
fn is_item_test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
            || cfg_requires_test(attr)
    })
}

/// Return whether a direct `cfg` predicate can only be true for test builds.
fn cfg_requires_test(attr: &Attribute) -> bool {
    // Only direct cfg attributes can establish a test-only region here.
    if !attr.path().is_ident("cfg") {
        return false;
    }

    // Parse the cfg predicate before applying its test-only truth rule.
    let Meta::List(arguments) = &attr.meta else {
        return false;
    };
    let Ok(predicate) = arguments.parse_args::<Meta>() else {
        return false;
    };

    predicate_requires_test(&predicate)
}

/// Evaluate whether every valid branch of a predicate requires `test`.
fn predicate_requires_test(predicate: &Meta) -> bool {
    match predicate {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) if list.path.is_ident("all") => {
            cfg_children(list).is_some_and(|children| children.iter().any(predicate_requires_test))
        }
        Meta::List(list) if list.path.is_ident("any") => {
            cfg_children(list).is_some_and(|children| {
                !children.is_empty() && children.iter().all(predicate_requires_test)
            })
        }
        Meta::List(_) | Meta::NameValue(_) => false,
    }
}

/// Parse the comma-separated child predicates of `all` or `any`.
fn cfg_children(list: &syn::MetaList) -> Option<Punctuated<Meta, Token![,]>> {
    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .ok()
}

/// Parse one direct `cfg` attribute into a normalized predicate key.
fn cfg_predicate(attr: &Attribute) -> Option<CfgPredicate> {
    // Ignore every attribute family except direct cfg gates.
    if !attr.path().is_ident("cfg") {
        return None;
    }

    // Parse the cfg body into a structured predicate before normalizing it.
    let Meta::List(arguments) = &attr.meta else {
        return None;
    };
    let predicate = arguments.parse_args::<Meta>().ok()?;
    predicate_key(&predicate)
}

/// Convert one structured meta item into its repetition key.
fn predicate_key(predicate: &Meta) -> Option<CfgPredicate> {
    // Preserve predicate shape so bare and name-value gates remain distinct.
    match predicate {
        Meta::Path(path) => Some(CfgPredicate::Word(path_name(path))),
        Meta::NameValue(name_value) => Some(CfgPredicate::NameValue {
            name: path_name(&name_value.path),
            value: literal_value(&name_value.value)?,
        }),
        Meta::List(arguments) => {
            let name = path_name(&arguments.path);
            // Parse nested predicates and reject unsupported child expressions.
            let mut children = arguments
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()?
                .iter()
                .map(predicate_key)
                .collect::<Option<Vec<_>>>()?;
            if matches!(name.as_str(), "all" | "any") {
                children.sort_unstable();
            }
            Some(CfgPredicate::List { name, children })
        }
    }
}

/// Return one string literal's semantic value for a name-value predicate.
fn literal_value(expression: &Expr) -> Option<String> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(literal),
        ..
    }) = expression
    else {
        return None;
    };
    Some(literal.value())
}

/// Join the identifiers in a meta-item path.
fn path_name(path: &SynPath) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Emit the single diagnostic for one predicate that exceeds the repetition limit.
fn emit_repeated_gate_lint(
    cx: &EarlyContext<'_>,
    span: Span,
    predicate: &CfgPredicate,
    count: usize,
) {
    cx.emit_span_lint(
        REPEATED_CFG_GATE,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(format!(
                "`cfg({predicate})` is repeated {count} times across this crate"
            ));
            let _ = diag.help("move the gated items into a dedicated module");
        }),
    );
}

/// Run the UI fixture suite.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::{
        CfgGateCollector, CfgPredicate, SourceLines, SourceLocation, cfg_predicate,
        crate_local_path, is_item_test_only, is_rust_source_path, source_span,
    };
    use std::path::{Path, PathBuf};
    use syn::Attribute;
    use syn::visit::Visit;

    /// Parse the first attribute in a source snippet.
    fn attribute(source: &str) -> Attribute {
        let mut attributes = syn::parse::Parser::parse_str(Attribute::parse_outer, source).unwrap();
        attributes.pop().unwrap()
    }

    /// Equivalent ordering in `all` predicates uses one repetition key.
    #[test]
    fn normalizes_commutative_predicates() {
        let first = cfg_predicate(&attribute("#[cfg(all(unix, feature = \"abc\"))]"));
        let second = cfg_predicate(&attribute("#[cfg(all(feature = \"abc\", unix))]"));

        assert_eq!(first, second);
    }

    /// Test-only items exclude their nested gates from the crate count.
    #[test]
    fn identifies_test_only_items() {
        assert!(is_item_test_only(&[attribute("#[cfg(test)]")]));
        assert!(is_item_test_only(&[attribute("#[tokio::test]")]));
        assert!(!is_item_test_only(&[attribute(
            "#[cfg(feature = \"abc\")]"
        )]));
    }

    /// A feature predicate remains distinct from a bare predicate.
    #[test]
    fn keeps_predicate_shapes_distinct() {
        let feature = cfg_predicate(&attribute("#[cfg(feature = \"abc\")]")).unwrap();
        let bare = cfg_predicate(&attribute("#[cfg(abc)]")).unwrap();

        assert!(matches!(feature, CfgPredicate::NameValue { .. }));
        assert!(matches!(bare, CfgPredicate::Word(_)));
    }

    /// Non-string name-value predicates are ignored conservatively.
    #[test]
    fn rejects_non_string_predicates() {
        assert_eq!(cfg_predicate(&attribute("#[cfg(feature = 1)]")), None);
    }

    /// Nested item visitors record gates while excluding test-only descendants.
    #[test]
    fn visits_nested_item_shapes() {
        let file = syn::parse_file(
            "#[cfg(unix)] struct Value { #[cfg(windows)] field: u8 }\
             #[cfg(feature = \"x\")] enum State { #[cfg(unix)] Ready }\
             #[cfg(target_os = \"linux\")] impl Value { #[cfg(unix)] fn value(&self) {} }\
             #[cfg(any(unix, windows))] trait Contract { #[cfg(unix)] type Value; }\
             #[allow(dead_code)] const COUNT: usize = 1;\
             #[cfg(test)] mod tests { #[cfg(unix)] fn ignored() {} }\
             unsafe extern \"C\" { #[cfg(unix)] fn foreign(); }",
        )
        .unwrap();
        let mut collector = CfgGateCollector::default();
        collector.visit_file(&file);

        assert_eq!(collector.gates.len(), 5);
    }

    /// Associated and foreign item visitors retain direct cfg attributes.
    #[test]
    fn visits_associated_item_shapes() {
        let file = syn::parse_file(
            "struct Value;\
             impl Value { #[cfg(unix)] const COUNT: usize = 1;\
                 #[cfg(unix)] fn value(&self) {}\
                 #[cfg(unix)] type Alias = usize;\
                 #[cfg(unix)] make_item!(); }\
             trait Contract { #[cfg(unix)] const COUNT: usize;\
                 #[cfg(unix)] fn value(&self);\
                 #[cfg(unix)] type Alias;\
                 #[cfg(unix)] make_item!(); }\
             unsafe extern \"C\" { #[cfg(unix)] static COUNT: usize;\
                 #[cfg(unix)] type Foreign; #[cfg(unix)] fn foreign(); }",
        )
        .unwrap();
        let mut collector = CfgGateCollector::default();
        collector.visit_file(&file);

        assert_eq!(collector.gates.len(), 1);
    }

    /// Test-only logical predicates cover the nested `all` form.
    #[test]
    fn recognizes_nested_test_predicates() {
        assert!(is_item_test_only(&[attribute("#[cfg(all(test, unix))]")]));
    }

    /// Invalid cfg attributes do not establish test-only regions.
    #[test]
    fn rejects_invalid_test_predicates() {
        assert!(!is_item_test_only(&[attribute("#[cfg]")]));
    }

    /// An `any` predicate is test-only only when every branch requires test mode.
    #[test]
    fn rejects_mixed_any_test_predicates() {
        assert!(!is_item_test_only(&[attribute("#[cfg(any(test, unix))]")]));
    }

    /// Invalid nested cfg syntax is ignored conservatively.
    #[test]
    fn rejects_malformed_cfg_predicates() {
        assert_eq!(cfg_predicate(&attribute("#[cfg(all(,))]")), None);
        assert!(!is_item_test_only(&[attribute("#[cfg(all(,))]")]));
    }

    /// Non-cfg attributes are ignored by predicate extraction.
    #[test]
    fn ignores_non_cfg_attributes() {
        assert_eq!(cfg_predicate(&attribute("#[allow(dead_code)]")), None);
    }

    /// Source offsets accept valid UTF-8 boundaries.
    #[test]
    fn validates_source_offsets() {
        assert_eq!(SourceLines::new("é\nvalue").offset(1, 2), Some(2));
    }

    /// Source offsets reject a byte column in the middle of a UTF-8 code point.
    #[test]
    fn rejects_non_boundary_offsets() {
        assert_eq!(SourceLines::new("é").offset(1, 1), None);
    }

    /// Line offsets handle an empty trailing line and invalid line numbers.
    #[test]
    fn handles_empty_trailing_lines() {
        assert_eq!(SourceLines::new("value\n").offset(2, 0), Some(6));
    }

    /// Line offsets reject a zero or missing line.
    #[test]
    fn rejects_missing_lines() {
        assert_eq!(SourceLines::new("value").offset(0, 0), None);
    }

    /// Indexed lookups preserve empty lines, CRLF and invalid-column behavior.
    #[test_case::test_case(1, 3, Some(3); "CRLF boundary")]
    #[test_case::test_case(1, 4, None; "past CRLF boundary")]
    #[test_case::test_case(2, 0, Some(4); "empty line")]
    #[test_case::test_case(2, 1, None; "past empty line")]
    #[test_case::test_case(3, 3, Some(8); "last line end")]
    #[test_case::test_case(3, usize::MAX, None; "overflowing column")]
    #[test_case::test_case(4, 0, None; "missing line")]
    fn checks_indexed_line_boundaries(line: usize, column: usize, expected: Option<usize>) {
        assert_eq!(
            SourceLines::new("é\r\n\nend").offset(line, column),
            expected
        );
    }

    /// Empty source still has a valid initial byte position.
    #[test]
    fn indexes_empty_source() {
        assert_eq!(SourceLines::new("").offset(1, 0), Some(0));
    }

    /// Valid source locations are converted into rustc spans.
    #[test]
    fn builds_source_spans() {
        let location = SourceLocation {
            start_line: 1,
            start_column: 0,
            end_line: 1,
            end_column: 5,
        };
        assert!(source_span(super::BytePos(10), &SourceLines::new("value"), location).is_some());
    }

    /// Source spans reject reversed source locations.
    #[test]
    fn rejects_reversed_source_spans() {
        let location = SourceLocation {
            start_line: 1,
            start_column: 2,
            end_line: 1,
            end_column: 1,
        };
        assert!(source_span(super::BytePos(0), &SourceLines::new("value"), location).is_none());
    }

    /// Rust source extensions include `.rs` and `.in` but exclude other files.
    #[test]
    fn identifies_rust_source_extensions() {
        assert!(is_rust_source_path(Path::new("module.rs")));
        assert!(is_rust_source_path(Path::new("module.in")));
        assert!(!is_rust_source_path(Path::new("module.txt")));
    }

    /// Path normalization folds lexical current and parent components.
    #[test]
    fn normalizes_source_paths() {
        let path = super::normalize_path(Path::new("/crate/./src/../lib.rs"));
        assert_eq!(path, PathBuf::from("/crate/lib.rs"));
    }

    /// Relative source paths resolve under the crate root and escapes are rejected.
    #[test]
    fn checks_crate_path_containment() {
        let root = Path::new("/crate");
        assert!(crate_local_path(PathBuf::from("src/lib.rs"), root).is_some());
        assert!(crate_local_path(PathBuf::from("../other/lib.rs"), root).is_none());
    }
}
