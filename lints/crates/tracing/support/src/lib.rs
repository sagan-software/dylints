#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for tracing-specific private lints.
//!
//! The helpers resolve tracing macro expansions, preserve structured field text,
//! and build source-aware replacements for diagnostics. They keep event analysis
//! tied to public tracing APIs while rejecting local lookalikes and malformed input.
//! The support surface is shared by several lints, so every result preserves the
//! source order, resolved owner, and exact span needed for a focused recommendation.

extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_span;

use rustc_hir::{ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, def::Res};
use rustc_lint::{LateContext, LintContext};
use rustc_span::{ExpnData, ExpnKind, MacroKind, Span, SyntaxContext, def_id::DefId};

use dylint_linting as _;

/// Names of the public tracing macros that construct events.
const EVENT_MACROS: &[&str] = &["debug", "error", "event", "info", "trace", "warn"];

/// Names of all public tracing macros that accept structured fields.
const FIELD_MACROS: &[&str] = &[
    "debug",
    "debug_span",
    "error",
    "error_span",
    "event",
    "info",
    "info_span",
    "span",
    "trace",
    "trace_span",
    "warn",
    "warn_span",
];

/// One source-level invocation of a tracing event or span macro.
///
/// The value combines resolved macro identity with borrowed-independent source
/// arguments so callers can inspect fields and construct precise replacements.
/// Its source text remains authoritative when a lint offers a machine-applicable
/// rewrite because expanded HIR does not preserve the user's delimiters directly.
#[derive(Clone, Debug)]
pub struct TracingMacroInvocation {
    /// Span of the complete macro invocation used for diagnostics and source fixes.
    /// The span points to the public call rather than generated implementation code.
    pub span: Span,
    /// Resolved name of the public tracing macro.
    macro_name: String,
    /// Top-level source arguments supplied to the macro.
    arguments: Vec<String>,
    /// Original source used to preserve delimiters and separators in rewrites.
    source: String,
}

impl TracingMacroInvocation {
    /// Return interpolated values that are not also recorded as structured fields.
    ///
    /// The iterator preserves format argument order and omits values already
    /// represented by shorthand or explicit structured fields.
    /// It yields owned normalized names so callers can compare values after the
    /// original macro invocation has been traversed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = tracing_support::TracingMacroInvocation::unstructured_message_values(value);
    /// };
    /// ```
    pub fn unstructured_message_values(&self) -> impl Iterator<Item = String> + '_ {
        let parsed_message = (|| {
            if !EVENT_MACROS.contains(&self.macro_name.as_str()) {
                return None;
            }
            let message_index = self.message_index()?;
            let message = self.arguments.get(message_index)?;
            let format_string = rust_string_contents(message)?;
            let format_arguments = self.arguments.get(message_index + 1..).unwrap_or_default();
            Some((
                format_values(&format_string, format_arguments),
                self.structured_values(message_index),
            ))
        })();

