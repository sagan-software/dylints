#![feature(rustc_private)]

//! A lint to check for interpolated logging messages.
//!
//! It inspects pre-expansion macro spelling and token structure. A macro whose
//! last path segment has a supported logging name is treated as a logging macro;
//! this lint does not resolve the macro's identity. It reads `concat!` using the
//! built-in macro's literal rules without resolving that macro's identity.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::{
    MacCall,
    ast::LitKind as AstLitKind,
    token::{Delimiter, Token, TokenKind},
    tokenstream::{TokenStream, TokenTree},
};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_pre_expansion_lint! {
    #[doc = include_str!("../README.md")]
    pub INTERPOLATED_LOGGING,
    Warn,
    "logging message uses string interpolation instead of structured fields",
    InterpolatedLogging
}

impl EarlyLintPass for InterpolatedLogging {
    /// Check mac for this lint.
    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &MacCall) {
        let Some(name) = logging_macro_name(mac) else {
            return;
        };

        // Inspect only the source message expression, leaving structured fields opaque.
        let Some(span) = interpolated_message_span(&mac.args.tokens, name) else {
            return;
        };
        emit_span_lint_with_help(
            cx,
            INTERPOLATED_LOGGING,
            span,
            "logging message uses string interpolation",
            "put runtime values in structured key/value fields and keep the message static",
        );
    }
}

/// Emit the interpolated-logging diagnostic with a concrete migration hint.
fn emit_span_lint_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _configured_message = diag.primary_message(message);
            let _configured_help = diag.help(help);
        }),
    );
}

/// Return the conventional logging macro name when the last path segment matches.
///
/// Formatted-output macros such as `println!` and `write!` are not logging.
///
/// Their text is program output, such as a `Display` impl or a command-line report.
fn logging_macro_name(mac: &MacCall) -> Option<&str> {
    let name = mac.path.segments.last()?.ident.name.as_str();
    matches!(
        name,
        "trace" | "debug" | "info" | "warn" | "error" | "event"
    )
    .then_some(name)
}

/// Return the source span of an interpolated message argument in a logging macro call.
fn interpolated_message_span(tokens: &TokenStream, macro_name: &str) -> Option<Span> {
    let trees = tokens.iter().collect::<Vec<_>>();

    // `log` separates structured key/value fields from its message with a semicolon.
    if let Some(separator) = trees.iter().position(|tree| {
        matches!(
            tree,
            TokenTree::Token(
                Token {
                    kind: TokenKind::Semi,
                    ..
                },
                _
            )
        )
    }) {
        // The `log` key/value form separates its message and format arguments after `;`.
        let (_, after_separator) = trees.split_at(separator);
        let message_arguments = split_top_level_arguments(after_separator.iter().skip(1).copied());
        return message_arguments
            .first()
            .and_then(|argument| interpolated_format_argument(argument));
    }

    let arguments = split_top_level_arguments(trees.into_iter());
    let mut event_level_pending = macro_name == "event";

    for argument in arguments {
        // Logging APIs put target, parent, name, or logger options before fields.
        if is_named_option_argument(&argument) {
            continue;
        }

        // `event!` takes its level as the first positional argument.
        if event_level_pending {
            event_level_pending = false;
            continue;
        }

        // Field assignments can contain arbitrary nested expressions and string literals.
        if is_field_argument(&argument) {
            continue;
        }

        // A string expression is the message; other positional expressions are fields.
        if string_literal_expression(&argument).is_some() {
            return interpolated_format_argument(&argument);
        }
    }

    None
}

/// Split a token stream at commas outside nested token-tree delimiters.
fn split_top_level_arguments<'a>(
    trees: impl Iterator<Item = &'a TokenTree>,
) -> Vec<Vec<&'a TokenTree>> {
    let mut arguments = Vec::new();
    let mut argument = Vec::new();

    for tree in trees {
        if matches!(
            tree,
            TokenTree::Token(
                Token {
                    kind: TokenKind::Comma,
                    ..
                },
                _
            )
        ) {
            if !argument.is_empty() {
                arguments.push(std::mem::take(&mut argument));
            }
        } else {
            argument.push(tree);
        }
    }

    if !argument.is_empty() {
        arguments.push(argument);
    }

    arguments
}

