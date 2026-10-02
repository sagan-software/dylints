#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![doc(hidden)]

//! Shared semantic helpers for test-case-specific private lints.
//!
//! The helpers resolve generated test suites through rustc expansion metadata,
//! recover every `#[test_case]` and `#[test_matrix]` attribute that the macro
//! consumed, and parse each attribute with the grammar that `test-case-core`
//! accepts. Rules then inspect typed syntax instead of attribute text, and
//! diagnostics keep the original source spans.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_span;

mod grammar;

use self::grammar::{CaseArgs, ExpectedResult, Leaf};
use dylint_linting as _;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LintContext as _};
use rustc_span::{
    BytePos, Pos as _, Span,
    hygiene::{ExpnKind, MacroKind},
    source_map::SourceMap,
};
use std::{ops::Range, str::FromStr as _};
use syn::{Attribute, Expr, ExprLit, Lit, Pat, parse::Parser as _, spanned::Spanned as _};

/// The number of generated tests at which a suite counts as large.
const LARGE_SUITE_THRESHOLD: usize = 64;

/// A public test-case procedural macro.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestCaseMacro {
    /// The singular `test_case` macro, including its `case` alias.
    TestCase,
    /// The Cartesian-product `test_matrix` macro.
    TestMatrix,
}

/// A documented generated-suite or attribute rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuiteViolation {
    /// A function's attributes generate at least 64 tests.
    LargeSuite,
    /// `ignore` has no reason.
    IgnoreWithoutReason,
    /// `ignore` or `inconclusive` has an empty reason.
    EmptyIgnoreReason,
    /// `panics` has no expected message.
    PanicsWithoutMessage,
    /// `panics` has an empty expected message.
    EmptyPanicMessage,
    /// A `test_case` attribute has no explicit description.
    UnnamedTestCase,
    /// A `test_case` attribute has a blank description.
    EmptyDescription,
    /// An attribute uses the legacy `inconclusive` modifier spelling.
    InconclusiveModifier,
    /// An async suite has no async test harness attribute.
    AsyncWithoutTestHarness,
    /// A multi-case matrix wraps one value in a singleton collection.
    SingletonMatrixDimension,
    /// A matrix applies a skip modifier to every generated case.
    IgnoredMatrix,
    /// An ignore reason contains only whitespace.
    WhitespaceIgnoreReason,
    /// A panic expectation contains only whitespace.
    WhitespacePanicMessage,
    /// A `matches` assertion uses only the wildcard pattern.
    WildcardMatch,
    /// A `matches` assertion uses a constant boolean guard.
    ConstantMatchGuard,
    /// An `almost` matcher uses a nonpositive literal precision.
    NonpositiveAlmostPrecision,
    /// `contains_in_order` receives a one-element literal sequence.
    SingletonContainsInOrder,
    /// `with` receives a path instead of a closure.
    WithFunctionPath,
    /// A description relies on the removed `inconclusive` naming convention.
    LegacyInconclusiveDescription,
    /// A matrix attribute generates no tests.
    EmptyMatrix,
    /// A matrix attribute generates exactly one test.
    SingleCaseMatrix,
}

/// One diagnostic location and its optional exact rewrite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// The primary diagnostic span.
    pub span: Span,
    /// A machine-applicable rewrite, present only when it preserves behavior.
    pub fix: Option<Fix>,
}

/// One exact source replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fix {
    /// The replaced source range.
    pub span: Span,
    /// The replacement text.
    pub replacement: String,
}

impl Finding {
    /// Build a help-only finding.
    const fn at(span: Span) -> Self {
        Self { span, fix: None }
    }
}

/// Match one generated test suite against a documented review rule.
///
/// The result holds one finding per offending attribute, or one finding at the
/// first attribute for suite-wide rules.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, item, violation| {
///     let _ = test_case_support::suite_violations(cx, item, violation);
/// };
/// ```
pub fn suite_violations<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
    violation: SuiteViolation,
) -> Vec<Finding> {
    let Some(suite) = generated_test_suite(cx, item) else {
        return Vec::new();
    };
    // Suite-wide rules depend on the generated functions, not on one attribute.
    if violation == SuiteViolation::LargeSuite {
        return suite_finding(&suite, suite.tests.len() >= LARGE_SUITE_THRESHOLD);
    }
    if violation == SuiteViolation::AsyncWithoutTestHarness {
        return suite_finding(&suite, has_async_test(&suite));
    }
    // Attribute rules inspect every case attribute that the macro consumed.
    let mut findings = Vec::new();
    for attribute in case_attributes(cx.sess().source_map(), &suite) {
        findings.extend(attribute.findings(violation));
    }
    findings
}

