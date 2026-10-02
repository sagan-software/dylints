#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![doc(hidden)]

//! Shared semantic helpers for test-case-specific private lints.
//!
//! The helpers resolve generated test suites through rustc expansion metadata,
//! parse only the attribute tokens needed by each rule, and preserve original
//! spans for diagnostics. This keeps test-case checks semantic, deterministic,
//! and independent of generated function names.

extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_span;

use dylint_linting as _;
use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LintContext as _};
use rustc_span::{
    Span,
    hygiene::{ExpnKind, MacroKind},
};
use std::str::FromStr as _;
use syn::LitStr;

/// A public test-case procedural macro.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestCaseMacro {
    /// The singular `test_case` macro, including its `case` alias.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    TestCase,
    /// The Cartesian-product `test_matrix` macro.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    TestMatrix,
}

/// A documented generated-suite or attribute-source smell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuiteViolation {
    /// A macro expands to at least 64 tests.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    LargeSuite,
    /// `ignore` has no reason.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    IgnoreWithoutReason,
    /// `ignore` has an empty reason.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    EmptyIgnoreReason,
    /// `panics` has no expected message.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    PanicsWithoutMessage,
    /// `panics` has an empty expected message.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    EmptyPanicMessage,
    /// A `test_case` attribute has no explicit display name.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    UnnamedTestCase,
    /// A `test_case` attribute has an empty display name.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    EmptyDescription,
    /// A `test_case` attribute uses the legacy `inconclusive` spelling.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    InconclusiveModifier,
    /// An async suite has no async test harness attribute.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    AsyncWithoutTestHarness,
    /// A multi-case matrix wraps a constant in a singleton collection.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    SingletonMatrixDimension,
    /// A matrix applies `ignore` to every generated case.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    IgnoredMatrix,
    /// An ignore reason contains only whitespace.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    WhitespaceIgnoreReason,
    /// A panic expectation contains only whitespace.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    WhitespacePanicMessage,
    /// A `matches` assertion uses only the wildcard pattern.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    WildcardMatch,
    /// A `matches` assertion uses a constant boolean guard.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    ConstantMatchGuard,
    /// An `almost` matcher uses a nonpositive literal precision.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    NonpositiveAlmostPrecision,
    /// `contains_in_order` receives a one-element literal sequence.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    SingletonContainsInOrder,
    /// `with` receives a function path.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    WithFunctionPath,
    /// A description relies on the removed `inconclusive` naming convention.
    /// The rule reports this source condition before generated tests execute,
    /// preserving the original attribute span for remediation.
    LegacyInconclusiveDescription,
}

/// Match one generated test suite against a documented review rule.
///
/// The returned span is the original test-case attribute when the selected rule
/// matches, allowing the caller to attach one focused diagnostic to generated code.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, item, violation| {
///     let _ = test_case_support::suite_violation(cx, item, violation);
/// };
/// ```
pub fn suite_violation<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
    violation: SuiteViolation,
) -> Option<Span> {
    // Resolve the generated suite before reading its original macro invocation.
    let suite = generated_test_suite(cx, item)?;
    let snippet = cx
        .sess()
        .source_map()
        .span_to_snippet(suite.call_span)
        .unwrap_or_default();
    // Parse tokens once for rules that need structured matcher details.
    let invocation = TestCaseInvocation::parse(&snippet);
    // Evaluate only the selected rule so overlapping policies emit one diagnostic.
    let is_invalid = is_simple_violation(&suite, &snippet, violation)
        || is_async_violation(&suite, violation)
        || is_matrix_violation(&suite, invocation.as_ref(), violation)
        || is_text_violation(invocation.as_ref(), violation)
        || is_matcher_violation(invocation.as_ref(), violation);
    is_invalid.then_some(suite.call_span)
}

