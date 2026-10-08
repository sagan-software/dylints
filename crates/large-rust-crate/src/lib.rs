#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated syntax variants"
)]

//! A lint to check combined Rust crate source size.
//!
//! It resolves the current Cargo package, parses local Rust sources, excludes
//! configured support and test paths, and totals source bytes and lines across
//! the crate. The diagnostic reports the stable source root and size policy so
//! maintainers can split an oversized crate without counting generated or
//! dependency artifacts.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    ops::RangeInclusive,
    path::{Component, Path, PathBuf},
};

use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext, def_id::LOCAL_CRATE};
use serde::Deserialize;
use syn::{
    Attribute, ForeignItem, ImplItem, Item, Meta, Token, TraitItem,
    punctuated::Punctuated,
    spanned::Spanned as _,
    visit::{self, Visit},
};

/// Default non-test line limit for one crate.
const DEFAULT_NON_TEST_LINE_LIMIT: usize = 20_000;

/// Default total line limit for one crate.
const DEFAULT_TOTAL_LINE_LIMIT: usize = 40_000;

dylint_support::documented_early_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub LARGE_RUST_CRATE,
    Warn,
    "Rust crate exceeds its configured source line limit",
    LargeRustCrate,
    LargeRustCrate::default()
}

/// Configuration read from the target workspace's `dylint.toml` table.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Maximum number of physical non-test lines allowed in one crate.
    #[serde(default = "default_non_test_line_limit")]
    non_test_line_limit: usize,
    /// Maximum number of physical lines allowed in one crate, including tests.
    #[serde(default = "default_total_line_limit")]
    total_line_limit: usize,
}

impl Default for Config {
    /// Construct the documented default crate line limits.
    fn default() -> Self {
        Self {
            non_test_line_limit: DEFAULT_NON_TEST_LINE_LIMIT,
            total_line_limit: DEFAULT_TOTAL_LINE_LIMIT,
        }
    }
}

/// Return the default non-test line limit for serde configuration.
const fn default_non_test_line_limit() -> usize {
    DEFAULT_NON_TEST_LINE_LIMIT
}

/// Return the default total line limit for serde configuration.
const fn default_total_line_limit() -> usize {
    DEFAULT_TOTAL_LINE_LIMIT
}

/// Lint pass that retains the configured crate line limits.
#[derive(Debug)]
struct LargeRustCrate {
    /// Active line limits for this compilation.
    config: Config,
}

impl Default for LargeRustCrate {
    /// Construct the pass from target workspace configuration or its defaults.
    fn default() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
        }
    }
}

impl EarlyLintPass for LargeRustCrate {
    /// Check the combined loaded source for this crate.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Aggregate the complete loaded source tree before selecting one diagnostic.
        let Some(stats) = crate_stats(cx, krate) else {
            return;
        };
        let Some(violation) = stats.violation(&self.config) else {
            return;
        };

        emit_crate_lint(cx, crate_first_line_span(cx, krate), violation);
    }
}

/// Aggregate source-line counts for one target crate.
#[derive(Default)]
struct CrateStats {
    /// Total physical lines across all loaded local Rust files.
    total_line_count: usize,
    /// Physical lines outside test-only items across all loaded local Rust files.
    non_test_line_count: usize,
}

impl CrateStats {
    /// Add one source file's counts to the crate aggregate.
    const fn add(&mut self, file: &FileStats) {
        self.total_line_count = self.total_line_count.saturating_add(file.total_line_count);
        self.non_test_line_count = self
            .non_test_line_count
            .saturating_add(file.non_test_line_count);
    }

    /// Select one violation, preferring the unconditional total limit.
    const fn violation(&self, config: &Config) -> Option<CrateSizeViolation> {
        if self.total_line_count > config.total_line_limit {
            Some(CrateSizeViolation::TotalLines {
                count: self.total_line_count,
                limit: config.total_line_limit,
            })
        } else if self.non_test_line_count > config.non_test_line_limit {
            Some(CrateSizeViolation::NonTestLines {
                count: self.non_test_line_count,
                limit: config.non_test_line_limit,
            })
        } else {
            None
        }
    }
}

/// Reason the crate exceeds one configured line limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CrateSizeViolation {
    /// The crate exceeds its total physical-line limit.
    TotalLines {
        /// Observed total physical lines.
        count: usize,
        /// Configured total physical-line limit.
        limit: usize,
    },
    /// The crate exceeds its non-test physical-line limit.
    NonTestLines {
        /// Observed non-test physical lines.
        count: usize,
        /// Configured non-test physical-line limit.
        limit: usize,
    },
}

