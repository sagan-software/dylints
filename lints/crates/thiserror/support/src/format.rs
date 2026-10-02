//! Parse `#[error(...)]` attribute text the way thiserror reads it.
//!
//! thiserror parses its attribute arguments as tokens, not as Rust
//! expressions, because it accepts `.field` shorthand in format arguments.
//! This module mirrors that: it splits the arguments at top-level commas and
//! scans the format string with thiserror's placeholder rules. Every range is
//! a byte offset into the attribute text, so callers can map it to a span.

use proc_macro2::{Spacing, TokenTree};
use std::ops::Range;
use syn::{Attribute, LitStr, Meta, parse::Parser as _};

/// One parsed `#[error(...)]` attribute.
#[derive(Debug)]
pub struct ErrorAttr {
    /// The attribute arguments.
    pub kind: ErrorAttrKind,
    /// The attribute text from `#` to `]`; ranges index into it.
    pub text: String,
}

/// The arguments of an `#[error(...)]` attribute.
#[derive(Debug)]
pub enum ErrorAttrKind {
    /// `#[error(transparent)]`.
    Transparent,
    /// `#[error("format", args...)]`.
    Format(FormatAttr),
}

/// A format-string `#[error(...)]` attribute.
#[derive(Debug)]
pub struct FormatAttr {
    /// The decoded format string.
    pub value: String,
    /// The string literal token.
    pub literal: Range<usize>,
    /// The offset of the first value byte, when every value byte appears
    /// unchanged in the source (no escape sequences).
    pub body: Option<usize>,
    /// The extra format arguments, in source order.
    pub args: Vec<FormatArg>,
}

/// One extra format argument.
#[derive(Debug)]
pub struct FormatArg {
    /// The name of a `name = value` argument.
    pub name: Option<String>,
    /// The identifier text when the whole argument is one identifier.
    pub ident: Option<String>,
    /// The argument tokens.
    pub range: Range<usize>,
    /// The end of the previous literal or argument, where the separating comma starts.
    pub previous_end: usize,
}

/// One `{...}` placeholder in a format string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placeholder {
    /// The placeholder, braces included, as a byte range of the format string value.
    pub range: Range<usize>,
    /// The argument before the optional `:`.
    pub argument: Argument,
    /// The text after the argument, up to the closing brace, including any `:`.
    pub spec: String,
}

/// The argument of one placeholder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Argument {
    /// `{}` or `{:spec}`: the next positional argument.
    Implicit,
    /// `{N}`: a tuple field or positional argument by number.
    Index(usize),
    /// `{name}` or `{r#name}`: a field or named argument, spelled as written.
    Named(String),
}

impl ErrorAttr {
    /// Parse the text of one attribute; return `None` for other attributes.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// let _ = ErrorAttr::parse("#[error(\"{source}\")]");
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        let attributes = Attribute::parse_outer.parse_str(text).ok()?;
        let [attribute] = attributes.as_slice() else {
            return None;
        };
        let Meta::List(list) = &attribute.meta else {
            return None;
        };
        if !list.path.is_ident("error") {
            return None;
        }
        let tokens = list.tokens.clone().into_iter().collect::<Vec<_>>();
        let kind = match tokens.as_slice() {
            [TokenTree::Ident(ident)] if ident == "transparent" => ErrorAttrKind::Transparent,
            [TokenTree::Literal(literal), rest @ ..] => {
                ErrorAttrKind::Format(FormatAttr::parse(literal, rest)?)
            }
            _ => return None,
        };
        Some(Self {
            kind,
            text: text.to_owned(),
        })
    }

    /// Return the format-string arguments, if any.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let attribute = ErrorAttr::parse("#[error(\"{source}\")]").unwrap();
    /// let _ = attribute.format();
    /// ```
    pub const fn format(&self) -> Option<&FormatAttr> {
        match &self.kind {
            ErrorAttrKind::Format(format) => Some(format),
            ErrorAttrKind::Transparent => None,
        }
    }
}

impl FormatAttr {
    /// Parse the literal and the comma-separated arguments that follow it.
    fn parse(literal: &proc_macro2::Literal, rest: &[TokenTree]) -> Option<Self> {
        let source = literal.to_string();
        let value = syn::parse_str::<LitStr>(&source).ok()?.value();
        let literal_range = literal.span().byte_range();
        let body = body_offset(&source).map(|offset| literal_range.start + offset);
        let args = match rest {
            [] => Vec::new(),
            [TokenTree::Punct(comma), args @ ..] if comma.as_char() == ',' => {
                split_args(args, literal_range.end)
            }
            _ => return None,
        };
        Some(Self {
            value,
            literal: literal_range,
            body,
            args,
        })
    }