/// Report the first attribute when a suite-wide condition holds.
fn suite_finding(suite: &GeneratedTestSuite<'_>, is_violation: bool) -> Vec<Finding> {
    if is_violation {
        vec![Finding::at(suite.call_span)]
    } else {
        Vec::new()
    }
}

/// Detect a generated test that is still async after every attribute expanded.
fn has_async_test(suite: &GeneratedTestSuite<'_>) -> bool {
    suite.tests.iter().any(|test| {
        matches!(
            test.kind,
            ItemKind::Fn { sig, .. } if sig.header.asyncness.is_async()
        )
    })
}

/// One parsed test-case attribute with its source span.
#[derive(Debug)]
struct CaseAttribute {
    /// The whole attribute, from `#` to `]`.
    span: Span,
    /// The macro that the attribute invokes.
    kind: TestCaseMacro,
    /// The parsed attribute arguments.
    args: CaseArgs,
}

impl CaseAttribute {
    /// Evaluate one attribute rule.
    fn findings(&self, violation: SuiteViolation) -> Vec<Finding> {
        if matches!(
            violation,
            SuiteViolation::IgnoreWithoutReason
                | SuiteViolation::EmptyIgnoreReason
                | SuiteViolation::WhitespaceIgnoreReason
        ) {
            return self.modifier_rule_findings(violation);
        }
        if matches!(
            violation,
            SuiteViolation::InconclusiveModifier
                | SuiteViolation::IgnoredMatrix
                | SuiteViolation::PanicsWithoutMessage
                | SuiteViolation::EmptyPanicMessage
                | SuiteViolation::WhitespacePanicMessage
                | SuiteViolation::UnnamedTestCase
                | SuiteViolation::EmptyDescription
                | SuiteViolation::LegacyInconclusiveDescription
        ) {
            return self.basic_findings(violation);
        }
        self.assertion_findings(violation)
    }

    /// Evaluate rules for modifiers and simple result shapes.
    fn basic_findings(&self, violation: SuiteViolation) -> Vec<Finding> {
        if violation == SuiteViolation::InconclusiveModifier {
            return self.inconclusive_findings();
        }
        if violation == SuiteViolation::IgnoredMatrix {
            return self.whole_attribute(
                self.kind == TestCaseMacro::TestMatrix
                    && self
                        .args
                        .expectation
                        .as_ref()
                        .is_some_and(|expectation| !expectation.modifiers.is_empty()),
            );
        }
        if violation == SuiteViolation::PanicsWithoutMessage {
            return self.whole_attribute(matches!(
                self.result(),
                Some(ExpectedResult::Panicking(None))
            ));
        }
        if violation == SuiteViolation::EmptyPanicMessage {
            return self
                .whole_attribute(self.panic_message().is_some_and(|value| value.is_empty()));
        }
        if violation == SuiteViolation::WhitespacePanicMessage {
            return self.whole_attribute(
                self.panic_message()
                    .is_some_and(|value| is_nonempty_whitespace(&value)),
            );
        }
        if violation == SuiteViolation::UnnamedTestCase {
            return self.whole_attribute(
                self.kind == TestCaseMacro::TestCase && self.args.description.is_none(),
            );
        }
        if violation == SuiteViolation::EmptyDescription {
            return self.whole_attribute(
                self.kind == TestCaseMacro::TestCase
                    && self
                        .args
                        .description
                        .as_ref()
                        .is_some_and(|description| description.value().is_empty()),
            );
        }
        if violation == SuiteViolation::LegacyInconclusiveDescription {
            return self.whole_attribute(self.args.description.as_ref().is_some_and(
                |description| is_legacy_inconclusive_description(&description.value()),
            ));
        }
        Vec::new()
    }