/// Evaluate source and suite rules that do not require parsed token groups.
fn is_simple_violation(
    suite: &GeneratedTestSuite<'_>,
    snippet: &str,
    violation: SuiteViolation,
) -> bool {
    match violation {
        SuiteViolation::LargeSuite => suite.tests.len() >= 64,
        SuiteViolation::IgnoreWithoutReason => {
            let expression = snippet.split_once(';').map_or(snippet, |part| part.0);
            expression.contains("ignore") && !expression.contains("ignore[")
        }
        SuiteViolation::EmptyIgnoreReason => compact_source(snippet).contains("ignore[\"\"]"),
        SuiteViolation::PanicsWithoutMessage => {
            let expression = snippet.split_once(';').map_or(snippet, |part| part.0);
            expression
                .split_once("panics")
                .is_some_and(|(_, message)| !message.contains('"'))
        }
        SuiteViolation::EmptyPanicMessage => compact_source(snippet).contains("panics\"\""),
        SuiteViolation::UnnamedTestCase => {
            suite.macro_kind == TestCaseMacro::TestCase && !snippet.contains(';')
        }
        SuiteViolation::EmptyDescription => {
            suite.macro_kind == TestCaseMacro::TestCase && compact_source(snippet).contains(";\"\"")
        }
        SuiteViolation::InconclusiveModifier => {
            let expression = snippet.split_once(';').map_or(snippet, |part| part.0);
            expression.contains("inconclusive")
        }
        SuiteViolation::AsyncWithoutTestHarness
        | SuiteViolation::SingletonMatrixDimension
        | SuiteViolation::IgnoredMatrix
        | SuiteViolation::WhitespaceIgnoreReason
        | SuiteViolation::WhitespacePanicMessage
        | SuiteViolation::WildcardMatch
        | SuiteViolation::ConstantMatchGuard
        | SuiteViolation::NonpositiveAlmostPrecision
        | SuiteViolation::SingletonContainsInOrder
        | SuiteViolation::WithFunctionPath
        | SuiteViolation::LegacyInconclusiveDescription => false,
    }
}

/// Detect an asynchronous generated test without its required harness.
fn is_async_violation(suite: &GeneratedTestSuite<'_>, violation: SuiteViolation) -> bool {
    if violation != SuiteViolation::AsyncWithoutTestHarness {
        return false;
    }
    suite.tests.iter().any(|test| {
        matches!(
            test.kind,
            ItemKind::Fn { sig, .. } if sig.header.asyncness.is_async()
        )
    })
}

/// Evaluate matrix-specific rules against the parsed invocation.
fn is_matrix_violation(
    suite: &GeneratedTestSuite<'_>,
    invocation: Option<&TestCaseInvocation>,
    violation: SuiteViolation,
) -> bool {
    match violation {
        SuiteViolation::SingletonMatrixDimension => {
            suite.macro_kind == TestCaseMacro::TestMatrix
                && suite.tests.len() > 1
                && invocation.is_some_and(TestCaseInvocation::has_singleton_matrix_dimension)
        }
        SuiteViolation::IgnoredMatrix => {
            suite.macro_kind == TestCaseMacro::TestMatrix
                && invocation.is_some_and(TestCaseInvocation::has_ignore_modifier)
        }
        SuiteViolation::LargeSuite
        | SuiteViolation::IgnoreWithoutReason
        | SuiteViolation::EmptyIgnoreReason
        | SuiteViolation::PanicsWithoutMessage
        | SuiteViolation::EmptyPanicMessage
        | SuiteViolation::UnnamedTestCase
        | SuiteViolation::EmptyDescription
        | SuiteViolation::InconclusiveModifier
        | SuiteViolation::AsyncWithoutTestHarness
        | SuiteViolation::WhitespaceIgnoreReason
        | SuiteViolation::WhitespacePanicMessage
        | SuiteViolation::WildcardMatch
        | SuiteViolation::ConstantMatchGuard
        | SuiteViolation::NonpositiveAlmostPrecision
        | SuiteViolation::SingletonContainsInOrder
        | SuiteViolation::WithFunctionPath
        | SuiteViolation::LegacyInconclusiveDescription => false,
    }
}

/// Evaluate whitespace and legacy-description rules.
fn is_text_violation(invocation: Option<&TestCaseInvocation>, violation: SuiteViolation) -> bool {
    match violation {
        SuiteViolation::WhitespaceIgnoreReason => invocation
            .and_then(TestCaseInvocation::ignore_reason)
            .is_some_and(|value| is_nonempty_whitespace(&value)),
        SuiteViolation::WhitespacePanicMessage => invocation
            .and_then(TestCaseInvocation::panic_message)
            .is_some_and(|value| is_nonempty_whitespace(&value)),
        SuiteViolation::LegacyInconclusiveDescription => invocation
            .and_then(TestCaseInvocation::description)
            .is_some_and(|value| is_legacy_inconclusive_description(&value)),
        SuiteViolation::LargeSuite
        | SuiteViolation::IgnoreWithoutReason
        | SuiteViolation::EmptyIgnoreReason
        | SuiteViolation::PanicsWithoutMessage
        | SuiteViolation::EmptyPanicMessage
        | SuiteViolation::UnnamedTestCase
        | SuiteViolation::EmptyDescription
        | SuiteViolation::InconclusiveModifier
        | SuiteViolation::AsyncWithoutTestHarness
        | SuiteViolation::SingletonMatrixDimension
        | SuiteViolation::IgnoredMatrix
        | SuiteViolation::WildcardMatch
        | SuiteViolation::ConstantMatchGuard
        | SuiteViolation::NonpositiveAlmostPrecision
        | SuiteViolation::SingletonContainsInOrder
        | SuiteViolation::WithFunctionPath => false,
    }
}

