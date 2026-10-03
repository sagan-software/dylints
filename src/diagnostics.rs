//! Cargo JSON parsing, machine-applicable fixes, and changed-line filtering.

use std::{
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use serde::Deserialize;
use thiserror::Error;

/// Changed inclusive line ranges keyed by repository-relative path.
pub(super) type ChangedRanges = BTreeMap<String, Vec<(u64, u64)>>;

/// A compiler diagnostic retained by the runner.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Diagnostic {
    /// Runner phase that emitted the diagnostic.
    tool: Box<str>,
    /// Compiler severity.
    level: Box<str>,
    /// Human-readable diagnostic text.
    message: Box<str>,
    /// Optional rustc or lint code.
    code: Box<str>,
    /// Primary repository-relative spans.
    spans: Vec<DiagnosticSpan>,
}

impl Diagnostic {
    /// Return the phase that emitted this diagnostic.
    pub(super) fn tool(&self) -> &str {
        &self.tool
    }

    /// Return the compiler severity.
    pub(super) fn level(&self) -> &str {
        &self.level
    }

    /// Return the human-readable diagnostic text.
    pub(super) fn message(&self) -> &str {
        &self.message
    }

    /// Return the stable compiler or lint code when one exists.
    pub(super) fn code(&self) -> &str {
        &self.code
    }

    /// Return whether rustc emitted a hard compiler error instead of a lint finding.
    pub(super) fn is_blocking_compiler_error(&self) -> bool {
        self.level.as_ref() == "error"
            && (self.code.is_empty()
                || self.code.strip_prefix('E').is_some_and(|digits| {
                    digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_digit())
                }))
    }

    /// Return the first source location that GitLab can annotate.
    pub(super) fn primary_location(&self) -> Option<(&str, u64)> {
        self.spans
            .first()
            .map(|span| (span.file.as_ref(), span.start))
    }
}

/// One source span attached to a compiler diagnostic.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DiagnosticSpan {
    /// Repository-relative source path.
    file: Box<str>,
    /// Inclusive first line.
    start: u64,
    /// Inclusive final line.
    end: u64,
}

/// One compiler-provided machine-applicable suggestion, applied all-or-nothing.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct MachineFix {
    /// Source replacements ordered by file and byte range.
    edits: Vec<SourceEdit>,
}

/// One byte-range replacement inside a repository file.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SourceEdit {
    /// Repository-relative source path.
    file: PathBuf,
    /// Half-open source byte range start.
    byte_start: usize,
    /// Exclusive source byte end.
    byte_end: usize,
    /// Replacement text supplied by rustc.
    replacement: Box<str>,
}

impl SourceEdit {
    /// Return whether two edits cannot both apply with one unambiguous result.
    ///
    /// Overlapping ranges conflict, and so does an insertion at the start of
    /// another edit, because their relative order would be arbitrary.
    fn has_conflict_with(&self, other: &Self) -> bool {
        self.file == other.file
            && ((self.byte_start < other.byte_end && other.byte_start < self.byte_end)
                || (self.byte_start == other.byte_start
                    && (self.byte_start == self.byte_end || other.byte_start == other.byte_end)))
    }
}

/// Summary of source replacements written during one fixer pass.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct AppliedFixes {
    /// Number of compiler suggestions applied.
    pub(super) suggestions: usize,
    /// Number of source files changed.
    pub(super) files: usize,
    /// Number of suggestions skipped because they overlap an applied suggestion.
    pub(super) deferred: usize,
}

/// Failures while deriving changed ranges or applying compiler fixes.
#[derive(Debug, Error)]
pub(super) enum DiagnosticError {
    /// Git could not be started.
    #[error("could not run Git for changed-range filtering")]
    StartGit(#[source] std::io::Error),
    /// Git rejected the revision range.
    #[error("Git could not resolve changed range `{range}`: {stderr}")]
    GitRange {
        /// Requested revision range.
        range: String,
        /// Git diagnostic output.
        stderr: String,
    },
    /// A source file needed by a machine-applicable suggestion could not be read.
    #[error("could not read source file for fixes `{}`", path.display())]
    ReadSource {
        /// Repository-relative source path.
        path: PathBuf,
        /// Underlying filesystem failure.
        source: std::io::Error,
    },
    /// A source file could not be replaced after its fixes were validated.
    #[error(
        "could not write source file after fixes `{}`",
        path.display()
    )]
    WriteSource {
        /// Repository-relative source path.
        path: PathBuf,
        /// Underlying filesystem failure.
        source: std::io::Error,
    },
    /// A machine-applicable suggestion does not match the current source.
    #[error(
        "could not apply fixes to `{}`: {source}",
        path.display()
    )]
    ApplyFixes {
        /// Repository-relative source path.
        path: PathBuf,
        /// Source-range or encoding failure.
        source: FixError,
    },
}

