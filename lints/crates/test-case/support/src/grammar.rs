//! A typed mirror of the attribute grammar that `test-case-core` 3.3 parses.
//!
//! The parser keeps only the nodes that lint rules inspect. It accepts the
//! inputs, the optional `=> expectation`, and the optional `; "description"`
//! of both `#[test_case]` and `#[test_matrix]`.

use syn::{
    Expr, Ident, LitStr, Pat, Token, bracketed,
    ext::IdentExt as _,
    parenthesized,
    parse::{ParseBuffer, ParseStream, Result, discouraged::Speculative as _},
    punctuated::Punctuated,
    token,
};

/// The parsed arguments of one test-case attribute.
#[derive(Debug)]
pub(crate) struct CaseArgs {
    /// Case arguments, or matrix dimensions for `test_matrix`.
    pub(crate) inputs: Vec<Expr>,
    /// The assertion after `=>`, if any.
    pub(crate) expectation: Option<Expectation>,
    /// The description after `;`, if any.
    pub(crate) description: Option<LitStr>,
    /// The whole attribute text; syn spans are byte offsets into it.
    pub(crate) source: String,
}

/// The assertion after `=>`.
#[derive(Debug)]
pub(crate) struct Expectation {
    /// Skip modifiers written before the result.
    pub(crate) modifiers: Vec<Modifier>,
    /// The checked result.
    pub(crate) result: ExpectedResult,
}

/// One `ignore` or `inconclusive` modifier.
#[derive(Debug)]
pub(crate) struct Modifier {
    /// The modifier keyword.
    pub(crate) keyword: Ident,
    /// The bracketed reason, if any.
    pub(crate) reason: Option<LitStr>,
}

/// The result checked by an expectation.
#[derive(Debug)]
pub(crate) enum ExpectedResult {
    /// Only modifiers, with no result check.
    Empty,
    /// A value compared with `assert_eq!`.
    Simple,
    /// `matches pattern [if guard]`.
    Matching {
        /// The match pattern.
        pattern: Pat,
        /// The guard expression, if any.
        guard: Option<Expr>,
    },
    /// `panics [message]`.
    Panicking(Option<Expr>),
    /// `with expression`.
    With {
        /// The `with` keyword.
        keyword: Ident,
        /// The validator expression.
        expr: Expr,
    },
    /// `using path`.
    UseFn,
    /// `it ...` or `is ...`, flattened to its leaf matchers.
    Complex(Vec<Leaf>),
}

/// One leaf matcher of a complex assertion.
#[derive(Debug)]
pub(crate) enum Leaf {
    /// `almost value precision precision`.
    Almost {
        /// The precision expression.
        precision: Expr,
    },
    /// `contains_in_order expected`.
    ContainsInOrder {
        /// The expected sequence.
        expected: Expr,
    },
    /// Any other matcher.
    Other,
}

impl CaseArgs {
    /// Parse attribute arguments in test-case's order.
    pub(crate) fn parse(input: ParseStream<'_>) -> Result<Self> {
        let inputs = Punctuated::<Expr, Token![,]>::parse_separated_nonempty(input)?;
        let expectation = if input.peek(Token![=>]) {
            let _arrow: Token![=>] = input.parse()?;
            Some(Expectation::parse(input)?)
        } else {
            None
        };
        let description = if input.peek(Token![;]) {
            let _semicolon: Token![;] = input.parse()?;
            Some(input.parse()?)
        } else {
            None
        };
        // Reject trailing tokens; test-case would reject the attribute too.
        if !input.is_empty() {
            return Err(input.error("unexpected tokens"));
        }
        Ok(Self {
            inputs: inputs.into_iter().collect(),
            expectation,
            description,
            source: String::new(),
        })
    }
}

impl Expectation {
    /// Parse modifiers and then exactly one result form.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut modifiers = Vec::new();
        while let Some(keyword) = peek_keyword(input, &["ignore", "inconclusive"]) {
            input.advance_to(&keyword.1);
            let reason = if input.peek(token::Bracket) {
                let content;
                let _bracket = bracketed!(content in input);
                Some(content.parse()?)
            } else {
                None
            };
            modifiers.push(Modifier {
                keyword: keyword.0,
                reason,
            });
        }
        let result = ExpectedResult::parse(input, !modifiers.is_empty())?;
        Ok(Self { modifiers, result })
    }
}

