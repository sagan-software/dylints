//! Machine-applicability classification from lint source and UI output.

use std::collections::BTreeSet;

use strum::Display;

/// Whether a lint can offer a fix that `cargo fix` and `--fix` apply automatically.
#[derive(Clone, Copy, Debug, Display, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Applicability {
    /// The lint never emits a `MachineApplicable` suggestion.
    NotMachineApplicable,
    /// The lint emits at least one `MachineApplicable` suggestion.
    MachineApplicable,
}

/// Source text that marks a machine-applicable suggestion.
const MACHINE_APPLICABLE: &str = "Applicability::MachineApplicable";

/// Token that starts a declarative macro definition.
const MACRO_RULES: &str = "macro_rules!";

/// Names of support-crate macros whose expansion emits a machine-applicable
/// suggestion.
///
/// Several lint families declare their lints through `macro_rules!` macros in a
/// sibling `support` crate, so the lint's own source never names an applicability.
#[derive(Debug, Default)]
pub(crate) struct SuggestingMacros(BTreeSet<String>);

impl From<&str> for SuggestingMacros {
    /// Collect every `macro_rules!` definition whose body names a
    /// machine-applicable suggestion.
    fn from(source: &str) -> Self {
        let names = source
            .match_indices(MACRO_RULES)
            .filter_map(|(start, _)| source.get(start + MACRO_RULES.len()..))
            .filter_map(suggesting_macro_name)
            .collect();
        Self(names)
    }
}

/// Return the macro's name when its brace-delimited body names a
/// machine-applicable suggestion.
fn suggesting_macro_name(definition: &str) -> Option<String> {
    let (name, body) = definition.split_once('{')?;
    let body = body.get(..closing_brace(body)).unwrap_or(body);
    body.contains(MACHINE_APPLICABLE)
        .then(|| name.trim().to_owned())
}

