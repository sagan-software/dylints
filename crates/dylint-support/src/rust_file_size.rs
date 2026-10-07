#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "unrelated syntax variants contain no nested items"
)]

//! Shared Rust source-size policy for file-organization lints.

use std::ops::RangeInclusive;

use syn::{
    Attribute, ForeignItem, ImplItem, Item, Meta, Token, TraitItem,
    punctuated::Punctuated,
    spanned::Spanned as _,
    visit::{self, Visit},
};

/// Minimum non-test line count that violates the file-size policy.
const NON_TEST_LINE_LIMIT: usize = 1500;

/// Minimum total line count that violates the file-size policy.
const TOTAL_LINE_LIMIT: usize = 2000;

/// Reason one Rust source file exceeds the shared size limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustFileSizeViolation {
    /// The file exceeds the limit regardless of whether its lines are test code.
    TotalLines(usize),
    /// The file exceeds the lower limit for non-test code.
    NonTestLines(usize),
}

/// Source line ranges occupied by test-only items.
#[derive(Debug, Default)]
struct TestLineCollector {
    /// Inclusive, one-based line ranges for test-only items.
    ranges: Vec<RangeInclusive<usize>>,
}

/// Return the shared size-policy violation for one Rust source file.
///
/// Dylint file-organization lints call this helper with source text.
///
/// # Examples
///
/// ```rust
/// #![feature(rustc_private)]
/// let _check = |source: &str| dylint_support::rust_file_size_violation(source);
/// ```
#[must_use]
pub fn rust_file_size_violation(source: &str) -> Option<RustFileSizeViolation> {
    let total_line_count = source.lines().count();
    // Total violations and files below both limits do not require test-region parsing.
    if total_line_count >= TOTAL_LINE_LIMIT {
        return Some(RustFileSizeViolation::TotalLines(total_line_count));
    }
    if total_line_count < NON_TEST_LINE_LIMIT {
        return None;
    }

    // Only the interval between the two limits can depend on test-only line ranges.
    let test_line_count = test_line_count(source, total_line_count);
    let non_test_line_count = total_line_count.saturating_sub(test_line_count);
    (non_test_line_count >= NON_TEST_LINE_LIMIT)
        .then_some(RustFileSizeViolation::NonTestLines(non_test_line_count))
}

/// Count unique physical lines spanned by test-only Rust items.
fn test_line_count(source: &str, total_line_count: usize) -> usize {
    // A parse failure keeps every line in the non-test count.
    let Ok(file) = syn::parse_file(source) else {
        return 0;
    };
    let mut collector = TestLineCollector::default();
    collector.visit_file(&file);
    let mut test_lines = vec![false; total_line_count.saturating_add(1)];

    // Mark line membership because nested or adjacent test attributes can overlap.
    for range in collector.ranges {
        let start = (*range.start()).max(1);
        let end = (*range.end()).min(total_line_count);
        if let Some(lines) = test_lines.get_mut(start..=end) {
            lines.fill(true);
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
        // Stop traversal at test-only items so nested lines are counted once.
        let attrs = item_attributes(item);
        if is_test_only(attrs) {
            self.record_test_item(attrs, item.span());
        } else {
            visit::visit_item(self, item);
        }
    }

    /// Visit one item inside an implementation block.
    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        // Apply the same item-level policy inside implementations.
        let attrs = impl_item_attributes(item);
        if is_test_only(attrs) {
            self.record_test_item(attrs, item.span());
        } else {
            visit::visit_impl_item(self, item);
        }
    }

    /// Visit one item inside a trait definition.
    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        // Apply the same item-level policy inside traits.
        let attrs = trait_item_attributes(item);
        if is_test_only(attrs) {
            self.record_test_item(attrs, item.span());
        } else {
            visit::visit_trait_item(self, item);
        }
    }

    /// Visit one item inside an external block.
    fn visit_foreign_item(&mut self, item: &'ast ForeignItem) {
        // Apply the same item-level policy inside external blocks.
        let attrs = foreign_item_attributes(item);
        if is_test_only(attrs) {
            self.record_test_item(attrs, item.span());
        } else {
            visit::visit_foreign_item(self, item);
        }
    }
}

/// Return whether any attribute makes an item test-only.
fn is_test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(test_only_attribute)
}

/// Return the attributes attached to a parsed Rust item.
fn item_attributes(item: &Item) -> &[Attribute] {
    // Return attributes only for item variants that can contain nested source lines.
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
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
    // Ignore unsupported syntax variants because they cannot contain nested items.
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
    // Ignore unsupported syntax variants because they cannot contain nested items.
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
    // Ignore unsupported syntax variants because they cannot contain nested items.
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
    // Framework-qualified test attributes share the same final `test` path segment.
    let last_segment = attr.path().segments.last();
    let is_test_attribute = last_segment.is_some_and(|segment| segment.ident == "test");

    is_test_attribute || cfg_attribute_requires_test(attr)
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

    cfg_predicate_requires_test(&predicate)
}