    /// Evaluate modifier rules for the expectation clause.
    fn modifier_rule_findings(&self, violation: SuiteViolation) -> Vec<Finding> {
        if violation == SuiteViolation::IgnoreWithoutReason {
            return self.modifier_findings(|modifier| {
                modifier.keyword == "ignore" && modifier.reason.is_none()
            });
        }
        if violation == SuiteViolation::EmptyIgnoreReason {
            return self.modifier_findings(|modifier| {
                modifier
                    .reason
                    .as_ref()
                    .is_some_and(|reason| reason.value().is_empty())
            });
        }
        if violation == SuiteViolation::WhitespaceIgnoreReason {
            return self.modifier_findings(|modifier| {
                modifier
                    .reason
                    .as_ref()
                    .is_some_and(|reason| is_nonempty_whitespace(&reason.value()))
            });
        }
        Vec::new()
    }

    /// Evaluate one matrix-shape or assertion rule.
    fn assertion_findings(&self, violation: SuiteViolation) -> Vec<Finding> {
        if violation == SuiteViolation::SingletonMatrixDimension {
            return self.singleton_dimension_findings();
        }
        if violation == SuiteViolation::EmptyMatrix {
            return self.whole_attribute(self.matrix_case_count() == Some(0));
        }
        if violation == SuiteViolation::SingleCaseMatrix {
            return self.whole_attribute(self.matrix_case_count() == Some(1));
        }
        if violation == SuiteViolation::WildcardMatch {
            return self.whole_attribute(matches!(
                self.result(),
                Some(ExpectedResult::Matching { pattern, .. }) if is_wildcard(pattern)
            ));
        }
        if violation == SuiteViolation::ConstantMatchGuard {
            return self.constant_guard_findings();
        }
        if violation == SuiteViolation::NonpositiveAlmostPrecision {
            return self.whole_attribute(self.leaves().any(
                |leaf| matches!(leaf, Leaf::Almost { precision } if is_nonpositive_literal(precision)),
            ));
        }
        if violation == SuiteViolation::SingletonContainsInOrder {
            return self.whole_attribute(self.leaves().any(
                |leaf| matches!(leaf, Leaf::ContainsInOrder { expected } if singleton_element(expected).is_some()),
            ));
        }
        if violation == SuiteViolation::WithFunctionPath {
            return self.with_path_findings();
        }
        Vec::new()
    }

    /// Report the whole attribute when a condition holds.
    fn whole_attribute(&self, is_violation: bool) -> Vec<Finding> {
        if is_violation {
            vec![Finding::at(self.span)]
        } else {
            Vec::new()
        }
    }

    /// Report the whole attribute once when any modifier matches.
    fn modifier_findings(&self, predicate: impl Fn(&grammar::Modifier) -> bool) -> Vec<Finding> {
        self.whole_attribute(
            self.args
                .expectation
                .as_ref()
                .is_some_and(|expectation| expectation.modifiers.iter().any(predicate)),
        )
    }

    /// Report each `inconclusive` keyword with its exact `ignore` rewrite.
    fn inconclusive_findings(&self) -> Vec<Finding> {
        let Some(expectation) = &self.args.expectation else {
            return Vec::new();
        };
        expectation
            .modifiers
            .iter()
            .filter(|modifier| modifier.keyword == "inconclusive")
            .map(|modifier| {
                let keyword = self.subspan(modifier.keyword.span().byte_range());
                Finding {
                    span: keyword,
                    fix: Some(Fix {
                        span: keyword,
                        replacement: "ignore".to_owned(),
                    }),
                }
            })
            .collect()
    }

    /// Report singleton matrix dimensions and unwrap the ones that stay one value.
    fn singleton_dimension_findings(&self) -> Vec<Finding> {
        // A one-case matrix belongs to `test-case-single-case-matrix` instead.
        if self.kind != TestCaseMacro::TestMatrix
            || self.matrix_case_count().is_none_or(|count| count <= 1)
        {
            return Vec::new();
        }
        self.args
            .inputs
            .iter()
            .filter_map(|input| {
                let element = singleton_element(input)?;
                let span = self.subspan(input.span().byte_range());
                // Unwrapping a collection or range would turn it into a new dimension.
                let fix = (!matches!(element, Expr::Array(_) | Expr::Tuple(_) | Expr::Range(_)))
                    .then(|| Fix {
                        span,
                        replacement: self.source_of(element.span().byte_range()),
                    });
                Some(Finding { span, fix })
            })
            .collect()
    }