/// Find the brace that closes a body whose opening brace was already consumed.
///
/// Macro bodies keep braces balanced, so counting is enough. An unbalanced body
/// extends to the end of the source.
fn closing_brace(body: &str) -> usize {
    let mut depth = 1_usize;
    body.char_indices()
        .find_map(|(index, character)| {
            // Track nesting and stop at the brace that returns the depth to zero.
            match character {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            (depth == 0).then_some(index)
        })
        .unwrap_or(body.len())
}

impl SuggestingMacros {
    /// Classify one lint from its Rust source and its UI test output.
    ///
    /// A lint is machine applicable when its source, or a support macro it invokes,
    /// names `Applicability::MachineApplicable` and its UI output renders at least one
    /// suggestion. Requiring both excludes macro-declared lints whose variant never
    /// builds a replacement.
    pub(crate) fn applicability(&self, source: &str, ui_stderr: &str) -> Applicability {
        let has_suggestion_source =
            source.contains(MACHINE_APPLICABLE) || self.is_invoked_by(source);
        if has_suggestion_source && has_rendered_suggestion(ui_stderr) {
            Applicability::MachineApplicable
        } else {
            Applicability::NotMachineApplicable
        }
    }

    /// Report whether the source invokes any suggesting macro.
    fn is_invoked_by(&self, source: &str) -> bool {
        self.0
            .iter()
            .any(|name| source.contains(&format!("{name}!")))
    }
}

/// Report whether compiletest output contains a rendered code suggestion.
///
/// rustc renders a short suggestion inline after the primary carets
/// (`^^^ help: ...`) and a longer one as a diff with `+`, `-`, or `~` markers
/// after the line number. A plain `= help:` note is not a suggestion.
fn has_rendered_suggestion(stderr: &str) -> bool {
    stderr.lines().any(|line| {
        is_diff_line(line)
            || line
                .split_once('|')
                .is_some_and(|(_, body)| is_suggestion_gutter(body.trim()))
    })
}

/// Report whether a line is a numbered diff line of a multi-line suggestion.
fn is_diff_line(line: &str) -> bool {
    ["LL + ", "LL - ", "LL ~ "]
        .iter()
        .any(|marker| line.starts_with(marker))
}

/// Report whether a gutter body holds an inline suggestion or insertion markers.
fn is_suggestion_gutter(body: &str) -> bool {
    let is_inline = body.starts_with('^') && body.contains("^ help: ");
    let is_marker_line = !body.is_empty()
        && body
            .chars()
            .all(|character| matches!(character, '+' | '~' | ' '));
    is_inline || is_marker_line
}

#[cfg(test)]
mod tests {
    use super::{Applicability, SuggestingMacros, has_rendered_suggestion};

    /// Support source with one suggesting macro, one help-only macro, and a
    /// suggesting function.
    const SUPPORT: &str = "macro_rules! suggests { () => { fix(Applicability::MachineApplicable) }; }\n\
         macro_rules! only_helps { () => { help() }; }\n\
         fn helper() { let _ = Applicability::MachineApplicable; }\n";

    /// Inline suggestion output used where a rendered suggestion is required.
    const INLINE: &str = "   |     ^^^ help: remove it\n";

    /// Only macro bodies that name the applicability are collected.
    #[test]
    fn collects_only_suggesting_macros() {
        // The helper function and the help-only macro must not be collected.
        let macros = SuggestingMacros::from(SUPPORT);
        assert_eq!(macros.0.iter().collect::<Vec<_>>(), ["suggests"]);
    }

    /// Direct suggestions and suggesting macro invocations both count.
    #[test]
    fn classifies_sources() {
        // Hold the UI output fixed so only the source evidence varies.
        let macros = SuggestingMacros::from(SUPPORT);
        let classify = |source: &str| macros.applicability(source, INLINE);

        // A plain source and a help-only macro do not suggest.
        assert_eq!(
            classify("fn check() {}"),
            Applicability::NotMachineApplicable
        );
        assert_eq!(
            classify("support::only_helps! {}"),
            Applicability::NotMachineApplicable
        );
        // Invoking a suggesting macro counts as a suggestion source.
        assert_eq!(
            classify("support::suggests! {}"),
            Applicability::MachineApplicable
        );
    }

    /// A suggesting source without rendered UI output is not machine applicable.
    #[test]
    fn requires_rendered_output() {
        // Hold the source fixed so only the rendered output varies.
        let macros = SuggestingMacros::from(SUPPORT);
        let direct = "diag.span_suggestion(span, msg, fix, Applicability::MachineApplicable)";

        // Rendered output confirms the suggestion; a plain help note does not.
        assert_eq!(
            macros.applicability(direct, INLINE),
            Applicability::MachineApplicable
        );
        assert_eq!(
            macros.applicability(direct, "   = help: plain\n"),
            Applicability::NotMachineApplicable
        );
    }

    /// Inline, diff, and insertion-marker suggestions are recognized.
    #[test]
    fn recognizes_rendered_suggestions() {
        // Each rustc rendering style counts as a suggestion.
        let renderings = [
            "LL |     #[a]\n   |     ^^^^ help: remove it\n",
            "LL - #[serde(default)]\n",
            "LL ~     let value = x?;\n",
            "LL |     value.ok()?\n   |          +++++\n",
        ];
        // Every style must be recognized.
        assert!(
            renderings
                .iter()
                .all(|output| has_rendered_suggestion(output))
        );
    }

    /// Plain help notes and ordinary span labels are not suggestions.
    #[test]
    fn ignores_help_notes_and_labels() {
        // Help notes, caret labels, and empty gutters are not suggestions.
        assert!(!has_rendered_suggestion("   = help: use `Router::merge`\n"));
        assert!(!has_rendered_suggestion(
            "   |     ^^^^ the router is nested here\n"
        ));
        assert!(!has_rendered_suggestion("   |\n"));
    }

    /// An unbalanced macro body extends to the end of the source.
    #[test]
    fn unbalanced_macro_body_reaches_the_end() {
        // The body has no closing brace, so the rest of the source belongs to it.
        let macros =
            SuggestingMacros::from("macro_rules! open { () => { Applicability::MachineApplicable");
        assert_eq!(macros.0.iter().collect::<Vec<_>>(), ["open"]);
    }
}