impl ExpectedResult {
    /// Parse one result form after the modifiers.
    fn parse(input: ParseStream<'_>, has_modifiers: bool) -> Result<Self> {
        let is_at_end = input.is_empty() || input.peek(Token![;]);
        let keyword = peek_keyword(input, &["matches", "it", "is", "using", "with", "panics"]);
        let Some((keyword, fork)) = keyword else {
            if is_at_end && has_modifiers {
                return Ok(Self::Empty);
            }
            let _value: Expr = input.parse()?;
            return Ok(Self::Simple);
        };
        input.advance_to(&fork);
        if keyword == "matches" {
            let pattern = Pat::parse_single(input)?;
            let guard = if input.peek(Token![if]) {
                let _if: Token![if] = input.parse()?;
                Some(input.parse()?)
            } else {
                None
            };
            return Ok(Self::Matching { pattern, guard });
        }
        if keyword == "it" || keyword == "is" {
            let mut leaves = Vec::new();
            parse_complex(input, &mut leaves)?;
            return Ok(Self::Complex(leaves));
        }
        if keyword == "using" {
            let _path: Expr = input.parse()?;
            return Ok(Self::UseFn);
        }
        if keyword == "with" {
            return Ok(Self::With {
                keyword,
                expr: input.parse()?,
            });
        }
        // `panics` is the only remaining keyword; its message is optional.
        Ok(Self::Panicking(
            (!input.is_empty() && !input.peek(Token![;]))
                .then(|| input.parse())
                .transpose()?,
        ))
    }
}

/// Parse a complex assertion joined by `and` or `or`.
fn parse_complex(input: ParseStream<'_>, leaves: &mut Vec<Leaf>) -> Result<()> {
    parse_complex_item(input, leaves)?;
    while let Some((_, fork)) = peek_keyword(input, &["and", "or"]) {
        input.advance_to(&fork);
        parse_complex_item(input, leaves)?;
    }
    Ok(())
}

/// Parse one complex matcher or parenthesized group.
fn parse_complex_item(input: ParseStream<'_>, leaves: &mut Vec<Leaf>) -> Result<()> {
    if input.peek(token::Paren) {
        let content;
        let _paren = parenthesized!(content in input);
        return parse_complex(&content, leaves);
    }
    let (keyword, fork) = peek_keyword(input, COMPLEX_KEYWORDS)
        .ok_or_else(|| input.error("unknown complex matcher"))?;
    input.advance_to(&fork);
    if keyword == "almost" || keyword == "almost_equal_to" {
        let _value: Expr = input.parse()?;
        let (_, fork) = peek_keyword(input, &["precision"])
            .ok_or_else(|| input.error("expected `precision`"))?;
        input.advance_to(&fork);
        leaves.push(Leaf::Almost {
            precision: input.parse()?,
        });
    } else if keyword == "contains_in_order" {
        leaves.push(Leaf::ContainsInOrder {
            expected: input.parse()?,
        });
    } else if keyword == "not" {
        parse_complex(input, leaves)?;
    } else if keyword == "existing_path"
        || keyword == "directory"
        || keyword == "dir"
        || keyword == "file"
        || keyword == "empty"
    {
        leaves.push(Leaf::Other);
    } else {
        // Every other matcher takes one expression argument.
        let _value: Expr = input.parse()?;
        leaves.push(Leaf::Other);
    }
    Ok(())
}

/// The keywords that start a complex matcher.
const COMPLEX_KEYWORDS: &[&str] = &[
    "eq",
    "equal_to",
    "lt",
    "less_than",
    "gt",
    "greater_than",
    "leq",
    "less_or_equal_than",
    "geq",
    "greater_or_equal_than",
    "almost",
    "almost_equal_to",
    "existing_path",
    "directory",
    "dir",
    "file",
    "contains",
    "contains_in_order",
    "not",
    "len",
    "has_length",
    "count",
    "has_count",
    "empty",
    "matching_regex",
    "matches_regex",
];

/// Peek one of the listed identifiers and return it with the advanced fork.
fn peek_keyword<'a>(input: ParseStream<'a>, keywords: &[&str]) -> Option<(Ident, ParseBuffer<'a>)> {
    let fork = input.fork();
    let ident = Ident::parse_any(&fork).ok()?;
    keywords
        .contains(&ident.to_string().as_str())
        .then_some((ident, fork))
}