/// Failure validating a compiler-provided source replacement.
#[derive(Debug, Error)]
pub(super) enum FixError {
    /// The compiler supplied a range outside the current source bytes.
    #[error("invalid byte range {start}..{end} for source length {byte_length}")]
    InvalidRange {
        /// Replacement start byte.
        start: usize,
        /// Replacement end byte.
        end: usize,
        /// Current source length in bytes.
        byte_length: usize,
    },
    /// The compiler supplied a range that splits a UTF-8 code point.
    #[error("byte range is not aligned to UTF-8 character boundaries")]
    InvalidUtf8Boundary,
    /// The source file is not valid UTF-8.
    #[error("source is not valid UTF-8")]
    InvalidUtf8,
}

/// Cargo's top-level JSON message shape.
#[derive(Debug, Deserialize)]
struct CargoMessage {
    /// Cargo event discriminator.
    reason: Option<Box<str>>,
    /// Compiler diagnostic payload.
    message: Option<CompilerMessage>,
}

/// Minimal rustc diagnostic representation used for filtering.
#[derive(Debug, Deserialize)]
struct CompilerMessage {
    /// Compiler severity.
    level: Box<str>,
    /// Human-readable diagnostic text.
    message: Box<str>,
    /// Optional diagnostic code.
    code: Option<DiagnosticCode>,
    /// Source spans attached to the diagnostic.
    #[serde(default)]
    spans: Vec<CompilerSpan>,
    /// Child diagnostics that can also carry source suggestions.
    #[serde(default)]
    children: Vec<Self>,
}

/// Rustc diagnostic code wrapper.
#[derive(Debug, Deserialize)]
struct DiagnosticCode {
    /// Stable lint or compiler code.
    code: Box<str>,
}

/// Minimal rustc source-span representation.
#[derive(Debug, Deserialize)]
struct CompilerSpan {
    /// Source path emitted by rustc.
    file_name: Box<str>,
    /// Inclusive first line.
    line_start: u64,
    /// Inclusive final line.
    line_end: u64,
    /// Whether this is a primary diagnostic span.
    is_primary: bool,
    /// Half-open source byte start for an applicable replacement.
    byte_start: Option<usize>,
    /// Half-open source byte end for an applicable replacement.
    byte_end: Option<usize>,
    /// Replacement text suggested by rustc or a lint.
    suggested_replacement: Option<Box<str>>,
    /// rustc's safety classification for the replacement.
    suggestion_applicability: Option<Box<str>>,
}

/// Parse Cargo's line-oriented event stream into compiler messages.
fn compiler_messages(output: &str) -> impl Iterator<Item = CompilerMessage> {
    // Ignore unrelated tool chatter and Cargo events that carry no compiler diagnostic.
    output
        .lines()
        .filter_map(|line| serde_json::from_str::<CargoMessage>(line).ok())
        .filter(|item| item.reason.as_deref() == Some("compiler-message"))
        .filter_map(|item| item.message)
}

/// Parse all machine-applicable suggestions from Cargo compiler JSON.
pub(super) fn parse_machine_fixes(repo: &Path, output: &str) -> Vec<MachineFix> {
    // BTreeSet removes suggestions that Cargo repeats for several targets of one file.
    let mut fixes = BTreeSet::new();
    for message in compiler_messages(output) {
        collect_machine_fixes(repo, &message, &mut fixes);
    }
    fixes.into_iter().collect()
}

/// Collect safe suggestions from one compiler message and its child messages.
fn collect_machine_fixes(repo: &Path, message: &CompilerMessage, fixes: &mut BTreeSet<MachineFix>) {
    // Every machine-applicable span of one diagnostic forms one atomic suggestion.
    let edits = message
        .spans
        .iter()
        .filter(|span| span.suggestion_applicability.as_deref() == Some("MachineApplicable"))
        .map(|span| {
            Some(SourceEdit {
                file: safe_relative_file(repo, &span.file_name)?,
                byte_start: span.byte_start?,
                byte_end: span.byte_end?,
                replacement: span.suggested_replacement.clone()?,
            })
        })
        .collect::<Option<Vec<_>>>();
    // Drop the whole suggestion when any part lacks a range or leaves the repository.
    if let Some(mut edits) = edits.filter(|edits| !edits.is_empty()) {
        edits.sort();
        let _is_new = fixes.insert(MachineFix { edits });
    }
    // Walk child diagnostics because rustc nests suggestions there.
    for child in &message.children {
        collect_machine_fixes(repo, child, fixes);
    }
}