    /// Report constant guards and remove a guard that is always `true`.
    fn constant_guard_findings(&self) -> Vec<Finding> {
        let Some(ExpectedResult::Matching {
            pattern,
            guard: Some(guard),
        }) = self.result()
        else {
            return Vec::new();
        };
        let Expr::Lit(ExprLit {
            lit: Lit::Bool(value),
            ..
        }) = guard
        else {
            return Vec::new();
        };
        let guard_end = guard.span().byte_range().end;
        let span = self.subspan(pattern.span().byte_range().end..guard_end);
        // A `false` guard fails every case, so deleting it would change behavior.
        let fix = value.value.then(|| Fix {
            span,
            replacement: String::new(),
        });
        vec![Finding { span, fix }]
    }

    /// Report `with` followed by a path and rename the keyword to `using`.
    fn with_path_findings(&self) -> Vec<Finding> {
        let Some(ExpectedResult::With { keyword, expr }) = self.result() else {
            return Vec::new();
        };
        if !matches!(expr, Expr::Path(_)) {
            return Vec::new();
        }
        let keyword = self.subspan(keyword.span().byte_range());
        vec![Finding {
            span: keyword,
            fix: Some(Fix {
                span: keyword,
                replacement: "using".to_owned(),
            }),
        }]
    }

    /// Return the parsed assertion, if the attribute has one.
    fn result(&self) -> Option<&ExpectedResult> {
        self.args
            .expectation
            .as_ref()
            .map(|expectation| &expectation.result)
    }

    /// Return the decoded string literal passed to `panics`.
    fn panic_message(&self) -> Option<String> {
        let Some(ExpectedResult::Panicking(Some(Expr::Lit(ExprLit {
            lit: Lit::Str(message),
            ..
        })))) = self.result()
        else {
            return None;
        };
        Some(message.value())
    }

    /// Iterate over the leaves of a complex `it`/`is` assertion.
    fn leaves(&self) -> impl Iterator<Item = &Leaf> {
        let leaves: &[Leaf] = match self.result() {
            Some(ExpectedResult::Complex(leaves)) => leaves,
            _ => &[],
        };
        leaves.iter()
    }

    /// Count the cases a matrix attribute generates, using test-case's rules.
    fn matrix_case_count(&self) -> Option<usize> {
        if self.kind != TestCaseMacro::TestMatrix {
            return None;
        }
        self.args.inputs.iter().try_fold(1_usize, |count, input| {
            count.checked_mul(dimension_len(input)?)
        })
    }

    /// Map a byte range of the attribute text to a source span.
    fn subspan(&self, range: Range<usize>) -> Span {
        let lo = self.span.lo();
        self.span
            .with_lo(lo + BytePos::from_usize(range.start))
            .with_hi(lo + BytePos::from_usize(range.end))
    }

    /// Return the attribute text in a byte range.
    fn source_of(&self, range: Range<usize>) -> String {
        self.args.source.get(range).unwrap_or_default().to_owned()
    }
}

/// Return the only element of a one-element array or tuple.
fn singleton_element(input: &Expr) -> Option<&Expr> {
    if let Expr::Array(array) = input {
        return (array.elems.len() == 1)
            .then(|| array.elems.first())
            .flatten();
    }
    if let Expr::Tuple(tuple) = input {
        return (tuple.elems.len() == 1)
            .then(|| tuple.elems.first())
            .flatten();
    }
    None
}

/// Count the values of one matrix dimension the way `test-case-core` expands it.
fn dimension_len(input: &Expr) -> Option<usize> {
    if let Expr::Array(array) = input {
        return Some(array.elems.len());
    }
    if let Expr::Tuple(tuple) = input {
        return Some(tuple.elems.len());
    }
    let Expr::Range(range) = input else {
        return Some(1);
    };
    // test-case accepts only integer literal bounds; others fail to expand.
    let start = integer_literal(range.start.as_deref()?)?;
    let end = integer_literal(range.end.as_deref()?)?;
    let end = match range.limits {
        syn::RangeLimits::HalfOpen(_) => end,
        syn::RangeLimits::Closed(_) => end.checked_add(1)?,
    };
    usize::try_from(end.saturating_sub(start).max(0)).ok()
}