/// Evaluate whether one parsed `cfg` predicate logically requires `test`.
fn cfg_predicate_requires_test(predicate: &Meta) -> bool {
    // An `all` needs one required test branch, while every `any` branch must require test.
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

#[cfg(test)]
mod tests {
    use super::{
        NON_TEST_LINE_LIMIT, RustFileSizeViolation, TOTAL_LINE_LIMIT, rust_file_size_violation,
        test_line_count,
    };

    /// Build a source file with one production item per requested line.
    fn non_test_source(line_count: usize) -> String {
        std::iter::repeat_n("const _: () = ();", line_count)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Build a source file whose lines after `main` belong to a test-only module.
    fn test_heavy_source(total_line_count: usize) -> String {
        let filler = std::iter::repeat_n("    // test", total_line_count.saturating_sub(4))
            .collect::<Vec<_>>()
            .join("\n");

        format!("fn main() {{}}\n#[cfg(test)]\nmod tests {{\n{filler}\n}}")
    }

    /// Both inclusive thresholds preserve their immediate valid boundary.
    #[test]
    fn enforces_inclusive_line_limits() {
        let below_non_test = non_test_source(NON_TEST_LINE_LIMIT - 1);
        let at_non_test = non_test_source(NON_TEST_LINE_LIMIT);
        let below_total = test_heavy_source(TOTAL_LINE_LIMIT - 1);
        let at_total = test_heavy_source(TOTAL_LINE_LIMIT);

        // Assert the valid predecessor and inclusive violation at both independent limits.
        let violations = [below_non_test, at_non_test, below_total, at_total]
            .map(|source| rust_file_size_violation(&source));
        assert_eq!(
            violations,
            [
                None,
                Some(RustFileSizeViolation::NonTestLines(NON_TEST_LINE_LIMIT)),
                None,
                Some(RustFileSizeViolation::TotalLines(TOTAL_LINE_LIMIT)),
            ]
        );
    }

    /// The optimized policy must match the original calculation at every threshold.
    #[test_case::test_case(0; "empty")]
    #[test_case::test_case(1; "single line")]
    #[test_case::test_case(1499; "below production limit")]
    #[test_case::test_case(1500; "production limit")]
    #[test_case::test_case(1501; "above production limit")]
    #[test_case::test_case(1999; "below total limit")]
    #[test_case::test_case(2000; "total limit")]
    #[test_case::test_case(2001; "above total limit")]
    fn preserves_policy_across_fast_path_boundaries(lines: usize) {
        // Production, test-only, and malformed sources must retain the original result.
        assert_original_policy(&non_test_source(lines));
        assert_original_policy(&test_heavy_source(lines));
        assert_original_policy(&"{\n".repeat(lines));
    }

    /// Compare one source with the original unconditional parsing calculation.
    fn assert_original_policy(source: &str) {
        let total = source.lines().count();
        let production = total.saturating_sub(test_line_count(source, total));
        // Preserve total-line precedence and parse-failure handling.
        let expected = if total >= TOTAL_LINE_LIMIT {
            Some(RustFileSizeViolation::TotalLines(total))
        } else if production >= NON_TEST_LINE_LIMIT {
            Some(RustFileSizeViolation::NonTestLines(production))
        } else {
            None
        };
        assert_eq!(rust_file_size_violation(source), expected);
    }

    /// `cfg` conjunctions and disjunctions follow whether `test` is required.
    #[test]
    fn classifies_test_only_cfg_predicates() {
        let test_only = "#[cfg(all(unix, test))]\nmod tests {\n    // test\n}";
        let mixed = "#[cfg(any(test, unix))]\nmod platform {\n    // code\n}";
        let framework_test = "#[tokio::test]\nasync fn works() {}";

        // Distinguish required test predicates from optional test branches and framework tests.
        assert_eq!(test_line_count(test_only, 4), 4);
        assert_eq!(test_line_count(mixed, 4), 0);
        assert_eq!(test_line_count(framework_test, 2), 2);
    }

    /// A parse failure keeps every physical line in the non-test count.
    #[test]
    fn malformed_source_is_conservative() {
        let source = "#[cfg(test)]\nmod tests {";

        assert_eq!(rust_file_size_violation(source), None);
        assert_eq!(test_line_count(source, 2), 0);
    }

    /// Source that exercises every item kind and nesting site in the collector.
    const TEST_ITEMS_SOURCE: &str = r#"const C: u8 = 0;
enum E { A }
extern crate alloc;
fn f() {}
extern "C" {
    fn ext();
    static S: u8;
    type Opaque;
    m!();
    fn with_body() {}
	#[cfg(test)] fn test_ext(); // test-only
}
impl E {
    const C: u8 = 0;
    fn g() {}
    type T = u8;
    m!();
    fn no_body();
    #[test] fn t() {} // test-only
}
m!();
mod inner {
    #[cfg(test)] fn helper() {} // test-only
}
static ST: u8 = 0;
struct St;
trait Tr {
    const C: u8;
    fn h();
    type T;
    m!();
    #[cfg(test)] fn only_test() {} // test-only
}
trait Alias = Tr;
type Ty = u8;
union U { a: u8 }
use std::fmt;
fn verbatim();
	#[cfg(any(test, all(test, unix)))] fn any_test() {} // test-only
#[cfg(not(test))] fn not_test() {}
#[cfg(feature = "x")] fn feature() {}
#[cfg(all())] fn empty_all() {}
#[cfg(any())] fn empty_any() {}
#[cfg(all(,))] fn malformed_children() {}
#[cfg(1)] fn malformed_predicate() {}
	#[cfg] fn bare_cfg() {}"#;
    /// Marker in comments on items compiled only for tests.
    const TEST_ONLY_MARKER: &str = "test-only";

    /// Every item kind and nesting site is classified, and only marker lines count.
    #[test]
    fn counts_test_items_in_every_nesting_site() {
        let expected = TEST_ITEMS_SOURCE.matches(TEST_ONLY_MARKER).count();

        assert_eq!(
            test_line_count(TEST_ITEMS_SOURCE, TEST_ITEMS_SOURCE.lines().count()),
            expected
        );
    }
}