/// Apply every non-conflicting suggestion and defer the rest to the next fixer pass.
///
/// Each suggestion applies all-or-nothing. A suggestion that overlaps one
/// already accepted in this pass is deferred, because the next pass re-derives
/// it from the rewritten source. Every file is validated before any is written.
pub(super) fn apply_machine_fixes(
    repo: &Path,
    fixes: &[MachineFix],
) -> Result<AppliedFixes, DiagnosticError> {
    let mut sources = BTreeMap::<PathBuf, String>::new();
    let (accepted, mut applied) = collect_accepted_fixes(repo, fixes, &mut sources)?;
    // Apply each file's edits from the end so earlier byte offsets remain valid.
    applied.files = write_accepted_fixes(repo, sources, accepted)?;
    Ok(applied)
}

/// Validate suggestions and retain the non-overlapping edits for one fixer pass.
fn collect_accepted_fixes(
    repo: &Path,
    fixes: &[MachineFix],
    sources: &mut BTreeMap<PathBuf, String>,
) -> Result<(Vec<SourceEdit>, AppliedFixes), DiagnosticError> {
    let mut accepted = Vec::<SourceEdit>::new();
    let mut applied = AppliedFixes::default();
    // Validate every suggestion against the unmodified source before writing anything.
    for fix in fixes {
        let mut changes = Vec::new();
        for edit in &fix.edits {
            let source = load_source(repo, sources, &edit.file)?;
            validate_edit(source, edit).map_err(|source| DiagnosticError::ApplyFixes {
                path: edit.file.clone(),
                source,
            })?;
            // Skip parts whose current text already equals the replacement.
            if source.get(edit.byte_start..edit.byte_end) != Some(&*edit.replacement) {
                changes.push(edit);
            }
        }
        // An already-applied suggestion must not trigger another fixer pass.
        if changes.is_empty() {
            continue;
        }
        // Defer a suggestion that overlaps an accepted suggestion or one of its own parts.
        let is_conflicting = changes.iter().enumerate().any(|(index, edit)| {
            accepted
                .iter()
                .chain(changes.iter().skip(index + 1).copied())
                .any(|other| edit.has_conflict_with(other))
        });
        if is_conflicting {
            applied.deferred += 1;
        } else {
            accepted.extend(changes.into_iter().cloned());
            applied.suggestions += 1;
        }
    }

    Ok((accepted, applied))
}

/// Write accepted edits after every source has passed validation.
fn write_accepted_fixes(
    repo: &Path,
    sources: BTreeMap<PathBuf, String>,
    mut accepted: Vec<SourceEdit>,
) -> Result<usize, DiagnosticError> {
    // Count only files with at least one accepted edit.
    let mut files = 0;
    // Sort source edits before grouping them by file and applying them backwards.
    accepted.sort();
    for (file, mut source) in sources {
        // Leave files without accepted changes untouched.
        let edits = accepted
            .iter()
            .filter(|edit| edit.file == file)
            .collect::<Vec<_>>();
        if edits.is_empty() {
            continue;
        }
        // Reverse order keeps earlier byte offsets valid after each replacement.
        for edit in edits.into_iter().rev() {
            source.replace_range(edit.byte_start..edit.byte_end, &edit.replacement);
        }
        fs::write(repo.join(&file), source)
            .map_err(|source| DiagnosticError::WriteSource { path: file, source })?;
        files += 1;
    }
    Ok(files)
}

/// Return one cached source file, reading and decoding it on first use.
fn load_source<'source>(
    repo: &Path,
    sources: &'source mut BTreeMap<PathBuf, String>,
    file: &Path,
) -> Result<&'source str, DiagnosticError> {
    // Read each file once so every suggestion validates against the same original text.
    match sources.entry(file.to_owned()) {
        Entry::Occupied(entry) => Ok(entry.into_mut()),
        Entry::Vacant(entry) => {
            let bytes =
                fs::read(repo.join(file)).map_err(|source| DiagnosticError::ReadSource {
                    path: file.to_owned(),
                    source,
                })?;
            let text = String::from_utf8(bytes).map_err(|_error| DiagnosticError::ApplyFixes {
                path: file.to_owned(),
                source: FixError::InvalidUtf8,
            })?;
            Ok(entry.insert(text))
        }
    }
}