/// Decode an unsigned integer literal.
fn integer_literal(expr: &Expr) -> Option<i128> {
    let Expr::Lit(ExprLit {
        lit: Lit::Int(literal),
        ..
    }) = expr
    else {
        return None;
    };
    literal.base10_parse().ok()
}

/// Check a `_` pattern, optionally parenthesized.
fn is_wildcard(pattern: &Pat) -> bool {
    if matches!(pattern, Pat::Wild(_)) {
        return true;
    }
    if let Pat::Paren(inner) = pattern {
        return is_wildcard(&inner.pat);
    }
    false
}

/// Check a zero or negative numeric literal.
fn is_nonpositive_literal(expr: &Expr) -> bool {
    if let Expr::Lit(ExprLit { lit, .. }) = expr {
        return numeric_literal(lit) == Some(0.0);
    }
    if let Expr::Unary(unary) = expr
        && matches!(unary.op, syn::UnOp::Neg(_))
    {
        return matches!(&*unary.expr, Expr::Lit(ExprLit { lit, .. }) if numeric_literal(lit).is_some());
    }
    false
}

/// Decode an integer or floating-point literal.
fn numeric_literal(literal: &Lit) -> Option<f64> {
    if let Lit::Int(value) = literal {
        return value.base10_parse().ok();
    }
    if let Lit::Float(value) = literal {
        return value.base10_parse().ok();
    }
    None
}

/// Check for meaningful-looking text that consists entirely of whitespace.
fn is_nonempty_whitespace(value: &str) -> bool {
    !value.is_empty() && value.trim().is_empty()
}

/// Recognize the pre-2.0 description convention that used to ignore a case.
fn is_legacy_inconclusive_description(value: &str) -> bool {
    let trimmed = value.trim_start();
    trimmed == "inconclusive"
        || trimmed
            .strip_prefix("inconclusive")
            .and_then(|suffix| suffix.chars().next())
            .is_some_and(|separator| matches!(separator, ' ' | '-' | ':'))
}

/// Recover and parse every case attribute that one macro invocation consumed.
///
/// The first attribute invoked the macro and may be renamed by an import. The
/// macro then removes later attributes that use one of its fixed path spellings,
/// and they sit between the first attribute and the function name.
fn case_attributes(source_map: &SourceMap, suite: &GeneratedTestSuite<'_>) -> Vec<CaseAttribute> {
    let mut attributes = Vec::new();
    attributes.extend(parse_attribute(
        source_map,
        suite.call_span,
        Some(suite.macro_kind),
    ));

    // Scan only the region between the first attribute and the function name.
    let (call_span, name_span) = (suite.call_span, suite.name_span);
    if !call_span.eq_ctxt(name_span) || call_span.hi() > name_span.lo() {
        return attributes;
    }
    let region = call_span.with_lo(call_span.hi()).with_hi(name_span.lo());
    let Ok(text) = source_map.span_to_snippet(region) else {
        return attributes;
    };
    let Ok(stream) = TokenStream::from_str(&text) else {
        return attributes;
    };
    for range in outer_attribute_ranges(stream) {
        let lo = region.lo();
        let span = region
            .with_lo(lo + BytePos::from_usize(range.start))
            .with_hi(lo + BytePos::from_usize(range.end));
        attributes.extend(parse_attribute(source_map, span, None));
    }
    attributes
}

/// Return the byte range of each `#[...]` attribute in a token stream.
fn outer_attribute_ranges(stream: TokenStream) -> Vec<Range<usize>> {
    let tokens = stream.into_iter().collect::<Vec<_>>();
    let mut ranges = Vec::new();
    for window in tokens.windows(2) {
        if let [TokenTree::Punct(pound), TokenTree::Group(group)] = window
            && pound.as_char() == '#'
            && group.delimiter() == Delimiter::Bracket
        {
            ranges.push(pound.span().byte_range().start..group.span().byte_range().end);
        }
    }
    ranges
}