/// State used to locate one local source file.
struct SourceCandidate {
    /// Path used for file-system source analysis.
    path: PathBuf,
}

/// Counts for one Rust source file.
struct FileStats {
    /// Total physical lines in the source file.
    total_line_count: usize,
    /// Physical lines outside test-only items.
    non_test_line_count: usize,
}

/// Source line ranges occupied by test-only items.
#[derive(Default)]
struct TestLineCollector {
    /// Inclusive, one-based line ranges for test-only items.
    ranges: Vec<RangeInclusive<usize>>,
}

/// Aggregate every readable local Rust file loaded for the target crate.
fn crate_stats(cx: &EarlyContext<'_>, krate: &Crate) -> Option<CrateStats> {
    let mut stats = CrateStats::default();
    let mut found_source = false;

    // Skip unreadable paths while retaining every readable local source file.
    for candidate in loaded_rust_source_files(cx, krate) {
        let Some(file) = file_stats(&candidate.path) else {
            continue;
        };

        found_source = true;
        stats.add(&file);
    }

    // Avoid a warning when rustc supplied no readable local source files.
    found_source.then_some(stats)
}

/// Locate the readable local Rust files loaded for one target crate.
fn loaded_rust_source_files(cx: &EarlyContext<'_>, krate: &Crate) -> Vec<SourceCandidate> {
    // Resolve source-map paths against the package that owns the crate root.
    let source_map = cx.sess().source_map();
    let files = source_map.files();
    let Some(crate_root) = crate_package_root(cx, krate) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for source_file in files.iter() {
        // Files imported from dependency metadata belong to other crates.
        if source_file.cnum != LOCAL_CRATE {
            continue;
        }

        // Retain unique local Rust files under the package root.
        let Some(path) = local_source_file_path(source_file, &crate_root) else {
            continue;
        };

        if !seen.insert(path.clone()) || !is_rust_file(&path) {
            continue;
        }

        candidates.push(SourceCandidate { path });
    }

    // Release the source-map guard before callers begin file I/O.
    drop(files);

    candidates
}

/// Find the Cargo package root that owns the compiler's crate-root source file.
fn crate_package_root(cx: &EarlyContext<'_>, krate: &Crate) -> Option<PathBuf> {
    // Anchor package discovery at the source file that rustc compiled as the crate root.
    let source_file = cx
        .sess()
        .source_map()
        .lookup_source_file(krate.spans.inner_span.lo());
    let path = source_file.name.clone().into_local_path()?;
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().ok()?.join(path)
    };

    // Stop at the nearest manifest so sibling workspace packages remain excluded.
    for directory in path.parent()?.ancestors() {
        if directory.join("Cargo.toml").is_file() {
            return Some(directory.to_path_buf());
        }
    }

    None
}

/// Resolve one source-map file to a local path inside the target crate.
fn local_source_file_path(source_file: &SourceFile, crate_root: &Path) -> Option<PathBuf> {
    // Virtual, remapped, and imported paths may not be readable on the local host.
    let path = source_file.name.clone().into_local_path()?;
    crate_local_path(path, crate_root)
}

/// Resolve one path and retain it only when it belongs to the target crate tree.
fn crate_local_path(path: PathBuf, crate_root: &Path) -> Option<PathBuf> {
    // Normalize both paths before checking containment so lexical parent segments cannot escape.
    let crate_root = normalize_path(crate_root);
    // Resolve relative source-map names under the package root.
    let path = if path.is_absolute() {
        path
    } else {
        crate_root.join(path)
    };
    let path = normalize_path(&path);
    path.starts_with(&crate_root).then_some(path)
}

/// Normalize lexical `.` and `..` components without requiring a file to exist.
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    // Preserve roots while folding lexical current- and parent-directory components.
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

    // Return a stable lexical path for containment and file access.
    normalized
}

/// Read and classify one Rust source file.
fn file_stats(path: &Path) -> Option<FileStats> {
    let mut source = String::new();

    // Read the local file so counts follow checked-out source rather than remapped text.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut source).ok()?;

    Some(file_stats_from_source(&source))
}

/// Compute total and non-test physical line counts for one Rust source file.
fn file_stats_from_source(source: &str) -> FileStats {
    let total_line_count = source.lines().count();
    let test_line_count = test_line_count(source, total_line_count);

    FileStats {
        total_line_count,
        non_test_line_count: total_line_count.saturating_sub(test_line_count),
    }
}