/// Evaluate matcher and argument-shape rules.
fn is_matcher_violation(
    invocation: Option<&TestCaseInvocation>,
    violation: SuiteViolation,
) -> bool {
    match violation {
        SuiteViolation::WildcardMatch => {
            invocation.is_some_and(TestCaseInvocation::has_wildcard_match)
        }
        SuiteViolation::ConstantMatchGuard => {
            invocation.is_some_and(TestCaseInvocation::has_constant_match_guard)
        }
        SuiteViolation::NonpositiveAlmostPrecision => {
            invocation.is_some_and(TestCaseInvocation::has_nonpositive_almost_precision)
        }
        SuiteViolation::SingletonContainsInOrder => {
            invocation.is_some_and(|source| source.contains_in_order_len() == Some(1))
        }
        SuiteViolation::WithFunctionPath => {
            invocation.is_some_and(TestCaseInvocation::uses_function_path_with_with)
        }
        SuiteViolation::LargeSuite
        | SuiteViolation::IgnoreWithoutReason
        | SuiteViolation::EmptyIgnoreReason
        | SuiteViolation::PanicsWithoutMessage
        | SuiteViolation::EmptyPanicMessage
        | SuiteViolation::UnnamedTestCase
        | SuiteViolation::EmptyDescription
        | SuiteViolation::InconclusiveModifier
        | SuiteViolation::AsyncWithoutTestHarness
        | SuiteViolation::SingletonMatrixDimension
        | SuiteViolation::IgnoredMatrix
        | SuiteViolation::WhitespaceIgnoreReason
        | SuiteViolation::WhitespacePanicMessage
        | SuiteViolation::LegacyInconclusiveDescription => false,
    }
}

/// The parsed top-level pieces of one test-case attribute.
#[derive(Debug)]
struct TestCaseInvocation {
    /// Input expressions before the optional output matcher.
    inputs: Vec<Vec<TokenTree>>,
    /// Tokens after `=>` and before the optional description.
    output: Vec<TokenTree>,
    /// Tokens after the optional description semicolon.
    description: Vec<TokenTree>,
}

impl TestCaseInvocation {
    /// Parse the attribute source without reimplementing Rust expression syntax.
    fn parse(source: &str) -> Option<Self> {
        // Split the token stream at the description and matcher boundaries.
        let stream = TokenStream::from_str(source).ok()?;
        let arguments = attribute_arguments(stream)?;
        let (body, description) = split_once_punct(&arguments, ';');
        let (inputs, output) = split_once_fat_arrow(body);

        Some(Self {
            inputs: split_commas(inputs),
            output: output.unwrap_or_default().to_vec(),
            description: description.unwrap_or_default().to_vec(),
        })
    }

    /// Return the ordinary string literal used as the description.
    fn description(&self) -> Option<String> {
        string_literal(self.description.first()?)
    }

    /// Return the ordinary string literal attached to `ignore` or `inconclusive`.
    fn ignore_reason(&self) -> Option<String> {
        let index = keyword_index(&self.output, &["ignore", "inconclusive"])?;
        let TokenTree::Group(group) = self.output.get(index + 1)? else {
            return None;
        };
        (group.delimiter() == Delimiter::Bracket)
            .then(|| first_string_literal(group))
            .flatten()
    }

    /// Return the ordinary string literal following `panics`.
    fn panic_message(&self) -> Option<String> {
        let index = keyword_index(&self.output, &["panics"])?;
        string_literal(self.output.get(index + 1)?)
    }

    /// Check whether a matrix collection wraps exactly one constant value.
    fn has_singleton_matrix_dimension(&self) -> bool {
        self.inputs
            .iter()
            .any(|input| explicit_matrix_values(input).is_some_and(|values| values.len() == 1))
    }

    /// Check whether the output applies an ignore modifier to the whole matrix.
    fn has_ignore_modifier(&self) -> bool {
        keyword_index(&self.output, &["ignore", "inconclusive"]).is_some()
    }