/// Parse one attribute span as a test-case attribute.
///
/// `resolved` carries the macro kind proven by expansion metadata for the first
/// attribute; later attributes must use a path spelling that test-case accepts.
fn parse_attribute(
    source_map: &SourceMap,
    span: Span,
    resolved: Option<TestCaseMacro>,
) -> Option<CaseAttribute> {
    let text = source_map.span_to_snippet(span).ok()?;
    let attributes = Attribute::parse_outer.parse_str(&text).ok()?;
    let [attribute] = attributes.as_slice() else {
        return None;
    };
    let kind = resolved.or_else(|| consumed_macro_kind(attribute.path()))?;
    let mut args = attribute.parse_args_with(CaseArgs::parse).ok()?;
    args.source = text;
    Some(CaseAttribute { span, kind, args })
}

/// Return the macro for a later attribute path that test-case consumes.
fn consumed_macro_kind(path: &syn::Path) -> Option<TestCaseMacro> {
    if path.leading_colon.is_some() {
        return None;
    }
    let segments = path
        .segments
        .iter()
        .map(|segment| {
            segment
                .arguments
                .is_none()
                .then(|| segment.ident.to_string())
        })
        .collect::<Option<Vec<_>>>()?;
    match segments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["test_case" | "case"] | ["test_case", "test_case" | "case"] => {
            Some(TestCaseMacro::TestCase)
        }
        ["test_matrix"] | ["test_case", "test_matrix"] => Some(TestCaseMacro::TestMatrix),
        _ => None,
    }
}

/// Declare one generated test-case suite lint.
#[macro_export]
macro_rules! declare_suite_lint {
    (
        $lint:ident, $pass:ident, $violation:ident,
        $description:literal, $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }
        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check one semantically resolved generated suite.
            fn check_item(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                item: &'tcx rustc_hir::Item<'tcx>,
            ) {
                for finding in
                    $crate::suite_violations(cx, item, $crate::SuiteViolation::$violation)
                {
                    $crate::emit(cx, $lint, finding, $message, $help);
                }
            }
        }
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Emit one finding, with its exact rewrite when present.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, lint, finding| {
///     test_case_support::emit(cx, lint, finding, "message", "help");
/// };
/// ```
pub fn emit(
    cx: &LateContext<'_>,
    lint: &'static rustc_lint::Lint,
    finding: Finding,
    message: &'static str,
    help: &'static str,
) {
    cx.emit_span_lint(
        lint,
        finding.span,
        rustc_errors::DiagDecorator(move |diagnostic| {
            let diagnostic = diagnostic.primary_message(message);
            match finding.fix {
                Some(fix) => {
                    let _configured_suggestion = diagnostic.span_suggestion(
                        fix.span,
                        help,
                        fix.replacement,
                        rustc_errors::Applicability::MachineApplicable,
                    );
                }
                None => {
                    let _configured_help = diagnostic.help(help);
                }
            }
        }),
    );
}

/// One module generated by a resolved test-case procedural macro.
#[derive(Debug)]
struct GeneratedTestSuite<'hir> {
    /// The resolved test-case macro that generated the module.
    macro_kind: TestCaseMacro,
    /// The source attribute that invoked the procedural macro.
    call_span: Span,
    /// The name of the annotated function, which the module reuses.
    name_span: Span,
    /// Test functions generated inside the suite module, in HIR order.
    tests: Vec<&'hir Item<'hir>>,
}

/// Return a generated test-case suite for one expanded HIR module.
fn generated_test_suite<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
) -> Option<GeneratedTestSuite<'tcx>> {
    let ItemKind::Mod(name, module) = item.kind else {
        return None;
    };
    let (macro_kind, call_span) = test_case_macro_call(cx, item.span)?;

    // The macro emits one child function per generated case in a dedicated module.
    let tests = module
        .item_ids
        .iter()
        .map(|item_id| cx.tcx.hir_item(*item_id))
        .filter(|child| matches!(child.kind, ItemKind::Fn { .. }))
        .collect();

    Some(GeneratedTestSuite {
        macro_kind,
        call_span,
        name_span: name.span,
        tests,
    })
}