        // Compare format inputs lazily with both shorthand fields and assigned field values.
        parsed_message.into_iter().flat_map(|(values, structured)| {
            values
                .into_iter()
                .filter(move |value| !structured.iter().any(|field| field == value))
        })
    }

    /// Return fields written as a redundant assignment such as `request_id = request_id`.
    ///
    /// Each returned name is normalized from one source argument and remains in
    /// the order used by the original tracing invocation.
    /// Configuration arguments and level markers are excluded from this field list,
    /// because they do not represent values recorded in the event payload.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = tracing_support::TracingMacroInvocation::redundant_field_assignments(value);
    /// };
    /// ```
    pub fn redundant_field_assignments(&self) -> impl Iterator<Item = String> + '_ {
        self.fields().filter_map(redundant_field_shorthand)
    }

    /// Return a source-preserving replacement for all redundant field assignments.
    ///
    /// The replacement changes only exact shorthand-equivalent field arguments;
    /// comments, malformed delimiters, and mismatched source arguments return `None`.
    /// The original macro path, delimiters, separators, and unrelated arguments stay
    /// byte-for-byte unchanged in the returned source string.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value| {
    ///     let _ = tracing_support::TracingMacroInvocation::redundant_field_assignment_replacement(value);
    /// };
    /// ```
    pub fn redundant_field_assignment_replacement(&self) -> Option<String> {
        let (after_bang, contents) = macro_contents(&self.source)?;
        let parts = split_top_level(contents, b',');
        if !is_exact_argument_split(&parts, &self.arguments) {
            return None;
        }

        let replacements = redundant_field_ranges(contents, &parts)?;
        apply_field_replacements(
            &self.source,
            self.source.len() - after_bang.len() + 1,
            replacements,
        )
    }

    /// Return the field whose value is the provided `format!` invocation.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value, expression_source| {
    ///     let _ = tracing_support::TracingMacroInvocation::formatted_field_for(value, expression_source);
    /// };
    /// ```
    pub fn formatted_field_for(&self, expression_source: &str) -> Option<String> {
        let expression_source = compact_source(expression_source);
        self.fields().find_map(|field| {
            let (name, value) = split_assignment(field)?;
            (compact_source(value) == expression_source && is_format_invocation(value))
                .then(|| compact_source(name))
        })
    }

    /// Return the field whose value is the provided `.to_string()` expression.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |value, expression_source| {
    ///     let _ = tracing_support::TracingMacroInvocation::stringified_field_for(value, expression_source);
    /// };
    /// ```
    pub fn stringified_field_for(&self, expression_source: &str) -> Option<String> {
        let expression_source = compact_source(expression_source);
        self.fields().find_map(|field| {
            let (name, value) = split_assignment(field)?;
            (compact_source(value) == expression_source && is_to_string_call(value))
                .then(|| compact_source(name))
        })
    }

    /// Iterate over source arguments that are structured field assignments.
    fn fields(&self) -> impl Iterator<Item = &str> {
        let end = self.message_index().unwrap_or(self.arguments.len());
        self.arguments
            .get(..end)
            .unwrap_or_default()
            .iter()
            .map(String::as_str)
            .filter(|argument| split_assignment(argument).is_some())
    }

    /// Locate the message format string in an event macro.
    fn message_index(&self) -> Option<usize> {
        if !EVENT_MACROS.contains(&self.macro_name.as_str()) {
            return None;
        }
        self.arguments.iter().position(|argument| {
            split_assignment(argument).is_none() && rust_string_contents(argument).is_some()
        })
    }

    /// Collect the source values already preserved as structured fields.
    fn structured_values(&self, message_index: usize) -> Vec<String> {
        // Preserve field order so comparisons follow the original macro invocation.
        let mut values = Vec::new();
        for argument in self.arguments.get(..message_index).unwrap_or_default() {
            // Configuration and level arguments do not represent logged values.
            if is_macro_configuration(argument) || is_level_argument(argument) {
                continue;
            }
            // Normalize explicitly named fields before checking their value paths.
            if let Some((_, value)) = split_assignment(argument) {
                let (_, value) = strip_field_sigil(value);
                let value = compact_source(value);
                if is_field_path(&value) {
                    values.push(value);
                }
                continue;
            }

            // Apply the same sigil and path rules to shorthand fields.
            let (_, value) = strip_field_sigil(argument);
            let value = compact_source(value);
            if is_field_path(&value) {
                values.push(value);
            }
        }
        values
    }
}

/// Return the source after the macro bang and its delimiter contents.
fn macro_contents(source: &str) -> Option<(&str, &str)> {
    let bang = source.find('!')?;
    let after_bang = source.get(bang + 1..)?.trim_start();
    let opening = after_bang.as_bytes().first().copied()?;
    let closing = match opening {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => return None,
    };
    let end = matching_delimiter(after_bang, 0, opening, closing)?;
    Some((after_bang, after_bang.get(1..end)?))
}

/// Find exact source ranges for redundant field assignments.
fn redundant_field_ranges<'source>(
    contents: &'source str,
    parts: &[&'source str],
) -> Option<Vec<(usize, usize, String)>> {
    let mut replacements = Vec::new();
    for part in parts {
        let Some(shorthand) = redundant_field_shorthand(part.trim()) else {
            continue;
        };
        let has_comment_marker = part
            .as_bytes()
            .windows(2)
            .any(|window| matches!(window, [b'/', b'/' | b'*']));
        if has_comment_marker {
            return None;
        }
        let leading = part.len() - part.trim_start().len();
        let trailing = part.len() - part.trim_end().len();
        let start = part.as_ptr() as usize - contents.as_ptr() as usize;
        let value_start = start + leading;
        let value_end = part.len() - trailing + start;
        replacements.push((value_start, value_end, shorthand));
    }
    (!replacements.is_empty()).then_some(replacements)
}