    /// Check whether a `matches` matcher ignores the returned value.
    fn has_wildcard_match(&self) -> bool {
        // Locate the matcher keyword before isolating its pattern tokens.
        let Some(index) = keyword_index(&self.output, &["matches"]) else {
            return false;
        };
        let Some(pattern) = self.output.get(index + 1..) else {
            return false;
        };
        let pattern_end = pattern
            .iter()
            .position(|token| is_ident(token, "if"))
            .unwrap_or(pattern.len());
        // Exclude guard tokens so only the match pattern decides the result.
        pattern.get(..pattern_end).is_some_and(is_wildcard)
    }

    /// Check whether a match guard is exactly `true` or `false`.
    fn has_constant_match_guard(&self) -> bool {
        let Some(index) = keyword_index(&self.output, &["if"]) else {
            return false;
        };
        matches!(
            self.output.get(index + 1..),
            Some([token]) if is_ident(token, "true") || is_ident(token, "false")
        )
    }

    /// Check whether an `almost` matcher has a literal precision at most zero.
    fn has_nonpositive_almost_precision(&self) -> bool {
        // Locate the precision argument before checking its literal shape.
        let Some(index) = keyword_index(&self.output, &["precision"]) else {
            return false;
        };
        let Some(precision) = self.output.get(index + 1..) else {
            return false;
        };
        // Treat unsigned zero and explicitly negative literals as invalid.
        match precision {
            [TokenTree::Literal(literal)] => {
                numeric_literal(literal).is_some_and(|value| value == 0.0)
            }
            [TokenTree::Punct(sign), TokenTree::Literal(literal)] if sign.as_char() == '-' => {
                numeric_literal(literal).is_some_and(|value| value >= 0.0)
            }
            _ => false,
        }
    }

    /// Return the literal sequence length supplied to `contains_in_order`.
    fn contains_in_order_len(&self) -> Option<usize> {
        let index = keyword_index(&self.output, &["contains_in_order"])?;
        let TokenTree::Group(group) = self.output.get(index + 1)? else {
            return None;
        };
        matches!(
            group.delimiter(),
            Delimiter::Bracket | Delimiter::Parenthesis
        )
        .then(|| split_commas(&group.stream().into_iter().collect::<Vec<_>>()).len())
    }

    /// Check whether `with` receives only a plain function path.
    fn uses_function_path_with_with(&self) -> bool {
        // Accept only nonempty identifier paths separated by colons.
        let Some(index) = keyword_index(&self.output, &["with"]) else {
            return false;
        };
        let Some(expression) = self.output.get(index + 1..) else {
            return false;
        };
        !expression.is_empty()
            && expression.iter().all(|token| match token {
                TokenTree::Ident(_) => true,
                TokenTree::Punct(punct) => punct.as_char() == ':',
                TokenTree::Group(_) | TokenTree::Literal(_) => false,
            })
    }
}

/// Find the parenthesized arguments inside an attribute snippet.
fn attribute_arguments(stream: TokenStream) -> Option<Vec<TokenTree>> {
    // Prefer the nested attribute group emitted by a complete source snippet.
    let tokens = stream.into_iter().collect::<Vec<_>>();
    for token in &tokens {
        if let TokenTree::Group(group) = token {
            if group.delimiter() == Delimiter::Bracket
                && let Some(arguments) = first_parenthesized_group(group)
            {
                return Some(arguments);
            }
            if group.delimiter() == Delimiter::Parenthesis {
                return Some(group.stream().into_iter().collect());
            }
        }
    }
    // Fall back to the supplied tokens when the caller already removed wrappers.
    (!tokens.is_empty()).then_some(tokens)
}

/// Return the first parenthesized token group inside another group.
fn first_parenthesized_group(group: &Group) -> Option<Vec<TokenTree>> {
    group.stream().into_iter().find_map(|token| {
        let TokenTree::Group(arguments) = token else {
            return None;
        };
        (arguments.delimiter() == Delimiter::Parenthesis)
            .then(|| arguments.stream().into_iter().collect())
    })
}

/// Split tokens at the first top-level punctuation mark.
fn split_once_punct(tokens: &[TokenTree], separator: char) -> (&[TokenTree], Option<&[TokenTree]>) {
    let Some(index) = tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == separator))
    else {
        return (tokens, None);
    };
    match (tokens.get(..index), tokens.get(index + 1..)) {
        (Some(before), Some(after)) => (before, Some(after)),
        _ => (tokens, None),
    }
}

/// Split tokens at the first top-level `=>`.
fn split_once_fat_arrow(tokens: &[TokenTree]) -> (&[TokenTree], Option<&[TokenTree]>) {
    let Some(index) = tokens.windows(2).position(|window| {
        matches!(
            window,
            [TokenTree::Punct(left), TokenTree::Punct(right)]
                if left.as_char() == '=' && right.as_char() == '>'
        )
    }) else {
        return (tokens, None);
    };
    match (tokens.get(..index), tokens.get(index + 2..)) {
        (Some(before), Some(after)) => (before, Some(after)),
        _ => (tokens, None),
    }
}