/// Resolve the test-case attribute macro responsible for an expanded span.
fn test_case_macro_call(cx: &LateContext<'_>, span: Span) -> Option<(TestCaseMacro, Span)> {
    let mut expansion = span.ctxt().outer_expn_data();

    // Resolve through wrapper expansions so aliases still prove their defining crate.
    loop {
        if matches!(expansion.kind, ExpnKind::Macro(MacroKind::Attr, _))
            && let Some(def_id) = expansion.macro_def_id
            && cx.tcx.crate_name(def_id.krate).as_str() == "test_case_macros"
        {
            let macro_kind = match cx.tcx.item_name(def_id).as_str() {
                "test_case" => TestCaseMacro::TestCase,
                _ => TestCaseMacro::TestMatrix,
            };
            return Some((macro_kind, expansion.call_site));
        }

        if !expansion.call_site.from_expansion() {
            return None;
        }
        // Continue outward until the public test-case attribute is found.
        expansion = expansion.call_site.ctxt().outer_expn_data();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_ranges_skip_other_tokens() {
        let text = "\n#[test_case(1)] /// doc\n#[ignore] pub async fn";
        let ranges = outer_attribute_ranges(TokenStream::from_str(text).unwrap());
        let found = ranges
            .into_iter()
            .map(|range| text.get(range).unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(
            found,
            ["#[test_case(1)]", concat!("/", "/", "/ doc"), "#[ignore]"]
        );
    }

    #[test]
    fn consumed_paths_match_test_case() {
        let kind = |source: &str| consumed_macro_kind(&syn::parse_str(source).unwrap());
        let actual = [
            kind("case"),
            kind("test_case::case"),
            kind("test_case::test_matrix"),
            kind("::test_case::test_case"),
            kind("test_case::<u8>"),
            kind("tokio::test"),
        ];
        let expected = [
            Some(TestCaseMacro::TestCase),
            Some(TestCaseMacro::TestCase),
            Some(TestCaseMacro::TestMatrix),
            None,
            None,
            None,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn dimension_lengths_follow_test_case() {
        let len = |source: &str| dimension_len(&syn::parse_str(source).unwrap());
        let actual = [
            len("[1, 2]"),
            len("(1,)"),
            len("0..3"),
            len("1..=3"),
            len("3..1"),
            len("0.."),
            len("x..3"),
            len("value"),
        ];
        assert_eq!(
            actual,
            [
                Some(2),
                Some(1),
                Some(3),
                Some(3),
                Some(0),
                None,
                None,
                Some(1)
            ]
        );
    }

    #[test]
    fn literal_helpers_decode_values() {
        let expr = |source: &str| syn::parse_str::<Expr>(source).unwrap();
        let actual = [
            is_nonpositive_literal(&expr("0.0")),
            is_nonpositive_literal(&expr("-1")),
            is_nonpositive_literal(&expr("-x")),
            is_nonpositive_literal(&expr("!1")),
            is_nonpositive_literal(&expr("\"0\"")),
            is_nonpositive_literal(&expr("x")),
        ];
        assert_eq!(actual, [true, true, false, false, false, false]);
    }

    #[test]
    fn grammar_mirrors_test_case() {
        let parse = |source: &str| {
            Attribute::parse_outer.parse_str(source).unwrap()[0]
                .parse_args_with(CaseArgs::parse)
                .is_ok()
        };
        let actual = [
            parse("#[a(1 => is not (eq 1 or lt 0))]"),
            parse("#[a(1 => it existing_path and file and dir and directory and empty)]"),
            parse("#[a(1 => is len 1 and count 2 and gt 3 and matches_regex \"x\")]"),
            parse("#[a(1 => is unknown 1)]"),
            parse("#[a(1 => is almost 1.0 0.1)]"),
            parse("#[a(1 ; \"x\" extra)]"),
            parse("#[a(1 => ignore)]"),
            parse("#[a(1 => 2)]"),
        ];
        assert_eq!(actual, [true, true, true, false, false, false, true, true]);
    }

    #[test]
    fn legacy_descriptions_need_a_separator() {
        assert!(is_legacy_inconclusive_description(" inconclusive"));
        assert!(is_legacy_inconclusive_description("inconclusive: slow"));
        assert!(!is_legacy_inconclusive_description("inconclusively"));
    }
}