/// Apply field replacements while retaining all untouched invocation source.
fn apply_field_replacements(
    source: &str,
    contents_start: usize,
    replacements: Vec<(usize, usize, String)>,
) -> Option<String> {
    let mut replacement = String::with_capacity(source.len());
    // Start at the full invocation so the replacement retains its macro name and delimiters.
    let mut cursor = 0;
    for (start, end, shorthand) in replacements {
        let start = contents_start + start;
        let end = contents_start + end;
        replacement.push_str(source.get(cursor..start)?);
        replacement.push_str(&shorthand);
        cursor = end;
    }
    replacement.push_str(source.get(cursor..)?);
    Some(replacement)
}

/// Recover one resolved tracing event or span macro invocation from expanded HIR.
///
/// The helper follows macro hygiene to the public `tracing` definition and parses
/// only top-level arguments, allowing callers to retain a source-level diagnostic.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tracing_support::tracing_macro_invocation(cx, expr);
/// };
/// ```
pub fn tracing_macro_invocation(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
) -> Option<TracingMacroInvocation> {
    let (macro_name, span) = tracing_macro_expansion(cx, expr.span)?;
    let source = cx.sess().source_map().span_to_snippet(span).ok()?;
    let arguments = macro_arguments(&source)?;
    Some(TracingMacroInvocation {
        span,
        macro_name,
        arguments,
        source,
    })
}

/// Return whether an expression resolves to the standard `ToString::to_string` method.
///
/// Resolution uses the canonical definition path rather than method spelling, so
/// an unrelated local trait or inherent method cannot trigger a tracing lint.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tracing_support::is_to_string_trait_call(cx, expr);
/// };
/// ```
pub fn is_to_string_trait_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Resolve only method calls before comparing the canonical trait path.
    if !matches!(expr.kind, ExprKind::MethodCall(..)) {
        return false;
    }
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    cx.tcx
        .def_path_str(def_id)
        .ends_with("::ToString::to_string")
}

/// Recover one standard-library `format!` invocation from expanded HIR.
///
/// The expansion chain is accepted only when rustc attributes the macro to `std`,
/// `core`, or `alloc`, which keeps formatted-field checks independent of local macros.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tracing_support::standard_format_invocation(cx, expr);
/// };
/// ```
pub fn standard_format_invocation(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Walk outward through macro hygiene from the expanded expression.
    let mut context = expr.span.ctxt();
    while context != SyntaxContext::root() {
        // Accept only the standard format macro from an owning standard crate.
        let expansion = context.outer_expn_data();
        if let (ExpnKind::Macro(MacroKind::Bang, macro_name), Some(def_id)) =
            (expansion.kind, expansion.macro_def_id)
            && macro_name.as_str() == "format"
            && matches!(
                cx.tcx.crate_name(def_id.krate).as_str(),
                "alloc" | "core" | "std"
            )
        {
            return Some(expansion.call_site);
        }

        // Stop when malformed hygiene points back to the same context.
        let next = expansion.call_site.ctxt();
        if next == context {
            break;
        }
        context = next;
    }
    None
}

/// One semantically resolved tracing method call and its original arguments.
///
/// The result preserves the method token span and borrowed HIR expressions needed
/// by lints that inspect a tracing receiver or constructor argument.
#[derive(Clone, Copy, Debug)]
pub struct TracingMethodCall<'hir> {
    /// Span of the method identifier used for the primary diagnostic.
    /// This span is suitable for a replacement that changes only the method name.
    pub method_span: Span,
    /// Method receiver expression borrowed from the analyzed HIR body.
    pub receiver: &'hir Expr<'hir>,
    /// Explicit method arguments borrowed in their original source order.
    pub arguments: &'hir [Expr<'hir>],
}

/// Resolve a method call to one exact tracing definition.
///
/// Type-dependent resolution excludes extension methods and local lookalikes while
/// accepting public tracing re-exports whose displayed path contains private modules.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_path| {
///     let _ = tracing_support::tracing_method_call(cx, expr, expected_path);
/// };
/// ```
pub fn tracing_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_path: &str,
) -> Option<TracingMethodCall<'hir>> {
    // Type-dependent resolution rejects extension traits and user methods with the same spelling.
    let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    is_tracing_def(cx, def_id, expected_path).then_some(TracingMethodCall {
        method_span: segment.ident.span,
        receiver,
        arguments,
    })
}

