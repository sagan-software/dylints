//! Lint README parsing and structural validation.

use std::{
    ffi::OsStr,
    fmt,
    path::{Path, PathBuf},
};

use crate::error::SiteError;

/// A validated lint README: its identity and the Markdown body after the title.
#[derive(Debug)]
pub(crate) struct Readme {
    /// Identity from the title, equal to the crate directory name.
    pub(crate) id: LintId,
    /// Markdown body without the title line.
    pub(crate) body: String,
}

impl Readme {
    /// Parse and validate the README of the lint crate in `lint_directory`.
    pub(crate) fn parse(document: &str, lint_directory: &Path) -> Result<Self, SiteError> {
        // Validate the structure first, then bind the title to the directory name.
        let path = lint_directory.join("README.md");
        let (title, body) = split_title(document, &path)?;
        validate_sections(body, &path)?;
        let id = LintId::parse(title, &path)?;

        // A title that differs from the directory would break source links.
        validate_directory_name(&id, lint_directory, &path).map(|()| Self {
            id,
            body: body.to_owned(),
        })
    }
}

/// Stable lint identity taken from the README title and the crate directory.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct LintId(Box<str>);

impl LintId {
    /// Validate a README title as a stable lint identity.
    fn parse(value: &str, path: &Path) -> Result<Self, SiteError> {
        // Reject every spelling that cannot remain a stable file, display, and URL identity.
        if value.is_empty() || !value.bytes().all(is_identity_byte) {
            return Err(SiteError::InvalidLintId {
                path: path.to_path_buf(),
                value: value.to_owned(),
            });
        }
        Ok(Self(value.into()))
    }

    /// The rustc lint name, which spells crate-directory hyphens as underscores.
    pub(crate) fn lint_name(&self) -> String {
        self.0.replace('-', "_")
    }
}

/// Report whether a byte may appear in a lint identity.
const fn is_identity_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
}

impl fmt::Display for LintId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Required README section headings in display order.
const REQUIRED_SECTIONS: [&str; 5] = [
    "## What it does",
    "## Why is this bad?",
    "## Known problems",
    "## Example",
    "## Use instead",
];

/// Split the required first-line title from its Markdown body.
fn split_title<'source>(
    source: &'source str,
    path: &Path,
) -> Result<(&'source str, &'source str), SiteError> {
    // A title line is `# ` followed by a non-empty name.
    let title = source
        .split_once('\n')
        .and_then(|(title_line, body)| Some((title_line.strip_prefix("# ")?.trim(), body)))
        .filter(|(title, _)| !title.is_empty());

    // The body starts at the first non-blank line after the title.
    let (title, body) = title.ok_or_else(|| SiteError::MissingTitle {
        path: path.to_path_buf(),
    })?;
    Ok((title, body.trim_start()))
}

/// Enforce the same level-two section structure for every lint document.
fn validate_sections(source: &str, path: &Path) -> Result<(), SiteError> {
    // Track the last heading so duplicates cannot satisfy the required order.
    let lines: Vec<_> = source.lines().map(str::trim_end).collect();
    let mut previous_index = None;

    // Require every shared section once, in its declared sequence.
    for heading in REQUIRED_SECTIONS {
        let index = lines
            .iter()
            .position(|line| *line == heading)
            .filter(|index| previous_index.is_none_or(|previous| *index > previous));
        previous_index = Some(index.ok_or_else(|| SiteError::InvalidReadmeStructure {
            path: path.to_path_buf(),
            heading,
        })?);
    }
    Ok(())
}

/// Confirm the README title matches the lint crate directory exactly.
fn validate_directory_name(
    id: &LintId,
    lint_directory: &Path,
    readme: &Path,
) -> Result<(), SiteError> {
    // Compare the final path component with the validated identity.
    let directory = lint_directory.file_name().and_then(OsStr::to_str);
    if directory == Some(id.0.as_ref()) {
        return Ok(());
    }
    Err(SiteError::MismatchedLintId {
        path: readme.to_path_buf(),
        title: id.to_string(),
        directory: PathBuf::from(lint_directory),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{LintId, Readme};
    use crate::error::SiteError;

    /// README body that satisfies the shared section contract.
    const BODY: &str = "## What it does\n\nA.\n\n## Why is this bad?\n\nB.\n\n## Known problems\n\nC.\n\n## Example\n\nD.\n\n## Use instead\n\nE.\n";

    /// Parse a README for a lint crate named `name`.
    fn parse(document: &str, name: &str) -> Result<Readme, SiteError> {
        Readme::parse(document, &Path::new("lints/style").join(name))
    }

    /// A valid README yields its identity and body.
    #[test]
    fn valid_readme_parses() {
        // A hyphenated title becomes an underscored rustc name.
        let readme = parse(&format!("# serde-x\n\n{BODY}"), "serde-x");
        let readme = readme.expect("README should parse");
        assert_eq!(readme.id.lint_name(), "serde_x");
        assert!(readme.body.starts_with("## What it does"));
    }

    /// Missing titles, misordered sections, and directory mismatches are rejected.
    #[test]
    fn invalid_readmes_are_rejected() {
        // Each document breaks exactly one structural rule.
        let missing_title = parse(BODY, "x");
        let old_headings = parse("# x\n### What it does\n", "x");
        let mismatch = parse(&format!("# x\n\n{BODY}"), "y");

        // Each rule has its own error.
        assert!(matches!(missing_title, Err(SiteError::MissingTitle { .. })));
        assert!(matches!(
            old_headings,
            Err(SiteError::InvalidReadmeStructure { .. })
        ));
        assert!(matches!(mismatch, Err(SiteError::MismatchedLintId { .. })));
    }

    /// Titles reject characters that cannot be URL fragments.
    #[test]
    fn identity_rejects_invalid_characters() {
        // Uppercase and empty titles are rejected; the allowed alphabet parses.
        let parse_id = |value: &str| LintId::parse(value, Path::new("README.md"));
        assert!(matches!(
            parse_id("Bad_Name"),
            Err(SiteError::InvalidLintId { .. })
        ));
        assert!(matches!(parse_id(""), Err(SiteError::InvalidLintId { .. })));
        assert_eq!(
            parse_id("a-b_1").map(|id| id.to_string()).ok().as_deref(),
            Some("a-b_1")
        );
    }

    /// A title line without a body line is rejected.
    #[test]
    fn title_without_newline_is_rejected() {
        assert!(matches!(
            parse("# x", "x"),
            Err(SiteError::MissingTitle { .. })
        ));
    }
}