    /// Return every placeholder in the format string.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let attribute = ErrorAttr::parse("#[error(\"{source}\")]").unwrap();
    /// # let format = attribute.format().unwrap();
    /// let _ = format.placeholders();
    /// ```
    pub fn placeholders(&self) -> Vec<Placeholder> {
        placeholders(&self.value)
    }

    /// Map a byte range of the format string value to the attribute text.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let attribute = ErrorAttr::parse("#[error(\"{source}\")]").unwrap();
    /// # let format = attribute.format().unwrap();
    /// let _ = format.source_range(&(0..1));
    /// ```
    pub fn source_range(&self, range: &Range<usize>) -> Option<Range<usize>> {
        self.body.map(|body| body + range.start..body + range.end)
    }

    /// Return the unnamed arguments in order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let attribute = ErrorAttr::parse("#[error(\"{source}\", source)]").unwrap();
    /// # let format = attribute.format().unwrap();
    /// let _ = format.positional_args().count();
    /// ```
    pub fn positional_args(&self) -> impl Iterator<Item = &FormatArg> {
        self.args.iter().filter(|arg| arg.name.is_none())
    }
}

/// Return the offset of the string body when the source spells the value verbatim.
fn body_offset(source: &str) -> Option<usize> {
    // A cooked string with an escape does not map value bytes one to one.
    if let Some(body) = source.strip_prefix('"') {
        return (!body.contains('\\')).then_some(1);
    }
    let hashes = source
        .strip_prefix('r')?
        .bytes()
        .take_while(|&byte| byte == b'#')
        .count();
    Some(hashes + 2)
}

/// Split format arguments at top-level commas.
fn split_args(tokens: &[TokenTree], literal_end: usize) -> Vec<FormatArg> {
    let mut args = Vec::new();
    let mut previous_end = literal_end;
    for arg in
        tokens.split(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ','))
    {
        let (Some(first), Some(last)) = (arg.first(), arg.last()) else {
            continue;
        };
        let range = first.span().byte_range().start..last.span().byte_range().end;
        let name = match arg {
            [TokenTree::Ident(name), TokenTree::Punct(equals), _, ..]
                if equals.as_char() == '=' && equals.spacing() == Spacing::Alone =>
            {
                Some(name.to_string())
            }
            _ => None,
        };
        let ident = match arg {
            [TokenTree::Ident(ident)] => Some(ident.to_string()),
            _ => None,
        };
        args.push(FormatArg {
            name,
            ident,
            range: range.clone(),
            previous_end,
        });
        previous_end = range.end;
    }
    args
}

/// Scan a format string with thiserror's placeholder rules.
#[must_use]
///
/// # Examples
///
/// ```rust
/// let _ = placeholders("{source}");
/// ```
pub fn placeholders(value: &str) -> Vec<Placeholder> {
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(relative) = value.get(index..).and_then(|rest| rest.find('{')) {
        let start = index + relative;
        let rest = value.get(start + 1..).unwrap_or_default();
        // `{{` is an escaped brace, not a placeholder.
        if rest.starts_with('{') {
            index = start + 2;
            continue;
        }
        let Some(close) = rest.find('}') else {
            break;
        };
        let inner = rest.get(..close).unwrap_or_default();
        let (argument, spec) = split_argument(inner);
        found.push(Placeholder {
            range: start..start + close + 2,
            argument,
            spec: spec.to_owned(),
        });
        index = start + close + 2;
    }
    found
}

/// Split placeholder text into its argument and the remaining spec.
fn split_argument(inner: &str) -> (Argument, &str) {
    let digits = inner.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 {
        let (number, spec) = inner.split_at(digits);
        return number
            .parse()
            .map_or((Argument::Implicit, inner), |number| {
                (Argument::Index(number), spec)
            });
    }
    let raw = if inner.starts_with("r#") { 2 } else { 0 };
    let name_len = inner
        .get(raw..)
        .unwrap_or_default()
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        .count();
    if name_len == 0 || inner.as_bytes().get(raw).is_some_and(u8::is_ascii_digit) {
        return (Argument::Implicit, inner);
    }
    let (name, spec) = inner.split_at(raw + name_len);
    (Argument::Named(name.to_owned()), spec)
}