/// Return the arguments when an expression calls one exact tracing function.
///
/// Direct path resolution accepts aliases only after they resolve to one of the
/// requested canonical tracing paths, preserving the original argument slice.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_paths| {
///     let _ = tracing_support::tracing_function_arguments(cx, expr, expected_paths);
/// };
/// ```
pub fn tracing_function_arguments<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_paths: &[&str],
) -> Option<&'hir [Expr<'hir>]> {
    // Resolve re-exports to their canonical definitions before matching.
    let ExprKind::Call(callee, arguments) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return None;
    };

    // Match aliases by resolved definition against every accepted canonical path.
    expected_paths
        .iter()
        .any(|expected_path| is_tracing_def(cx, def_id, expected_path))
        .then_some(arguments)
}

/// Return whether an expression directly calls one exact tracing function.
///
/// This predicate shares the same resolved-definition test as the argument helper,
/// so boolean-only callers cannot drift from the full semantic contract.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_paths| {
///     let _ = tracing_support::is_tracing_function_call(cx, expr, expected_paths);
/// };
/// ```
pub fn is_tracing_function_call(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    expected_paths: &[&str],
) -> bool {
    tracing_function_arguments(cx, expr, expected_paths).is_some()
}

/// Match a tracing method whose receiver is one direct tracing constructor.
///
/// The receiver must resolve to a requested tracing constructor before the method
/// match is returned, excluding wrappers and unrelated values with matching names.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, method_path, constructor_paths| {
///     let _ = tracing_support::method_on_direct_span(cx, expr, method_path, constructor_paths);
/// };
/// ```
pub fn method_on_direct_span<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    method_path: &str,
    constructor_paths: &[&str],
) -> Option<TracingMethodCall<'hir>> {
    let call = tracing_method_call(cx, expr, method_path)?;
    is_tracing_function_call(cx, call.receiver, constructor_paths).then_some(call)
}

/// Match a tracing method with one direct tracing constructor argument.
///
/// Exactly one argument is required, and that argument must resolve to a requested
/// tracing constructor before the method is considered a direct span operation.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, method_path, constructor_paths| {
///     let _ = tracing_support::method_with_direct_span_argument(cx, expr, method_path, constructor_paths);
/// };
/// ```
pub fn method_with_direct_span_argument<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    method_path: &str,
    constructor_paths: &[&str],
) -> Option<TracingMethodCall<'hir>> {
    let call = tracing_method_call(cx, expr, method_path)?;
    matches!(call.arguments, [argument] if is_tracing_function_call(cx, argument, constructor_paths))
        .then_some(call)
}

/// Declare one direct-span tracing simplification.
///
/// The macro generates a semantic lint pass that reports only the requested method
/// and constructor pair, with diagnostics inherited from the declaring lint crate.
#[macro_export]
macro_rules! declare_direct_span_lint {
    (
        $lint:ident,
        $pass:ident,
        $matcher:ident,
        $method:literal,
        [$($constructor:literal),+ $(,)?],
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
            /// Check one resolved tracing method and direct span constructor.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(call) = $crate::$matcher(cx, expr, $method, &[$($constructor),+])
                else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    call.method_span,
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

/// Return whether a closure is explicitly async or immediately returns an async block.
///
/// The check follows coroutine metadata and one returned async expression, while
/// excluding ordinary closures and source shapes that do not execute asynchronously.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tracing_support::is_explicit_async_closure(cx, expr);
/// };
/// ```
pub fn is_explicit_async_closure(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Reject other expression forms before inspecting closure coroutine metadata.
    let ExprKind::Closure(closure) = expr.kind else {
        return false;
    };
    if is_async_closure_kind(closure.kind) {
        return true;
    }

    // A synchronous `|| async { ... }` closure returns the future after the scope has ended.
    let body = cx.tcx.hir_body(closure.body);
    is_async_expression(body.value)
}

/// Return whether a definition belongs to the tracing API at the target API path.
fn is_tracing_def(cx: &LateContext<'_>, def_id: DefId, expected_path: &str) -> bool {
    if !matches!(
        cx.tcx.crate_name(def_id.krate).as_str(),
        "tracing" | "tracing_core"
    ) {
        return false;
    }

    // Public re-exports can flatten private modules in rustc's displayed definition path.
    let actual_path = cx.tcx.def_path_str(def_id);
    actual_path == expected_path || owner_and_item(&actual_path) == owner_and_item(expected_path)
}

/// Return the final owner and item segments of a definition path.
fn owner_and_item(path: &str) -> Option<(&str, &str)> {
    let mut segments = path.rsplit("::");
    let item = segments.next()?;
    let owner = segments.next()?;
    Some((owner, item))
}

/// Recognize both async blocks and async closures.
const fn is_async_closure_kind(kind: ClosureKind) -> bool {
    matches!(
        kind,
        ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _))
            | ClosureKind::CoroutineClosure(CoroutineDesugaring::Async)
    )
}

