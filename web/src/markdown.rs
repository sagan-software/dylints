//! README Markdown rendering for lint documentation fragments.

use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html,
};

/// Render README Markdown into the lint documentation fragment.
///
/// Level-two README headings become level-three headings to match the lint list
/// styles. Fenced code keeps only its first info token, so `rust,ignore` renders
/// as `language-rust` for client-side highlighting.
pub(crate) fn render_markdown(source: &str) -> String {
    // Rewrite the event stream before HTML rendering so both edits stay local.
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let events = Parser::new_ext(source, options).map(adjust_event);
    let mut rendered = String::new();
    html::push_html(&mut rendered, events);
    rendered
}

/// Demote headings and normalize code-block languages; pass other events through.
fn adjust_event(event: Event<'_>) -> Event<'_> {
    match event {
        Event::Start(Tag::Heading {
            level,
            id,
            classes,
            attrs,
        }) => Event::Start(Tag::Heading {
            level: demote(level),
            id,
            classes,
            attrs,
        }),
        Event::End(TagEnd::Heading(level)) => Event::End(TagEnd::Heading(demote(level))),
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => Event::Start(Tag::CodeBlock(
            CodeBlockKind::Fenced(CowStr::from(first_info_token(&info).to_owned())),
        )),
        other @ (Event::Start(_)
        | Event::End(_)
        | Event::Text(_)
        | Event::Code(_)
        | Event::InlineMath(_)
        | Event::DisplayMath(_)
        | Event::Html(_)
        | Event::InlineHtml(_)
        | Event::FootnoteReference(_)
        | Event::SoftBreak
        | Event::HardBreak
        | Event::Rule
        | Event::TaskListMarker(_)) => other,
    }
}

/// Keep the language token of a fenced-code info string.
fn first_info_token(info: &str) -> &str {
    info.split(|character: char| character == ',' || character.is_whitespace())
        .next()
        .unwrap_or_default()
}

/// Lower a heading by one level, keeping level six unchanged.
const fn demote(level: HeadingLevel) -> HeadingLevel {
    match level {
        HeadingLevel::H1 => HeadingLevel::H2,
        HeadingLevel::H2 => HeadingLevel::H3,
        HeadingLevel::H3 => HeadingLevel::H4,
        HeadingLevel::H4 => HeadingLevel::H5,
        HeadingLevel::H5 | HeadingLevel::H6 => HeadingLevel::H6,
    }
}

#[cfg(test)]
mod tests {
    use super::render_markdown;

    /// Headings drop one level and fenced code keeps its first language token.
    #[test]
    fn demotes_headings_and_normalizes_languages() {
        // `rust,ignore` must highlight as plain Rust.
        let html = render_markdown("## Title\n\n```rust,ignore\nfn main() {}\n```\n");
        assert!(html.contains("<h3>Title</h3>"));
        assert!(html.contains("<code class=\"language-rust\">"));
    }

    /// Every heading level is demoted, and level six stays at six.
    #[test]
    fn demotes_every_level() {
        // Render one heading per level; level six cannot go lower.
        let html = render_markdown("# a\n### b\n#### c\n##### d\n###### e\n");

        // Every level must move down by one.
        let expected = [
            "<h2>a</h2>",
            "<h4>b</h4>",
            "<h5>c</h5>",
            "<h6>d</h6>",
            "<h6>e</h6>",
        ];
        assert!(expected.iter().all(|tag| html.contains(tag)), "{html}");
    }

    /// Indented code and other events pass through unchanged.
    #[test]
    fn passes_other_events_through() {
        // Indented code, inline markup, and rules are not rewritten.
        let html = render_markdown("    indented\n\nText with `code` and *emphasis*.\n\n---\n");
        assert!(html.contains("<pre><code>indented"), "{html}");
        assert!(html.contains("<hr />"), "{html}");
    }
}
