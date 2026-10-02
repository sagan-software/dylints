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
};

use proc_macro2::Span as ProcMacroSpan;
use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext};
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
    source: String,
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

        // Merge this file's occurrences into the crate-wide predicate map.
        for (predicate, ranges) in collector.gates {
            let occurrences: &mut Vec<GateOccurrence> = gates.entry(predicate).or_default();
            occurrences.extend(ranges.into_iter().filter_map(|range| {
                source_span(candidate.start_pos, &candidate.source, range)
                    .map(|span| GateOccurrence { span })
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
    let Some(crate_root) = crate_package_root(cx) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for source_file in files.iter() {
        // De-duplicate source-map entries and exclude Dylint's local toolchain files.
        let Some(path) = local_source_file_path(source_file, &crate_root) else {
            continue;
        };
        if !seen.insert(path.clone()) || !is_rust_source_path(&path) || is_toolchain_source(&path) {
            continue;
        }

        // Copy source text while the source-map guard is still held.
        let Some(source) = source_file.src.as_deref() else {
            continue;
        };
        candidates.push(SourceCandidate {
            start_pos: source_file.start_pos,
            source: source.to_owned(),
        });
    }

    // Release the source-map guard before callers parse and retain source text.
    drop(files);

    candidates
}

/// Find the Cargo package root that owns the compiler's crate-root source file.
fn crate_package_root(cx: &EarlyContext<'_>) -> Option<PathBuf> {
    // Anchor package discovery at the source file rustc compiled as the crate root.
    cx.sess()
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

/// Return whether the path belongs to the workspace-local Dylint toolchain.
fn is_toolchain_source(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == ".rustup-dylint")
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
fn source_span(start_pos: BytePos, source: &str, location: SourceLocation) -> Option<Span> {
    // Convert both endpoints before constructing the diagnostic span.
    line_column_offset(source, location.start_line, location.start_column)
        .zip(line_column_offset(
            source,
            location.end_line,
            location.end_column,
        ))
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

/// Convert a one-based line and zero-based byte column into a source offset.
fn line_column_offset(source: &str, line: usize, column: usize) -> Option<usize> {
    // Reject invalid one-based line numbers before scanning source text.
    if line == 0 {
        return None;
    }

    // Track the byte offset of each line while preserving UTF-8 boundaries.
    let mut current_line = 1;
    let mut line_start = 0;
    for segment in source.split_inclusive('\n') {
        if current_line == line {
            // Exclude the newline itself from the valid byte-column range.
            let line_end = line_start + segment.trim_end_matches('\n').len();
            let offset = line_start.checked_add(column)?;
            return (offset <= line_end && source.is_char_boundary(offset)).then_some(offset);
        }
        line_start += segment.len();
        current_line += 1;
    }

    if current_line == line && line_start == source.len() {
        return (column == 0).then_some(line_start);
    }

    None
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
    use super::{CfgPredicate, cfg_predicate, is_item_test_only};
    use syn::Attribute;

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
}