/// Return whether an argument begins with a logging macro option such as `target:`.
fn is_named_option_argument(argument: &[&TokenTree]) -> bool {
    let [
        TokenTree::Token(
            Token {
                kind: TokenKind::Ident(name, _),
                ..
            },
            _,
        ),
        TokenTree::Token(
            Token {
                kind: TokenKind::Colon,
                ..
            },
            _,
        ),
        ..,
    ] = argument
    else {
        return false;
    };

    matches!(name.as_str(), "target" | "parent" | "name" | "logger")
}

/// Return whether an argument is a structured field assignment or field block.
fn is_field_argument(argument: &[&TokenTree]) -> bool {
    if matches!(argument, [TokenTree::Delimited(_, _, Delimiter::Brace, _)]) {
        return true;
    }

    argument.iter().any(|tree| {
        matches!(
            tree,
            TokenTree::Token(
                Token {
                    kind: TokenKind::Eq,
                    ..
                },
                _
            )
        )
    })
}

/// Return the message span when a format expression contains a placeholder.
fn interpolated_format_argument(argument: &[&TokenTree]) -> Option<Span> {
    let (message, span) = string_literal_expression(argument)?;
    format_string_has_placeholder(&message).then_some(span)
}

/// Read a string literal or a literal `concat!` expression without descending into fields.
fn string_literal_expression(argument: &[&TokenTree]) -> Option<(String, Span)> {
    match argument {
        [
            TokenTree::Token(
                Token {
                    kind: TokenKind::Literal(literal),
                    span,
                },
                _,
            ),
        ] => match AstLitKind::from_token_lit(*literal).ok()? {
            // Decode Rust escapes before scanning the message text.
            AstLitKind::Str(message, _) => Some((message.as_str().to_owned(), *span)),
            AstLitKind::ByteStr(..)
            | AstLitKind::CStr(..)
            | AstLitKind::Byte(_)
            | AstLitKind::Char(_)
            | AstLitKind::Int(..)
            | AstLitKind::Float(..)
            | AstLitKind::Bool(_)
            | AstLitKind::Err(_) => None,
        },
        _ => concat_macro_arguments(argument).and_then(concat_string_expression),
    }
}

/// Read the argument group from a macro invocation whose final path segment is `concat`.
fn concat_macro_arguments<'tree>(argument: &[&'tree TokenTree]) -> Option<&'tree TokenStream> {
    match argument {
        [
            ..,
            TokenTree::Token(
                Token {
                    kind: TokenKind::Ident(name, _),
                    ..
                },
                _,
            ),
            TokenTree::Token(
                Token {
                    kind: TokenKind::Bang,
                    ..
                },
                _,
            ),
            TokenTree::Delimited(
                _,
                _,
                Delimiter::Parenthesis | Delimiter::Bracket | Delimiter::Brace,
                nested,
            ),
        ] if name.as_str() == "concat" => Some(nested),
        _ => None,
    }
}

/// Read a literal `concat!` expression and select its diagnostic span.
fn concat_string_expression(tokens: &TokenStream) -> Option<(String, Span)> {
    let mut combined = String::new();
    let mut first_part_span = None;
    let mut first_string_span = None;

    for part in split_top_level_arguments(tokens.iter()) {
        let (text, span, is_string) = concat_literal_part(&part)?;
        if first_part_span.is_none() {
            first_part_span = Some(span);
        }
        if is_string && first_string_span.is_none() {
            // Point at a string part when one exists, including cross-part placeholders.
            first_string_span = Some(span);
        }
        combined.push_str(&text);
    }

    Some((combined, first_string_span.or(first_part_span)?))
}

