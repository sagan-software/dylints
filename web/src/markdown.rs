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
    let mut rust_code = None;
    let events = Parser::new_ext(source, options).map(|event| {
        // Hide compiler-only setup within Rust code, then reset at the block boundary.
        match &event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                rust_code = (code_language(info) == "rust").then_some(());
            }
            Event::End(TagEnd::CodeBlock) => rust_code = None,
            _ => {}
        }
        match event {
            Event::Text(text) if rust_code.is_some() => {
                Event::Text(visible_rust_lines(&text).into())
            }
            other => adjust_event(other),
        }
    });
    let mut rendered = String::new();
    html::push_html(&mut rendered, events);
    rendered
}

/// Apply rustdoc's hidden-line and escaped-hash rules to Rust example text.
/// <https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html>
fn visible_rust_lines(source: &str) -> String {
    // Preserve visible line endings while omitting compiler-only setup lines.
    let mut visible = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        // An escaped hash is visible; setup lines disappear without adding blank lines.
        if line.starts_with("##") {
            visible.push_str(&line[1..]);
        } else if !line.starts_with("# ") && line.trim_end() != "#" {
            visible.push_str(line);
        }
    }
    visible
}

/// Resolve Rust execution flags and explicit code languages for catalog highlighting.
fn code_language(info: &str) -> &str {
    let language = info
        .split(|character: char| character == ',' || character.is_whitespace())
        .next()
        .unwrap_or_default();
    match language {
        "no_run" | "ignore" | "compile_fail" => "rust",
        other => other,
    }
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
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
            // Keep only the language token for client-side syntax highlighting.
            let language = code_language(&info);
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from(
                language.to_owned(),
            ))))
        }
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

    /// Rustdoc setup lines are hidden while escaped hashes remain visible.
    #[test]
    fn renders_rustdoc_hidden_lines() {
        // Rustdoc compiles hidden imports and displays an escaped leading hash.
        let html = render_markdown(
            "```rust\n# use std::fmt;\n#\n## visible\nprintln!(\"example\");\n```\n",
        );
        // Visible code keeps its line endings after setup lines disappear.
        assert_eq!(
            html,
            "<pre><code class=\"language-rust\"># visible\nprintln!(\"example\");\n</code></pre>\n"
        );
    }

    /// Rustdoc execution flags without a language still describe Rust code.
    #[test]
    fn recognizes_rustdoc_execution_flags() {
        // Each rustdoc execution flag implies Rust unless another language is named.
        let rendered = ["no_run", "ignore", "compile_fail"]
            .map(|flag| render_markdown(&format!("```{flag}\n# hidden\nvisible\n```\n")));
        // All execution profiles hide setup and retain Rust highlighting.
        assert!(
            rendered
                .iter()
                .all(|html| html == "<pre><code class=\"language-rust\">visible\n</code></pre>\n")
        );
    }

    /// Hashes in other code blocks and ordinary Markdown remain source content.
    #[test]
    fn preserves_non_rust_hash_lines() {
        // A later non-Rust block must not inherit the preceding Rust block's mode.
        let html = render_markdown(
            "```rust\n# hidden\n```\n```sh\n# shell comment\n```\n\n# Heading\n\n    # indented\n",
        );
        // Only Rust setup lines disappear; other hashes keep their own semantics.
        assert!(html.contains("# shell comment"));
        assert!(html.contains("<h2>Heading</h2>"));
        assert!(html.contains("# indented"));
    }
}