/// Validate one compiler-provided byte range and its UTF-8 boundaries.
const fn validate_edit(source: &str, edit: &SourceEdit) -> Result<(), FixError> {
    // Reject ranges outside the current source before checking UTF-8 boundaries.
    if edit.byte_start > edit.byte_end || edit.byte_end > source.len() {
        return Err(FixError::InvalidRange {
            start: edit.byte_start,
            end: edit.byte_end,
            byte_length: source.len(),
        });
    }
    if source.is_char_boundary(edit.byte_start) && source.is_char_boundary(edit.byte_end) {
        Ok(())
    } else {
        Err(FixError::InvalidUtf8Boundary)
    }
}

/// Return a compiler path only when it is inside the target repository.
fn safe_relative_file(repo: &Path, file: &str) -> Option<PathBuf> {
    // Normalize compiler paths while preserving repository-relative diagnostics.
    let path = Path::new(file);
    let relative = if path.is_absolute() {
        path.strip_prefix(repo).ok()?.to_path_buf()
    } else {
        PathBuf::from(file.strip_prefix("./").unwrap_or(file))
    };
    // Refuse traversal and platform-root components before the caller joins the path.
    relative
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
        .then_some(relative)
}

/// Ask Git for zero-context hunks in the revision range.
pub(super) fn changed_ranges(repo: &Path, range: &str) -> Result<ChangedRanges, DiagnosticError> {
    // Pin the output format so user diff configuration cannot change prefixes or add color.
    let output = Command::new("git")
        .current_dir(repo)
        .args([
            "-c",
            "core.quotePath=false",
            "diff",
            "--unified=0",
            "--no-color",
            "--no-ext-diff",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "--relative",
            range,
            "--",
            "*.rs",
            "Cargo.toml",
            "*/Cargo.toml",
        ])
        .output()
        .map_err(DiagnosticError::StartGit)?;
    // Preserve Git's diagnostic when the revision range is invalid.
    if !output.status.success() {
        return Err(DiagnosticError::GitRange {
            range: range.to_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(parse_changed_ranges(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

/// Parse new-file hunk coordinates from zero-context unified diff output.
fn parse_changed_ranges(diff: &str) -> ChangedRanges {
    let mut result = ChangedRanges::new();
    let mut current_file: Option<&str> = None;
    // Associate each hunk with its preceding new-file header; deleted files have none.
    for line in diff.lines() {
        if let Some(header) = line.strip_prefix("+++ ") {
            // Git appends a tab to a header whose path contains a space.
            let header = header.strip_suffix('\t').unwrap_or(header);
            current_file = header.strip_prefix("b/");
            continue;
        }
        // Ignore metadata until a retained destination file is active.
        let (Some(file), Some((start, count))) = (current_file, new_hunk_range(line)) else {
            continue;
        };
        // A pure deletion has no new lines to retain.
        if count > 0 {
            result
                .entry(file.to_owned())
                .or_default()
                .push((start, start + count - 1));
        }
    }
    result
}

/// Parse the `+start,count` portion of a unified diff hunk header.
fn new_hunk_range(line: &str) -> Option<(u64, u64)> {
    // Parse only the new-file coordinate and default an omitted count to one.
    let coordinates = line
        .strip_prefix("@@ -")
        .and_then(|header| header.split_once(" +"))
        .and_then(|(_, new_and_rest)| new_and_rest.split_ascii_whitespace().next())?;
    let (start, count) = coordinates.split_once(',').unwrap_or((coordinates, "1"));
    Some((start.parse().ok()?, count.parse().ok()?))
}

/// Project unique warning and error diagnostics from Cargo JSON output.
pub(super) fn diagnostics_from_cargo_output(
    tool: &str,
    repo: &Path,
    output: &str,
) -> Vec<Diagnostic> {
    // Cargo repeats a diagnostic for each target that compiles the same source.
    let mut seen = BTreeSet::new();
    compiler_messages(output)
        .filter(|message| matches!(message.level.as_ref(), "warning" | "error"))
        .map(|message| {
            // Prefer primary spans, but retain all spans when Cargo marks none as primary.
            let primary: Vec<_> = message
                .spans
                .iter()
                .filter(|span| span.is_primary)
                .collect();
            let spans = if primary.is_empty() {
                message.spans.iter().collect()
            } else {
                primary
            };
            // Convert Cargo's transport model into the runner's stable diagnostic model.
            Diagnostic {
                tool: tool.into(),
                level: message.level,
                message: message.message,
                code: message.code.map_or_else(|| "".into(), |code| code.code),
                spans: spans
                    .into_iter()
                    .map(|span| DiagnosticSpan {
                        file: relative_file(repo, &span.file_name).into(),
                        start: span.line_start,
                        end: span.line_end,
                    })
                    .collect(),
            }
        })
        .filter(|diagnostic| seen.insert(diagnostic.clone()))
        .collect()
}

/// Retain diagnostics whose primary span overlaps a changed line.
pub(super) fn filter_diagnostics(
    ranges: &ChangedRanges,
    diagnostics: &[Diagnostic],
) -> Vec<Diagnostic> {
    // Keep spanless compiler errors and diagnostics that intersect an inclusive changed range.
    diagnostics
        .iter()
        .filter(|diagnostic| {
            (diagnostic.level.as_ref() == "error" && diagnostic.spans.is_empty())
                || diagnostic.spans.iter().any(|span| {
                    ranges.get(span.file.as_ref()).is_some_and(|changed| {
                        changed
                            .iter()
                            .any(|(start, end)| span.start <= *end && span.end >= *start)
                    })
                })
        })
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Format one retained diagnostic for concise terminal output.
pub(super) fn format_diagnostic(diagnostic: &Diagnostic) -> String {
    // Prefer the first retained span while preserving spanless compiler failures.
    let location = diagnostic.spans.first().map_or_else(
        || "<no span>".to_owned(),
        |span| format!("{}:{}", span.file, span.start),
    );
    // Omit empty diagnostic codes so output contains no empty brackets.
    let code = if diagnostic.code.is_empty() {
        String::new()
    } else {
        format!(" [{}]", diagnostic.code)
    };
    // Keep tool, severity, location, code, and message in one stable line.
    format!(
        "{}: {location}: {}{code}: {}",
        diagnostic.tool, diagnostic.level, diagnostic.message
    )
}

/// Normalize an emitted span path relative to the target repository.
fn relative_file(repo: &Path, file: &str) -> String {
    // Strip the repository root only from absolute paths that are actually inside it.
    let path = Path::new(file);
    if path.is_absolute() {
        path.strip_prefix(repo)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    } else {
        file.strip_prefix("./").unwrap_or(file).to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::PermissionsExt as _,
        path::{Path, PathBuf},
    };

    use super::{
        AppliedFixes, DiagnosticError, FixError, MachineFix, SourceEdit, apply_machine_fixes,
        diagnostics_from_cargo_output, filter_diagnostics, format_diagnostic, new_hunk_range,
        parse_changed_ranges, parse_machine_fixes,
    };

    /// Build one edit in `src/lib.rs`.
    fn edit(byte_start: usize, byte_end: usize, replacement: &str) -> SourceEdit {
        file_edit("src/lib.rs", byte_start, byte_end, replacement)
    }

    /// Build one edit in a named repository file.
    fn file_edit(file: &str, byte_start: usize, byte_end: usize, replacement: &str) -> SourceEdit {
        SourceEdit {
            file: PathBuf::from(file),
            byte_start,
            byte_end,
            replacement: replacement.into(),
        }
    }

    /// Write `src/lib.rs` into a fresh repository and return the repository.
    fn repository(source: &[u8]) -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        fs::create_dir_all(repo.path().join("src")).unwrap();
        fs::write(repo.path().join("src/lib.rs"), source).unwrap();
        repo
    }

    /// Apply suggestions to a fresh `src/lib.rs` and return the summary and new source.
    fn apply(source: &str, fixes: &[MachineFix]) -> (AppliedFixes, String) {
        let repo = repository(source.as_bytes());
        let applied = apply_machine_fixes(repo.path(), fixes).unwrap();
        (
            applied,
            fs::read_to_string(repo.path().join("src/lib.rs")).unwrap(),
        )
    }

    /// Wrap one rustc message in Cargo's compiler-message event.
    fn cargo_message(message: &str) -> String {
        format!(r#"{{"reason":"compiler-message","message":{message}}}"#)
    }

    /// Build one rustc span with an optional suggestion.
    fn span(file: &str, line: u64, is_primary: bool, suggestion: &str) -> String {
        format!(
            r#"{{"file_name":"{file}","line_start":{line},"line_end":{line},"is_primary":{is_primary}{suggestion}}}"#
        )
    }

    /// Build the suggestion fields of one span.
    fn suggestion(byte_start: usize, byte_end: usize, text: &str, applicability: &str) -> String {
        format!(
            r#","byte_start":{byte_start},"byte_end":{byte_end},"suggested_replacement":"{text}","suggestion_applicability":"{applicability}""#
        )
    }

    /// One diagnostic's machine-applicable spans form one suggestion; other spans are ignored.
    #[test]
    fn groups_machine_applicable_spans_by_diagnostic() {
        // Build parent and nested compiler messages with duplicate Cargo output.
        let parent_span = span(
            "/repo/src/lib.rs",
            1,
            true,
            &suggestion(0, 1, "A", "MachineApplicable"),
        );
        let multipart = [
            span(
                "src/lib.rs",
                1,
                true,
                &suggestion(5, 6, "F", "MachineApplicable"),
            ),
            span(
                "./src/lib.rs",
                1,
                true,
                &suggestion(3, 4, "D", "MachineApplicable"),
            ),
        ]
        .join(",");
        let maybe = span(
            "src/lib.rs",
            1,
            true,
            &suggestion(1, 2, "X", "MaybeIncorrect"),
        );
        let message = cargo_message(&format!(
            r#"{{"level":"warning","message":"m","spans":[{parent_span}],"children":[{{"level":"help","message":"h","spans":[{multipart}]}},{{"level":"help","message":"h","spans":[{maybe}]}}]}}"#
        ));
        // Cargo repeats messages per target, and unrelated lines are not compiler JSON.
        let output = format!("not json\n{{\"reason\":\"build-finished\"}}\n{message}\n{message}\n");

        assert_eq!(
            parse_machine_fixes(Path::new("/repo"), &output),
            vec![
                MachineFix {
                    edits: vec![edit(0, 1, "A")],
                },
                MachineFix {
                    edits: vec![edit(3, 4, "D"), edit(5, 6, "F")],
                },
            ]
        );
    }

    /// A suggestion with any unusable part is dropped as a whole.
    #[test]
    fn drops_suggestions_with_unusable_parts() {
        let valid = span(
            "src/lib.rs",
            1,
            true,
            &suggestion(0, 1, "A", "MachineApplicable"),
        );
        let unusable = [
            span(
                "/elsewhere/src/lib.rs",
                1,
                true,
                &suggestion(2, 3, "B", "MachineApplicable"),
            ),
            span(
                "../outside.rs",
                1,
                true,
                &suggestion(2, 3, "B", "MachineApplicable"),
            ),
            span(
                "src/lib.rs",
                1,
                true,
                r#","suggested_replacement":"B","suggestion_applicability":"MachineApplicable""#,
            ),
        ];
        let output = unusable
            .iter()
            .map(|part| {
                cargo_message(&format!(
                    r#"{{"level":"help","message":"h","spans":[{valid},{part}]}}"#
                ))
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(parse_machine_fixes(Path::new("/repo"), &output), Vec::new());
    }

    /// Non-overlapping suggestions apply from the end across several files.
    #[test]
    fn applies_non_overlapping_suggestions() {
        // Include adjacent edits and a second file to verify grouping boundaries.
        let repo = repository(b"abcdef");
        fs::write(repo.path().join("src/other.rs"), "xyz").unwrap();
        let fixes = [
            MachineFix {
                edits: vec![edit(0, 1, "A"), edit(3, 5, "DE")],
            },
            // An edit ending where another starts is unambiguous.
            MachineFix {
                edits: vec![edit(1, 3, "BC")],
            },
            MachineFix {
                edits: vec![file_edit("src/other.rs", 3, 3, "!")],
            },
        ];

        let applied = apply_machine_fixes(repo.path(), &fixes).unwrap();

        assert_eq!(
            applied,
            AppliedFixes {
                suggestions: 3,
                files: 2,
                deferred: 0,
            }
        );
        assert_eq!(
            fs::read_to_string(repo.path().join("src/lib.rs")).unwrap(),
            "ABCDEf"
        );
        assert_eq!(
            fs::read_to_string(repo.path().join("src/other.rs")).unwrap(),
            "xyz!"
        );
    }

    /// Overlapping suggestions wait for the next pass instead of aborting the run.
    #[test]
    fn defers_conflicting_suggestions() {
        let fixes = [
            MachineFix {
                edits: vec![edit(1, 4, "X")],
            },
            // Overlaps the accepted replacement.
            MachineFix {
                edits: vec![edit(3, 5, "Y")],
            },
            // Inserts where the accepted replacement starts.
            MachineFix {
                edits: vec![edit(1, 1, "Z")],
            },
            // Inserts twice at one point within itself.
            MachineFix {
                edits: vec![edit(5, 5, "P"), edit(5, 5, "Q")],
            },
        ];

        assert_eq!(
            apply("abcdef", &fixes),
            (
                AppliedFixes {
                    suggestions: 1,
                    files: 1,
                    deferred: 3,
                },
                "aXef".to_owned()
            )
        );
    }

    /// A suggestion that is already present does not start another fixer pass.
    #[test]
    fn ignores_applied_suggestions() {
        let fixes = [MachineFix {
            edits: vec![edit(1, 3, "bc")],
        }];

        assert_eq!(
            apply("abcdef", &fixes),
            (AppliedFixes::default(), "abcdef".to_owned())
        );
    }

    /// Ranges outside the source or inside a code point fail without writing.
    #[test]
    fn rejects_invalid_ranges() {
        let repo = repository("é".as_bytes());
        // Exercise bounds, oversized ranges, and UTF-8 alignment independently.
        let cases = [
            (
                edit(1, 0, ""),
                "invalid byte range 1..0 for source length 2",
            ),
            (
                edit(0, 3, ""),
                "invalid byte range 0..3 for source length 2",
            ),
            (
                edit(1, 2, ""),
                "byte range is not aligned to UTF-8 character boundaries",
            ),
        ];
        assert_invalid_ranges(repo.path(), &cases);
        assert_eq!(
            fs::read(repo.path().join("src/lib.rs")).unwrap(),
            "é".as_bytes()
        );
    }

    /// Check each invalid range and verify that the source remains unchanged.
    fn assert_invalid_ranges(repo: &Path, cases: &[(SourceEdit, &str)]) {
        // Every invalid suggestion must fail before the write stage.
        for (invalid, message) in cases {
            let error = apply_machine_fixes(
                repo,
                &[MachineFix {
                    edits: vec![invalid.clone()],
                }],
            )
            .unwrap_err();
            assert!(matches!(error, DiagnosticError::ApplyFixes { .. }));
            assert_eq!(
                error.to_string(),
                format!("could not apply fixes to `src/lib.rs`: {message}")
            );
        }
    }

    /// Unreadable, undecodable, and unwritable sources report their path.
    #[test]
    fn reports_source_file_failures() {
        // Use separate repositories so read, decode, and write failures stay distinguishable.
        let fix = [MachineFix {
            edits: vec![edit(0, 0, "x")],
        }];
        let missing = tempfile::tempdir().unwrap();
        let binary = repository(&[0xff]);
        let read_only = repository(b"abc");
        let path = read_only.path().join("src/lib.rs");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();

        // Read and decode failures must report their distinct error categories.
        assert!(matches!(
            apply_machine_fixes(missing.path(), &fix),
            Err(DiagnosticError::ReadSource { .. })
        ));
        assert!(matches!(
            apply_machine_fixes(binary.path(), &fix),
            Err(DiagnosticError::ApplyFixes {
                source: FixError::InvalidUtf8,
                ..
            })
        ));
        assert_eq!(
            apply_machine_fixes(read_only.path(), &fix)
                .unwrap_err()
                .to_string(),
            "could not write source file after fixes `src/lib.rs`"
        );
    }

    /// Unified diff output yields inclusive new-file ranges for retained files only.
    #[test]
    fn parses_changed_ranges() {
        // Retain only additions from tracked files and preserve paths with spaces.
        let diff = "@@ -1 +1 @@\n\
diff --git a/src/lib.rs b/src/lib.rs\n\
--- a/src/lib.rs\n\
+++ b/src/lib.rs\n\
@@ -4,2 +9,3 @@ fn context()\n\
-old\n\
+new\n\
@@ -20,2 +21,0 @@\n\
+++ /dev/null\n\
@@ -1,3 +0,0 @@\n\
+++ b/crates/a b/Cargo.toml\t\n\
@@ -2 +2 @@\n";

        let ranges = parse_changed_ranges(diff);

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges.get("src/lib.rs"), Some(&vec![(9, 11)]));
        assert_eq!(ranges.get("crates/a b/Cargo.toml"), Some(&vec![(2, 2)]));
    }

    /// Unified hunk coordinates retain their inclusive start and count.
    #[test]
    fn parses_explicit_new_hunk_range() {
        assert_eq!(new_hunk_range("@@ -4,2 +9,3 @@"), Some((9, 3)));
    }

    /// Unified hunk coordinates default a missing new-file count to one.
    #[test]
    fn parses_default_new_hunk_count() {
        assert_eq!(new_hunk_range("@@ -1 +7 @@"), Some((7, 1)));
    }

    /// Unified hunk coordinates reject text outside the hunk grammar.
    #[test]
    fn rejects_non_hunk_text() {
        assert_eq!(new_hunk_range("not a hunk"), None);
    }

    /// Unified hunk coordinates reject nonnumeric new-file locations.
    #[test]
    fn rejects_invalid_new_hunk_location() {
        assert_eq!(new_hunk_range("@@ -1 +x @@"), None);
    }

    /// Diagnostics are unique, warning-or-error, and repository-relative.
    #[test]
    fn projects_unique_diagnostics() {
        // Build duplicate warning and secondary-only compiler diagnostics.
        let warning = cargo_message(&format!(
            r#"{{"level":"warning","message":"w","code":{{"code":"clippy::unwrap_used"}},"spans":[{},{}]}}"#,
            span("/repo/src/lib.rs", 3, true, ""),
            span("/repo/src/lib.rs", 9, false, ""),
        ));
        let secondary_only = cargo_message(&format!(
            r#"{{"level":"error","message":"e","code":{{"code":"E0425"}},"spans":[{}]}}"#,
            span("/elsewhere/lib.rs", 4, false, ""),
        ));
        let note = cargo_message(r#"{"level":"note","message":"n","spans":[]}"#);
        let output = format!("{warning}\n{warning}\n{secondary_only}\n{note}\n");

        // Duplicate messages collapse before stable formatting and classification.
        let diagnostics = diagnostics_from_cargo_output("clippy", Path::new("/repo"), &output);

        // Collect the observable report facets into one structured assertion.
        let formatted = diagnostics
            .iter()
            .map(format_diagnostic)
            .collect::<Vec<_>>();
        let first_location = diagnostics[0].primary_location();
        let is_first_blocking = diagnostics[0].is_blocking_compiler_error();
        let is_second_blocking = diagnostics[1].is_blocking_compiler_error();
        assert_eq!(
            (
                formatted,
                first_location,
                is_first_blocking,
                is_second_blocking
            ),
            (
                vec![
                    "clippy: src/lib.rs:3: warning [clippy::unwrap_used]: w".to_owned(),
                    "clippy: /elsewhere/lib.rs:4: error [E0425]: e".to_owned(),
                ],
                Some(("src/lib.rs", 3)),
                false,
                true,
            )
        );
    }

    /// Only uncoded errors and `E` codes block a changed-range run.
    #[test]
    fn classifies_blocking_compiler_errors() {
        let output = [
            r#"{"level":"error","message":"a","code":null,"spans":[]}"#,
            r#"{"level":"error","message":"b","code":{"code":"E012"},"spans":[]}"#,
            r#"{"level":"error","message":"c","code":{"code":"E01x5"},"spans":[]}"#,
            r#"{"level":"error","message":"d","code":{"code":"string_error_result"},"spans":[]}"#,
        ]
        .map(cargo_message)
        .join("\n");

        let diagnostics = diagnostics_from_cargo_output("dylint", Path::new("/repo"), &output);

        assert_eq!(
            diagnostics
                .iter()
                .map(super::Diagnostic::is_blocking_compiler_error)
                .collect::<Vec<_>>(),
            [true, false, false, false]
        );
        assert_eq!(
            format_diagnostic(&diagnostics[0]),
            "dylint: <no span>: error: a"
        );
    }

    /// Changed-range filtering keeps intersecting spans and spanless errors.
    #[test]
    fn filters_diagnostics_to_changed_lines() {
        // Keep spanless errors while selecting only intersecting source lines.
        let output = [
            format!(
                r#"{{"level":"warning","message":"inside","spans":[{}]}}"#,
                span("src/lib.rs", 5, true, "")
            ),
            format!(
                r#"{{"level":"warning","message":"outside","spans":[{}]}}"#,
                span("src/lib.rs", 9, true, "")
            ),
            format!(
                r#"{{"level":"warning","message":"unchanged file","spans":[{}]}}"#,
                span("src/main.rs", 5, true, "")
            ),
            r#"{"level":"error","message":"spanless","spans":[]}"#.to_owned(),
            r#"{"level":"warning","message":"spanless warning","spans":[]}"#.to_owned(),
        ]
        .map(|message| cargo_message(&message))
        .join("\n");
        let diagnostics = diagnostics_from_cargo_output("dylint", Path::new("/repo"), &output);
        let ranges = [("src/lib.rs".to_owned(), vec![(1, 2), (4, 6)])].into();

        // The changed-file filter must preserve diagnostics in stable source order.
        let selected = filter_diagnostics(&ranges, &diagnostics);

        assert_eq!(
            selected.iter().map(format_diagnostic).collect::<Vec<_>>(),
            [
                "dylint: <no span>: error: spanless",
                "dylint: src/lib.rs:5: warning: inside",
            ]
        );
    }
}