/// Read one literal accepted by the built-in `concat!` macro.
fn concat_literal_part(argument: &[&TokenTree]) -> Option<(String, Span, bool)> {
    concat_literal_token_part(argument)
        .or_else(|| concat_boolean_part(argument))
        .or_else(|| concat_negative_number_part(argument))
}

/// Decode a string, character, boolean, integer, or float token from a `concat!` argument.
fn concat_literal_token_part(argument: &[&TokenTree]) -> Option<(String, Span, bool)> {
    let [
        TokenTree::Token(
            Token {
                kind: TokenKind::Literal(literal),
                span,
            },
            _,
        ),
    ] = argument
    else {
        return None;
    };

    // Use rustc's semantic conversion so escapes and numeric spellings match macro expansion.
    let literal = AstLitKind::from_token_lit(*literal).ok()?;
    let (text, is_string) = match literal {
        AstLitKind::Str(message, _) => (message.as_str().to_owned(), true),
        AstLitKind::Char(character) => (character.to_string(), false),
        AstLitKind::Bool(value) => (value.to_string(), false),
        AstLitKind::Int(value, _) => (value.to_string(), false),
        AstLitKind::Float(value, _) => (value.as_str().to_owned(), false),
        AstLitKind::ByteStr(..)
        | AstLitKind::CStr(..)
        | AstLitKind::Byte(_)
        | AstLitKind::Err(_) => {
            return None;
        }
    };

    Some((text, *span, is_string))
}

/// Read a boolean identifier from a `concat!` argument.
fn concat_boolean_part(argument: &[&TokenTree]) -> Option<(String, Span, bool)> {
    match argument {
        [
            TokenTree::Token(
                Token {
                    kind: TokenKind::Ident(value, _),
                    span,
                },
                _,
            ),
        ] if matches!(value.as_str(), "true" | "false") => {
            Some((value.as_str().to_owned(), *span, false))
        }
        _ => None,
    }
}

/// Read a negative integer or float token pair from a `concat!` argument.
fn concat_negative_number_part(argument: &[&TokenTree]) -> Option<(String, Span, bool)> {
    match argument {
        [
            TokenTree::Token(
                Token {
                    kind: TokenKind::Minus,
                    span,
                },
                _,
            ),
            TokenTree::Token(
                Token {
                    kind: TokenKind::Literal(literal),
                    ..
                },
                _,
            ),
        ] => {
            // Decode the numeric token because an integer token can carry a float suffix.
            let value = match AstLitKind::from_token_lit(*literal).ok()? {
                AstLitKind::Int(value, _) => value.to_string(),
                AstLitKind::Float(value, _) => value.as_str().to_owned(),
                AstLitKind::Str(..)
                | AstLitKind::ByteStr(..)
                | AstLitKind::CStr(..)
                | AstLitKind::Byte(_)
                | AstLitKind::Char(_)
                | AstLitKind::Bool(_)
                | AstLitKind::Err(_) => return None,
            };
            Some((format!("-{value}"), *span, false))
        }
        _ => None,
    }
}

/// Return whether a format string contains an unescaped opening brace.
fn format_string_has_placeholder(message: &str) -> bool {
    // Scan characters so escaped braces remain distinguishable from placeholders.
    let mut chars = message.chars().peekable();

    while let Some(ch) = chars.next() {
        // Ignore ordinary characters until an opening brace appears.
        if ch != '{' {
            continue;
        }

        // Consume doubled opening braces because they render as literal text.
        if chars.peek().is_some_and(|next| *next == '{') {
            let _ = chars.next();
            continue;
        }

        return true;
    }

    false
}

/// Verify that the documented structured tracing calls compile.
#[test]
fn tracing_examples_compile() {
    use log as _;
    use tracing::{info, warn};

    let user_id = 42;
    let request_id = "req-7";
    info!(user_id, "user logged in");
    warn!(request_id, "request failed");
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "logging_macro_ui");
}