/// Return whether a placeholder spec formats with `Display`, as thiserror decides.
#[must_use]
///
/// # Examples
///
/// ```rust
/// assert!(!is_display_spec(":?"));
/// ```
pub fn is_display_spec(spec: &str) -> bool {
    !matches!(
        spec.chars().next_back(),
        Some('?' | 'o' | 'x' | 'X' | 'p' | 'b' | 'e' | 'E')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse an attribute that must be a format attribute.
    fn format(text: &str) -> FormatAttr {
        match ErrorAttr::parse(text).unwrap().kind {
            ErrorAttrKind::Format(format) => format,
            ErrorAttrKind::Transparent => panic!("transparent"),
        }
    }

    #[test]
    fn parses_transparent_attribute() {
        let attribute = ErrorAttr::parse("#[error(transparent)]").unwrap();
        assert!(matches!(attribute.kind, ErrorAttrKind::Transparent));
        assert!(attribute.format().is_none());
    }

    #[test]
    fn rejects_non_error_attributes() {
        assert!(ErrorAttr::parse("#[error(fmt = path)]").is_none());
        assert!(ErrorAttr::parse("#[error = \"x\"]").is_none());
        assert!(ErrorAttr::parse("#[source]").is_none());
    }

    #[test]
    fn rejects_invalid_error_shapes() {
        assert!(ErrorAttr::parse("#[error(\"x\" y)]").is_none());
        assert!(ErrorAttr::parse("#[error(1)]").is_none());
        assert!(ErrorAttr::parse("not an attribute").is_none());
    }

    #[test]
    fn splits_arguments() {
        let text = "#[error(\"{} {}\", .0, x = a == b, source,)]";
        let parsed = format(text);
        let args = parsed
            .args
            .iter()
            .map(|arg| {
                let source = text
                    .get(arg.range.clone())
                    .expect("format argument range must be valid");
                (arg.name.clone(), arg.ident.clone(), source)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            [
                (None, None, ".0"),
                (Some("x".to_owned()), None, "x = a == b"),
                (None, Some("source".to_owned()), "source"),
            ]
        );
        assert_eq!(parsed.positional_args().count(), 2);
        let third = parsed.args.get(2).expect("three arguments were parsed");
        let separator = text
            .get(third.previous_end..third.range.end)
            .expect("argument separator range must be valid");
        assert_eq!(separator, ", source");
    }

    #[test]
    fn maps_string_body_offsets() {
        assert_eq!(body_offset("\"a\""), Some(1));
        assert_eq!(body_offset("\"a\\n\""), None);
        assert_eq!(body_offset("r##\"a\"##"), Some(4));
    }

    #[test]
    fn rejects_non_string_body_offsets() {
        assert_eq!(body_offset("b\"a\""), None);
    }

    #[test]
    fn maps_placeholder_to_source() {
        let text = "#[error(r#\"{x}\"#)]";
        let parsed = format(text);
        let placeholders = parsed.placeholders();
        let placeholder = placeholders.first().expect("one placeholder was parsed");
        let range = parsed.source_range(&placeholder.range).unwrap();
        let source = text
            .get(range)
            .expect("placeholder source range must be valid");
        assert_eq!(source, "{x}");
    }

    #[test]
    fn rejects_escaped_source_mapping() {
        assert!(
            format("#[error(\"\\t{x}\")]")
                .source_range(&(0..1))
                .is_none()
        );
    }

    #[test]
    #[expect(
        clippy::literal_string_with_formatting_args,
        reason = "the input is a thiserror format string under test"
    )]
    fn scans_placeholders() {
        let found = placeholders("{{x}} {} {0:?} {r#type} {name:>8} {:x} {_} {0x} {-} {");
        let arguments = found
            .iter()
            .map(|p| (p.argument.clone(), p.spec.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            arguments,
            [
                (Argument::Implicit, ""),
                (Argument::Index(0), ":?"),
                (Argument::Named("r#type".to_owned()), ""),
                (Argument::Named("name".to_owned()), ":>8"),
                (Argument::Implicit, ":x"),
                (Argument::Named("_".to_owned()), ""),
                (Argument::Index(0), "x"),
                (Argument::Implicit, "-"),
            ]
        );
        assert_eq!(found[0].range, 6..8);
        assert_eq!(
            split_argument("99999999999999999999999"),
            (Argument::Implicit, "99999999999999999999999")
        );
    }

    #[test]
    fn recognizes_display_specs() {
        assert!(is_display_spec(""));
        assert!(is_display_spec(":"));
        assert!(is_display_spec(":>10"));
    }

    #[test]
    fn recognizes_non_display_specs() {
        assert!(!is_display_spec(":?"));
        assert!(!is_display_spec(":#x"));
    }
}