/// Strip compiler wrappers before checking for an async block expression.
fn is_async_expression(expr: &Expr<'_>) -> bool {
    // Recognize closure nodes before peeling compiler-generated wrappers.
    if let ExprKind::Closure(closure) = expr.kind {
        return is_async_closure_kind(closure.kind);
    }
    // Peel transient and block wrappers recursively to reach the user expression.
    if let ExprKind::DropTemps(inner) = expr.kind {
        return is_async_expression(inner);
    }
    if let ExprKind::Block(block, _) = expr.kind {
        return block.expr.is_some_and(is_async_expression);
    }
    false
}

/// Walk expansion hygiene until a public tracing field macro is found.
fn tracing_macro_expansion(cx: &LateContext<'_>, span: Span) -> Option<(String, Span)> {
    // Start at the expanded HIR span and walk toward the public macro call.
    let mut context = span.ctxt();
    while context != SyntaxContext::root() {
        // Require a field macro whose resolved definition belongs to tracing.
        let expansion = context.outer_expn_data();
        if let Some(macro_name) = tracing_field_macro_name(cx, &expansion) {
            return Some((macro_name, expansion.call_site));
        }

        // Guard against expansion cycles before moving to the caller context.
        let next = expansion.call_site.ctxt();
        if next == context {
            break;
        }
        context = next;
    }
    None
}

/// Return the name of a field macro expansion whose definition belongs to tracing.
fn tracing_field_macro_name(cx: &LateContext<'_>, expansion: &ExpnData) -> Option<String> {
    // Only bang macros with a resolved definition can be public tracing field macros.
    let (ExpnKind::Macro(MacroKind::Bang, macro_name), Some(def_id)) =
        (&expansion.kind, expansion.macro_def_id)
    else {
        return None;
    };
    // Reject same-named macros from other crates and tracing macros without fields.
    let is_tracing_macro = cx.tcx.crate_name(def_id.krate).as_str() == "tracing";
    let is_field_macro = FIELD_MACROS.contains(&macro_name.as_str());
    (is_tracing_macro && is_field_macro).then(|| macro_name.to_string())
}

/// Return whether split source parts are exactly the parsed macro arguments.
fn is_exact_argument_split(parts: &[&str], arguments: &[String]) -> bool {
    // A different count means the split disagrees with the parsed invocation.
    if parts.len() != arguments.len() {
        return false;
    }
    parts
        .iter()
        .zip(arguments)
        .all(|(part, argument)| part.trim() == argument)
}