/// Count unique physical lines spanned by test-only Rust items.
fn test_line_count(source: &str, total_line_count: usize) -> usize {
    // Every recognized test-only marker is an attribute and therefore needs `#`.
    // This is only a negative filter; sources containing it still use the parser.
    if !source.contains('#') {
        return 0;
    }

    // Parse the file and collect every item whose attributes require test mode.
    let Ok(file) = syn::parse_file(source) else {
        return 0;
    };
    let mut collector = TestLineCollector::default();
    collector.visit_file(&file);
    let mut test_lines = vec![false; total_line_count.saturating_add(1)];

    // Mark line membership because nested or adjacent test attributes can overlap.
    // Clamp spans to the physical file length before indexing the bitmap.
    for range in collector.ranges {
        let start = (*range.start()).max(1);
        let end = (*range.end()).min(total_line_count);
        for line in start..=end {
            let Some(is_test_line) = test_lines.get_mut(line) else {
                continue;
            };
            *is_test_line = true;
        }
    }

    test_lines
        .into_iter()
        .filter(|is_test_line| *is_test_line)
        .count()
}

impl TestLineCollector {
    /// Record the physical lines spanned by one test-only item.
    fn record_test_item(&mut self, attrs: &[Attribute], span: proc_macro2::Span) {
        let item_start = span.start().line;
        let start = attrs
            .first()
            .map_or(item_start, |attr| attr.span().start().line);
        self.ranges.push(start..=span.end().line);
    }
}

impl<'ast> Visit<'ast> for TestLineCollector {
    /// Visit one top-level or module item.
    fn visit_item(&mut self, item: &'ast Item) {
        let attrs = item_attributes(item);
        if is_test_only(attrs) {
            self.record_test_item(attrs, item.span());
        } else {
            visit::visit_item(self, item);
        }
    }

    /// Visit one item inside an implementation block.
    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        let attrs = impl_item_attributes(item);
        if is_test_only(attrs) {
            self.record_test_item(attrs, item.span());
        } else {
            visit::visit_impl_item(self, item);
        }
    }

    /// Visit one item inside a trait definition.
    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        let attrs = trait_item_attributes(item);
        if is_test_only(attrs) {
            self.record_test_item(attrs, item.span());
        } else {
            visit::visit_trait_item(self, item);
        }
    }

    /// Visit one item inside an external block.
    fn visit_foreign_item(&mut self, item: &'ast ForeignItem) {
        let attrs = foreign_item_attributes(item);
        if is_test_only(attrs) {
            self.record_test_item(attrs, item.span());
        } else {
            visit::visit_foreign_item(self, item);
        }
    }
}

/// Return whether any attribute makes its item test-only.
fn is_test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(test_only_attribute)
}

/// Return the attributes attached to a parsed Rust item.
fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        _ => remaining_item_attributes(item),
    }
}

/// Return attributes for the remaining parsed item variants.
fn remaining_item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
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

/// Return whether one attribute makes its item test-only.
fn test_only_attribute(attr: &Attribute) -> bool {
    is_test_attribute(attr) || cfg_attribute_requires_test(attr)
}

/// Return whether an attribute path ends in the test marker.
fn is_test_attribute(attr: &Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
}

/// Return whether a `cfg` predicate can only be true when `test` is true.
fn cfg_attribute_requires_test(attr: &Attribute) -> bool {
    // Parse only list-form cfg attributes into one predicate tree.
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let Meta::List(list) = &attr.meta else {
        return false;
    };
    let Ok(predicate) = list.parse_args::<Meta>() else {
        return false;
    };

    // Evaluate whether every valid branch requires the test predicate.
    cfg_predicate_requires_test(&predicate)
}