/// Split a top-level comma-separated token list.
fn split_commas(tokens: &[TokenTree]) -> Vec<Vec<TokenTree>> {
    // Preserve top-level token order while creating one bucket per separator.
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut values = vec![Vec::new()];
    for token in tokens {
        if matches!(token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            values.push(Vec::new());
        } else if let Some(value) = values.last_mut() {
            value.push(token.clone());
        }
    }
    // Remove empty buckets created by leading, trailing, or repeated separators.
    values.retain(|value| !value.is_empty());
    values
}

/// Return explicit array or tuple values from one matrix input.
fn explicit_matrix_values(input: &[TokenTree]) -> Option<Vec<Vec<TokenTree>>> {
    // Accept only one grouped expression and distinguish tuple commas from grouping.
    let [TokenTree::Group(group)] = input else {
        return None;
    };
    match group.delimiter() {
        Delimiter::Bracket => Some(split_commas(
            &group.stream().into_iter().collect::<Vec<_>>(),
        )),
        Delimiter::Parenthesis => {
            let tokens = group.stream().into_iter().collect::<Vec<_>>();
            tokens
                .iter()
                .any(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ','))
                .then(|| split_commas(&tokens))
        }
        Delimiter::Brace | Delimiter::None => None,
    }
}

/// Find one of the listed top-level keywords.
fn keyword_index(tokens: &[TokenTree], keywords: &[&str]) -> Option<usize> {
    tokens.iter().position(|token| {
        let TokenTree::Ident(identifier) = token else {
            return false;
        };
        keywords.contains(&identifier.to_string().as_str())
    })
}

/// Check one identifier token.
fn is_ident(token: &TokenTree, expected: &str) -> bool {
    matches!(token, TokenTree::Ident(identifier) if identifier == expected)
}

/// Check an optionally parenthesized wildcard pattern.
fn is_wildcard(tokens: &[TokenTree]) -> bool {
    match tokens {
        [token] if is_ident(token, "_") => true,
        [TokenTree::Group(group)] if group.delimiter() == Delimiter::Parenthesis => {
            is_wildcard(&group.stream().into_iter().collect::<Vec<_>>())
        }
        _ => false,
    }
}

/// Decode an ordinary or raw Rust string literal.
fn string_literal(token: &TokenTree) -> Option<String> {
    let TokenTree::Literal(literal) = token else {
        return None;
    };
    syn::parse_str::<LitStr>(&literal.to_string())
        .ok()
        .map(|literal| literal.value())
}

/// Decode the first string literal inside a modifier bracket.
fn first_string_literal(group: &Group) -> Option<String> {
    string_literal(&group.stream().into_iter().next()?)
}

/// Parse a suffix-free integer or floating-point literal.
fn numeric_literal(literal: &proc_macro2::Literal) -> Option<f64> {
    let source = literal.to_string().replace('_', "");
    source.parse().ok()
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

/// Remove insignificant whitespace from one macro invocation.
fn compact_source(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
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
                let Some(span) =
                    $crate::suite_violation(cx, item, $crate::SuiteViolation::$violation)
                else {
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
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// One module generated by a resolved test-case procedural macro.
#[derive(Debug)]
pub struct GeneratedTestSuite<'hir> {
    /// The resolved test-case macro that generated the module.
    /// This value distinguishes one-case attributes from Cartesian-product matrices
    /// when a lint applies a rule only to one generation strategy.
    pub macro_kind: TestCaseMacro,
    /// The source attribute that invoked the procedural macro.
    /// Diagnostics use this span because generated test functions do not have stable
    /// source locations that callers can edit directly.
    pub call_span: Span,
    /// Test functions generated inside the suite module.
    /// The collection is borrowed from HIR and preserves the module's generated
    /// item order for suite-size and async-harness checks.
    pub tests: Vec<&'hir Item<'hir>>,
}

/// Return a generated test-case suite for one expanded HIR module.
///
/// The result is present only for modules created by a resolved test-case macro;
/// ordinary user modules and unrelated procedural expansions are rejected.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, item| {
///     let _ = test_case_support::generated_test_suite(cx, item);
/// };
/// ```
pub fn generated_test_suite<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
) -> Option<GeneratedTestSuite<'tcx>> {
    let ItemKind::Mod(_, module) = item.kind else {
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
                "test_matrix" => TestCaseMacro::TestMatrix,
                _ => return None,
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