/// Split a complete macro invocation into its top-level comma-separated arguments.
fn macro_arguments(source: &str) -> Option<Vec<String>> {
    // Determine the invocation delimiter from the source after the macro bang.
    let bang = source.find('!')?;
    let after_bang = source.get(bang + 1..)?.trim_start();
    let opening = after_bang.as_bytes().first().copied()?;
    let closing = match opening {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => return None,
    };
    let end = matching_delimiter(after_bang, 0, opening, closing)?;
    // Split only top-level commas and discard empty argument slots.
    let contents = after_bang.get(1..end)?;
    Some(
        split_top_level(contents, b',')
            .into_iter()
            .map(str::trim)
            .filter(|argument| !argument.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

/// Find the closing delimiter paired with `start`, ignoring literals and comments.
fn matching_delimiter(source: &str, start: usize, opening: u8, closing: u8) -> Option<usize> {
    // Track nested copies of the delimiter pair.
    let bytes = source.as_bytes();
    let mut index = start;
    let mut depth = 0_usize;
    while index < bytes.len() {
        // Ignore delimiters that occur inside comments or literals.
        if let Some(next) = skipped_token_end(source, index) {
            index = next;
            continue;
        }
        let byte = *bytes.get(index)?;
        // Update depth only for the opening and closing bytes.
        match byte {
            byte if byte == opening => depth = depth.checked_add(1)?,
            byte if byte == closing => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    // The first return to zero closes the original delimiter.
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// Split source on one delimiter while preserving nested Rust syntax.
fn split_top_level(source: &str, delimiter: u8) -> Vec<&str> {
    // Preserve borrowed source slices and their original order.
    let bytes = source.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0_usize;
    let mut index = 0_usize;
    // Store expected closing delimiters for all nested groups.
    let mut delimiters = Vec::new();
    while index < bytes.len() {
        // Skip literals and comments because their punctuation is data.
        if let Some(next) = skipped_token_end(source, index) {
            index = next;
            continue;
        }
        let Some(&byte) = bytes.get(index) else {
            break;
        };
        // Split only when the delimiter appears outside every nested group.
        match byte {
            b'(' => delimiters.push(b')'),
            b'[' => delimiters.push(b']'),
            b'{' => delimiters.push(b'}'),
            byte if delimiters.last().copied() == Some(byte) => {
                let _closing = delimiters.pop();
            }
            byte if byte == delimiter && delimiters.is_empty() => {
                if let Some(part) = source.get(start..index) {
                    parts.push(part);
                }
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    // Append the final segment after the last delimiter.
    if let Some(part) = source.get(start..) {
        parts.push(part);
    }
    parts
}

/// Find one top-level assignment operator.
fn split_assignment(source: &str) -> Option<(&str, &str)> {
    let index = top_level_byte(source, b'=')?;
    if source.as_bytes().get(index + 1) == Some(&b'>') {
        return None;
    }
    Some((source.get(..index)?, source.get(index + 1..)?))
}

/// Return the shorthand spelling for one redundant field assignment.
fn redundant_field_shorthand(field: &str) -> Option<String> {
    let (name, value) = split_assignment(field)?;
    let (sigil, value) = strip_field_sigil(value);
    let name = compact_source(name);
    let value = compact_source(value);
    (name == value && is_field_path(&name)).then(|| format!("{sigil}{value}"))
}

/// Locate a byte outside nested syntax, literals, and comments.
fn top_level_byte(source: &str, target: u8) -> Option<usize> {
    // Track group nesting while scanning the original source bytes.
    let bytes = source.as_bytes();
    let mut index = 0_usize;
    let mut delimiters = Vec::new();
    while index < bytes.len() {
        // Ignore targets embedded inside literals or comments.
        if let Some(next) = skipped_token_end(source, index) {
            index = next;
            continue;
        }
        let Some(&byte) = bytes.get(index) else {
            break;
        };
        // Return the target only when no group delimiter remains open.
        match byte {
            b'(' => delimiters.push(b')'),
            b'[' => delimiters.push(b']'),
            b'{' => delimiters.push(b'}'),
            byte if delimiters.last().copied() == Some(byte) => {
                let _closing = delimiters.pop();
            }
            byte if byte == target && delimiters.is_empty() => return Some(index),
            _ => {}
        }
        index += 1;
    }
    None
}

/// Skip a literal or comment starting at `index`.
fn skipped_token_end(source: &str, index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    match bytes.get(index..)? {
        [b'/', b'/', ..] => Some(
            source
                .get(index..)?
                .find('\n')
                .map_or(bytes.len(), |offset| index + offset + 1),
        ),
        [b'/', b'*', ..] => block_comment_end(bytes, index),
        [b'"', ..] => Some(cooked_literal_end(bytes, index, b'"')),
        [b'\'', ..] => char_literal_end(bytes, index),
        [b'b' | b'c', b'"', ..] => Some(cooked_literal_end(bytes, index + 1, b'"')),
        [b'b', b'\'', ..] => Some(cooked_literal_end(bytes, index + 1, b'\'')),
        [b'r', ..] => raw_literal_end(bytes, index),
        [b'b' | b'c', b'r', ..] => raw_literal_end(bytes, index + 1),
        _ => None,
    }
}

/// Skip a nested block comment.
fn block_comment_end(bytes: &[u8], start: usize) -> Option<usize> {
    // Begin after the opening marker with one active comment level.
    let mut index = start + 2;
    let mut depth = 1_usize;
    while index + 1 < bytes.len() {
        // Count nested openers and closers without interpreting comment contents.
        match bytes.get(index..index + 2)? {
            [b'/', b'*'] => {
                depth = depth.checked_add(1)?;
                index += 2;
            }
            [b'*', b'/'] => {
                depth = depth.checked_sub(1)?;
                index += 2;
                if depth == 0 {
                    // Return the first byte after the balanced closing marker.
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    Some(bytes.len())
}

/// Skip a cooked string or byte literal, including escape sequences.
fn cooked_literal_end(bytes: &[u8], quote: usize, delimiter: u8) -> usize {
    // Start after the opening quote and skip escaped byte pairs atomically.
    let mut index = quote + 1;
    while index < bytes.len() {
        let Some(&byte) = bytes.get(index) else {
            break;
        };
        // Return after the first unescaped matching delimiter.
        match byte {
            b'\\' => index = (index + 2).min(bytes.len()),
            byte if byte == delimiter => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

/// Treat a quote as a character literal only when a closing quote is nearby.
fn char_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    let end = cooked_literal_end(bytes, start, b'\'');
    (end > start + 2 && end <= start + 8).then_some(end)
}

/// Skip a raw string after its `r` prefix.
fn raw_literal_end(bytes: &[u8], raw_prefix: usize) -> Option<usize> {
    // Count opening hashes before validating the raw string quote.
    let mut quote = raw_prefix + 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return None;
    }
    // Preserve the exact hash sequence required after the closing quote.
    let hashes = quote.checked_sub(raw_prefix + 1)?;
    let expected_hashes = bytes.get(raw_prefix + 1..quote)?;
    let mut index = quote + 1;
    while index < bytes.len() {
        // Accept a quote only when the complete opening hash count follows it.
        if is_raw_string_close(bytes, index, expected_hashes) {
            return Some(index + 1 + hashes);
        }
        index += 1;
    }
    // Treat an unterminated raw literal as extending through the available source.
    Some(bytes.len())
}

/// Return whether a quote at `index` is followed by the expected closing hashes.
fn is_raw_string_close(bytes: &[u8], index: usize, expected_hashes: &[u8]) -> bool {
    let is_quote = bytes.get(index) == Some(&b'"');
    let hashes_start = index + 1;
    let hashes_end = hashes_start + expected_hashes.len();
    is_quote && bytes.get(hashes_start..hashes_end) == Some(expected_hashes)
}

/// Extract the contents of one complete cooked or raw string literal.
fn rust_string_contents(source: &str) -> Option<String> {
    // Trim surrounding whitespace before identifying the literal spelling.
    let source = source.trim();
    // Handle ordinary strings without applying raw-string suffix rules.
    if let Some(without_opening) = source.strip_prefix('"')
        && let Some(contents) = without_opening.strip_suffix('"')
    {
        return Some(contents.to_owned());
    }
    let prefix_end = source.find('"')?;
    let prefix = source.get(..prefix_end)?;
    // Require an `r` prefix followed only by hash delimiters.
    let is_raw_prefix = prefix.starts_with('r');
    if !is_raw_prefix {
        return None;
    }
    let hashes = prefix.get(1..)?;
    let has_only_hashes = hashes.bytes().all(|byte| byte == b'#');
    if !has_only_hashes {
        return None;
    }
    // Remove the closing quote and the same hash sequence used by the prefix.
    let suffix = format!("\"{hashes}");
    source
        .get(prefix_end + 1..)?
        .strip_suffix(&suffix)
        .map(str::to_owned)
}

/// Resolve simple captured and explicit format arguments from one format string.
fn format_values(format_string: &str, format_arguments: &[String]) -> Vec<String> {
    // Separate positional arguments from explicitly named assignments.
    let positional: Vec<_> = format_arguments
        .iter()
        .filter(|argument| split_assignment(argument).is_none())
        .map(|argument| compact_source(argument))
        .collect();
    let named: Vec<_> = format_arguments
        .iter()
        .filter_map(|argument| {
            let (name, value) = split_assignment(argument)?;
            Some((compact_source(name), compact_source(value)))
        })
        .collect();

    // Scan placeholders while preserving their order in the format string.
    let bytes = format_string.as_bytes();
    let mut values = Vec::new();
    let mut index = 0_usize;
    let mut implicit_position = 0_usize;
    while index < bytes.len() {
        // Advance over ordinary bytes and escaped opening braces.
        if bytes.get(index) != Some(&b'{') {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) == Some(&b'{') {
            index += 2;
            continue;
        }
        let Some(end_offset) = format_string
            .get(index + 1..)
            .and_then(|tail| tail.find('}'))
        else {
            break;
        };
        // Separate the argument selector from formatting options.
        let end = index + 1 + end_offset;
        let placeholder = format_string.get(index + 1..end).unwrap_or_default();
        let argument = placeholder.split(':').next().unwrap_or_default().trim();
        // Resolve implicit, indexed, named, and captured arguments distinctly.
        let value = resolve_format_argument(argument, &positional, &named, &mut implicit_position);
        // Retain only values that can duplicate a structured field path.
        if let Some(value) = value
            && is_field_path(&value)
        {
            values.push(value);
        }
        index = end + 1;
    }
    values
}

/// Resolve one implicit, indexed, named, or captured format argument.
fn resolve_format_argument(
    argument: &str,
    positional: &[String],
    named: &[(String, String)],
    implicit_position: &mut usize,
) -> Option<String> {
    // Consume implicit arguments in source order while advancing the caller's cursor.
    if argument.is_empty() {
        let value = positional.get(*implicit_position).cloned();
        *implicit_position += 1;
        return value;
    }
    // Resolve explicit positions before named arguments and captured identifiers.
    if let Ok(position) = argument.parse::<usize>() {
        return positional.get(position).cloned();
    }
    named
        .iter()
        .find_map(|(name, value)| (name == argument).then(|| value.clone()))
        .or_else(|| Some(argument.to_owned()))
}

/// Remove whitespace without changing identifier or punctuation spelling.
fn compact_source(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// Separate tracing's `%` and `?` field recording sigils.
fn strip_field_sigil(source: &str) -> (&str, &str) {
    let source = source.trim();
    source.strip_prefix('%').map_or_else(
        || {
            source
                .strip_prefix('?')
                .map_or(("", source), |value| ("?", value))
        },
        |value| ("%", value),
    )
}

/// Return whether source is a field path accepted by tracing's shorthand grammar.
fn is_field_path(source: &str) -> bool {
    !source.is_empty()
        && source.split('.').all(|segment| {
            let segment = segment.strip_prefix("r#").unwrap_or(segment);
            let mut characters = segment.chars();
            characters
                .next()
                .is_some_and(|character| character == '_' || character.is_alphabetic())
                && characters.all(|character| character == '_' || character.is_alphanumeric())
        })
}

/// Recognize macro configuration arguments rather than shorthand fields.
fn is_macro_configuration(source: &str) -> bool {
    ["target:", "parent:", "name:"]
        .iter()
        .any(|prefix| source.trim_start().starts_with(prefix))
}

/// Recognize the explicit verbosity argument used by `event!` and `span!`.
fn is_level_argument(source: &str) -> bool {
    compact_source(source).contains("Level::")
}

/// Recognize a `format!` call used directly as a field value.
fn is_format_invocation(source: &str) -> bool {
    let source = compact_source(source);
    ["format!(", "std::format!(", "::std::format!("]
        .iter()
        .any(|prefix| source.starts_with(prefix) && source.ends_with(')'))
}

/// Recognize a direct `.to_string()` field value.
fn is_to_string_call(source: &str) -> bool {
    let source = compact_source(source);
    source
        .strip_suffix(".to_string()")
        .is_some_and(is_field_path)
}

#[cfg(test)]
mod tests {
    use super::{TracingMacroInvocation, macro_arguments};
    use rustc_span::DUMMY_SP;

    /// Parse a macro source fixture into an invocation, panicking on malformed fixtures.
    fn invocation_fixture(source: &str) -> TracingMacroInvocation {
        TracingMacroInvocation {
            span: DUMMY_SP,
            macro_name: "info".to_owned(),
            arguments: macro_arguments(source).expect("test macro source parses"),
            source: source.to_owned(),
        }
    }

    #[test]
    fn rewrites_redundant_assignments_in_place() {
        let invocation = invocation_fixture("info!(request_id = request_id, user.id = user.id);");
        assert_eq!(
            invocation.redundant_field_assignment_replacement(),
            Some("info!(request_id, user.id);".to_owned())
        );
    }

    #[test]
    fn preserves_sigil_and_rejects_comment_rewrites() {
        let invocation = invocation_fixture("debug_span!(\"request\", request_id = %request_id);");
        assert_eq!(
            invocation.redundant_field_assignment_replacement(),
            Some("debug_span!(\"request\", %request_id);".to_owned())
        );

        let invocation = invocation_fixture("info!(request_id = request_id /* keep this */);");
        assert_eq!(invocation.redundant_field_assignment_replacement(), None);
    }
}