/// Evaluate whether one parsed `cfg` predicate logically requires `test`.
fn cfg_predicate_requires_test(predicate: &Meta) -> bool {
    match predicate {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) if list.path.is_ident("all") => cfg_children(list)
            .is_some_and(|children| children.iter().any(cfg_predicate_requires_test)),
        Meta::List(list) if list.path.is_ident("any") => {
            cfg_children(list).is_some_and(|children| {
                !children.is_empty() && children.iter().all(cfg_predicate_requires_test)
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

/// Return the first physical source-line span of the crate root.
fn crate_first_line_span(cx: &EarlyContext<'_>, krate: &Crate) -> Span {
    let source_file = cx
        .sess()
        .source_map()
        .lookup_source_file(krate.spans.inner_span.lo());
    let first_line_len = source_file
        .src
        .as_deref()
        .map_or(1, |source| first_line_len(source));

    first_line_span(source_file.start_pos, first_line_len)
}

/// Return the first line length for one source file.
fn first_line_len(source: &str) -> u32 {
    source
        .lines()
        .next()
        .map_or(1, |line| u32::try_from(line.len()).unwrap_or(u32::MAX))
        .max(1)
}

/// Build a span covering the first physical source line.
fn first_line_span(start_pos: BytePos, first_line_len: u32) -> Span {
    let lo = start_pos;
    let hi = BytePos(lo.0.saturating_add(first_line_len));

    Span::new(lo, hi, SyntaxContext::root(), None)
}

/// Return whether one path points to a Rust source file.
fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
}

/// Emit the selected crate-size diagnostic.
fn emit_crate_lint(cx: &EarlyContext<'_>, span: Span, violation: CrateSizeViolation) {
    // Keep total size as the primary diagnostic when both configured limits fail.
    // Render the message for the selected total-line or non-test-line violation.
    let message = match violation {
        CrateSizeViolation::TotalLines { count, limit } => {
            format!("Rust crate has {count} total lines (limit: {limit})")
        }
        CrateSizeViolation::NonTestLines { count, limit } => {
            format!("Rust crate has {count} non-test lines (limit: {limit})")
        }
    };

    // Point at the crate root because the violation applies to its complete source tree.
    cx.emit_span_lint(
        LARGE_RUST_CRATE,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(
                "create new crates to split large crates into smaller crates by responsibility",
            );
        }),
    );
}

#[test]
fn ui() {
    let mut test = dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui");
    test.dylint_toml("[large-rust-crate]\nnon_test_line_limit = 5\ntotal_line_limit = 7\n")
        .run();
}

#[cfg(test)]
mod tests {
    use super::{
        Config, CrateSizeViolation, CrateStats, DEFAULT_NON_TEST_LINE_LIMIT,
        DEFAULT_TOTAL_LINE_LIMIT, crate_local_path, default_non_test_line_limit,
        default_total_line_limit, file_stats_from_source, first_line_len, first_line_span,
        normalize_path, test_line_count,
    };
    use rustc_span::{BytePos, Span};
    use std::path::{Path, PathBuf};

    /// Attribute-free files and attribute-like text remain production source.
    #[test]
    fn keeps_non_attribute_source_in_production_count() {
        for source in [
            "const VALUE: usize = 1;\n",
            "// #[test]\nfn ordinary() {}\n",
            "const TEXT: &str = \"#[test]\";\n",
            "not valid Rust {\n",
        ] {
            assert_eq!(test_line_count(source, source.lines().count()), 0);
        }
    }

    /// The serde default helpers return the documented limits.
    #[test]
    fn serde_defaults_match_documented_limits() {
        assert_eq!(default_non_test_line_limit(), DEFAULT_NON_TEST_LINE_LIMIT);
        assert_eq!(default_total_line_limit(), DEFAULT_TOTAL_LINE_LIMIT);
    }

    /// Test-only items and their attribute lines are excluded from counts.
    #[test]
    fn excludes_test_only_associated_and_foreign_items() {
        let source = "\
struct Value;
impl Value {
    #[cfg(test)]
    fn helper(&self) {}
    fn kept(&self) {}
}
trait Contract {
    #[test]
    fn check();
    fn kept();
}
unsafe extern \"C\" {
    #[cfg(all(test, unix))]
    fn foreign();
    fn kept_foreign();
}
";
        assert_eq!(test_line_count(source, 16), 6);
    }

    /// Only `cfg` predicates that require `test` mark an item as test-only.
    #[test]
    fn evaluates_cfg_predicates() {
        // `any` requires `test` only when every non-empty branch does.
        let cases = [
            ("#[cfg(any(test, test))]\nfn a() {}\n", 2),
            ("#[cfg(any(test, unix))]\nfn a() {}\n", 0),
            ("#[cfg(any())]\nfn a() {}\n", 0),
            ("#[cfg(all(unix, test))]\nfn a() {}\n", 2),
            ("#[cfg(all(unix))]\nfn a() {}\n", 0),
            ("#[cfg(not(test))]\nfn a() {}\n", 0),
            ("#[cfg(feature = \"test\")]\nfn a() {}\n", 0),
            ("#[cfg]\nfn a() {}\n", 0),
            ("#[cfg(all(,,))]\nfn a() {}\n", 0),
            ("#[cfg(test, unix)]\nfn a() {}\n", 0),
        ];
        assert!(
            cases
                .iter()
                .all(|(source, expected)| test_line_count(source, 2) == *expected)
        );
    }

    /// Lexical parent and current-directory components are folded.
    #[test]
    fn normalizes_lexical_components() {
        assert_eq!(
            normalize_path(Path::new("/work/./crate/../crate/src")),
            PathBuf::from("/work/crate/src")
        );
    }

    /// The documented defaults are used when configuration is absent.
    #[test]
    fn uses_documented_defaults() {
        let config = Config::default();

        assert_eq!(config.non_test_line_limit, DEFAULT_NON_TEST_LINE_LIMIT);
        assert_eq!(config.total_line_limit, DEFAULT_TOTAL_LINE_LIMIT);
    }

    /// Limits are strict maxima, and total size takes precedence over non-test size.
    #[test]
    fn applies_limits_after_aggregation() {
        // Keep the exact boundary values so strict maxima remain covered.
        let config = Config {
            non_test_line_limit: 4,
            total_line_limit: 8,
        };
        let at_limits = CrateStats {
            total_line_count: 8,
            non_test_line_count: 4,
        };
        let over_non_test = CrateStats {
            total_line_count: 8,
            non_test_line_count: 5,
        };
        let over_total = CrateStats {
            total_line_count: 9,
            non_test_line_count: 5,
        };

        // Verify the exact boundary and each independent violation category.
        assert_eq!(at_limits.violation(&config), None);
        assert_eq!(
            over_non_test.violation(&config),
            Some(CrateSizeViolation::NonTestLines { count: 5, limit: 4 })
        );
        assert_eq!(
            over_total.violation(&config),
            Some(CrateSizeViolation::TotalLines { count: 9, limit: 8 })
        );
    }

    /// Test-only item ranges are excluded from the non-test count.
    #[test]
    fn classifies_test_only_source() {
        // Count total and non-test lines separately around a test module.
        let source = "fn main() {}\n#[cfg(test)]\nmod tests {\n    // test\n}\n";
        let stats = file_stats_from_source(source);

        assert_eq!(stats.total_line_count, 5);
        assert_eq!(stats.non_test_line_count, 1);
        assert_eq!(test_line_count(source, 5), 4);
    }

    /// A parse failure conservatively leaves every line in the non-test count.
    #[test]
    fn treats_parse_failure_as_non_test() {
        let stats = file_stats_from_source("#[cfg(test)]\nmod tests {");

        assert_eq!(stats.total_line_count, 2);
        assert_eq!(stats.non_test_line_count, 2);
    }

    /// Source files from sibling packages are excluded from the crate aggregate.
    #[test]
    fn keeps_source_within_package_root() {
        let crate_root = Path::new("/work/target-crate");

        assert_eq!(
            crate_local_path(PathBuf::from("src/lib.rs"), crate_root),
            Some(crate_root.join("src/lib.rs"))
        );
        assert!(crate_local_path(PathBuf::from("../other-crate/src/lib.rs"), crate_root).is_none());
    }

    /// The first source line always yields a positive diagnostic span length.
    #[test]
    fn measures_first_source_line() {
        assert_eq!(first_line_len(""), 1);
    }

    /// The first source line length counts bytes before the first newline.
    #[test]
    fn measures_nonempty_source_line() {
        assert_eq!(first_line_len("é\nrest"), 2);
    }

    /// First-line spans preserve their start and saturate their end position.
    #[test]
    fn builds_first_source_span() {
        let span = first_line_span(BytePos(u32::MAX), 10);

        assert_eq!(
            span,
            Span::new(
                BytePos(u32::MAX),
                BytePos(u32::MAX),
                rustc_span::SyntaxContext::root(),
                None
            )
        );
    }

    /// Every parsed item family reaches its attribute accessor.
    #[test]
    fn visits_remaining_item_families() {
        let source = "\
const VALUE: usize = 0;\
enum State { Ready }\
extern crate self as alias;\
fn function() {}\
macro_rules! generated { () => {} }\
static STATIC: usize = 0;\
struct Struct;\
trait Contract {}\
type Alias = usize;\
union Union { value: u8 }\
use std::fmt;\
impl Struct { const VALUE: usize = 0; type Alias = usize; fn method(&self) {} }\
trait Extended { const VALUE: usize; fn method(&self); type Alias; }\
unsafe extern \"C\" { static FOREIGN: usize; type Foreign; fn foreign(); }\
";

        assert_eq!(test_line_count(source, source.lines().count()), 0);
    }
}
