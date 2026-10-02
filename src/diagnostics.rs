//! Cargo JSON parsing and changed-line filtering.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
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

/// One compiler-provided machine-applicable source replacement.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct MachineFix {
    /// Repository-relative source path.
    file: PathBuf,
    /// Half-open source byte range start.
    byte_start: usize,
    /// Exclusive source byte end.
    byte_end: usize,
    /// UTF-8 replacement bytes supplied by rustc.
    replacement: Vec<u8>,
}

/// Summary of source replacements written during one fixer pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AppliedFixes {
    /// Number of compiler suggestions applied.
    pub(super) suggestions: usize,
    /// Number of source files changed.
    pub(super) files: usize,
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
    /// A machine-applicable suggestion could not be applied safely.
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
    /// Two suggestions overlap or insert at the same source position.
    #[error("machine-applicable suggestions overlap")]
    Overlapping,
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

/// Parse all machine-applicable replacements from Cargo compiler JSON.
pub(super) fn parse_machine_fixes(repo: &Path, output: &str) -> Vec<MachineFix> {
    // Parse Cargo's line-oriented event stream without allowing unrelated tool chatter to fail a run.
    let mut fixes = BTreeSet::new();
    for line in output.lines() {
        let Ok(item) = serde_json::from_str::<CargoMessage>(line) else {
            continue;
        };
        if item.reason.as_deref() != Some("compiler-message") {
            continue;
        }
        // Child diagnostics can carry suggestions even when the parent has none.
        item.message
            .iter()
            .for_each(|message| collect_machine_fixes(repo, message, &mut fixes));
    }
    // BTreeSet removes duplicate spans before the runner groups them by file.
    fixes.into_iter().collect()
}

/// Collect safe replacements from one compiler message and its child messages.
fn collect_machine_fixes(repo: &Path, message: &CompilerMessage, fixes: &mut BTreeSet<MachineFix>) {
    // Keep only replacements that rustc says are safe to apply without review.
    for span in &message.spans {
        let Some(replacement) = span.suggested_replacement.as_deref() else {
            continue;
        };
        if span.suggestion_applicability.as_deref() != Some("MachineApplicable") {
            continue;
        }
        let (Some(byte_start), Some(byte_end)) = (span.byte_start, span.byte_end) else {
            continue;
        };
        let Some(file) = safe_relative_file(repo, &span.file_name) else {
            continue;
        };
        // Deduplicate repeated parent and child representations of one suggestion.
        let _is_new = fixes.insert(MachineFix {
            file,
            byte_start,
            byte_end,
            replacement: replacement.as_bytes().to_vec(),
        });
    }
    // Walk child diagnostics because rustc nests secondary suggestions there.
    message
        .children
        .iter()
        .for_each(|child| collect_machine_fixes(repo, child, fixes));
}

/// Apply validated replacements to one source buffer from the end backwards.
fn apply_fixes_to_source(source: &mut Vec<u8>, fixes: &[MachineFix]) -> Result<usize, FixError> {
    let source_text = std::str::from_utf8(source).map_err(|_error| FixError::InvalidUtf8)?;
    // Validate every range before mutating the shared source buffer.
    let applicable = applicable_fixes(source, source_text, fixes)?;
    // Reject ambiguous edits before applying any replacement.
    ensure_non_overlapping(&applicable)?;
    // Apply from the end so earlier byte offsets remain stable.
    for fix in applicable.iter().rev() {
        let _spliced = source.splice(
            fix.byte_start..fix.byte_end,
            fix.replacement.iter().copied(),
        );
    }
    Ok(applicable.len())
}

/// Return validated replacements whose current source differs from their output.
fn applicable_fixes(
    source: &[u8],
    source_text: &str,
    fixes: &[MachineFix],
) -> Result<Vec<MachineFix>, FixError> {
    // Sort spans before validating overlap and before applying reverse-order edits.
    let mut ordered = fixes.to_vec();
    ordered.sort_by_key(|fix| (fix.byte_start, fix.byte_end));
    let mut applicable = Vec::new();
    // Validate each compiler range before comparing its current bytes.
    for fix in ordered {
        validate_fix_range(source.len(), source_text, &fix)?;
        if source.get(fix.byte_start..fix.byte_end) != Some(fix.replacement.as_slice()) {
            applicable.push(fix);
        }
    }
    Ok(applicable)
}

/// Validate one compiler-provided byte range and its UTF-8 boundaries.
const fn validate_fix_range(
    byte_length: usize,
    source_text: &str,
    fix: &MachineFix,
) -> Result<(), FixError> {
    // Reject ranges outside the current source before checking UTF-8 boundaries.
    if fix.byte_start > fix.byte_end || fix.byte_end > byte_length {
        return Err(FixError::InvalidRange {
            start: fix.byte_start,
            end: fix.byte_end,
            byte_length,
        });
    }
    if source_text.is_char_boundary(fix.byte_start) && source_text.is_char_boundary(fix.byte_end) {
        Ok(())
    } else {
        Err(FixError::InvalidUtf8Boundary)
    }
}

/// Reject overlapping replacements and duplicate insertions.
fn ensure_non_overlapping(fixes: &[MachineFix]) -> Result<(), FixError> {
    // Adjacent sorted spans are sufficient to detect every overlap.
    let is_overlapping = fixes.windows(2).any(|pair| {
        let [previous, current] = pair else {
            return false;
        };
        current.byte_start < previous.byte_end
            || (previous.byte_start == previous.byte_end
                && current.byte_start == previous.byte_start)
    });
    if is_overlapping {
        Err(FixError::Overlapping)
    } else {
        Ok(())
    }
}

/// Apply all safe compiler replacements without partially validating one file.
pub(super) fn apply_machine_fixes(
    repo: &Path,
    fixes: &[MachineFix],
) -> Result<AppliedFixes, DiagnosticError> {
    let mut grouped = BTreeMap::<PathBuf, Vec<MachineFix>>::new();
    // Group by repository-relative path so each source file is validated atomically.
    for fix in fixes {
        grouped
            .entry(fix.file.clone())
            .or_default()
            .extend([fix.clone()]);
    }

    // Read and validate every file before writing any file.
    let mut updated = Vec::new();
    for (file, file_fixes) in grouped {
        let path = repo.join(&file);
        let mut source = fs::read(&path).map_err(|source| DiagnosticError::ReadSource {
            path: file.clone(),
            source,
        })?;
        let suggestions = apply_fixes_to_source(&mut source, &file_fixes).map_err(|source| {
            DiagnosticError::ApplyFixes {
                path: file.clone(),
                source,
            }
        })?;
        if suggestions > 0 {
            updated.push((file, source, suggestions));
        }
    }

    // Commit the validated batch only after every file passes validation.
    for (file, source, _suggestions) in &updated {
        // Preserve all-or-nothing behavior by writing only after validation completed.
        fs::write(repo.join(file), source).map_err(|source| DiagnosticError::WriteSource {
            path: file.clone(),
            source,
        })?;
    }
    Ok(AppliedFixes {
        suggestions: updated.iter().map(|(_, _, count)| count).sum(),
        files: updated.len(),
    })
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
    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        None
    } else {
        // Return only paths that remain inside the caller-selected repository.
        Some(relative)
    }
}

/// Ask Git for zero-context hunks in the revision range.
pub(super) fn changed_ranges(repo: &Path, range: &str) -> Result<ChangedRanges, DiagnosticError> {
    // Ask Git for only Rust and manifest changes because those are lint inputs.
    let output = Command::new("git")
        .current_dir(repo)
        .args([
            "diff",
            "--unified=0",
            "--no-ext-diff",
            "--relative",
            range,
            "--",
            "*.rs",
            "Cargo.toml",
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

    // Parse only file headers and the standard new-file hunk coordinates.
    let mut result = ChangedRanges::new();
    let mut current_file: Option<&str> = None;
    let text = String::from_utf8_lossy(&output.stdout);
    // Associate each zero-context hunk with its preceding new-file header.
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_file = Some(path);
            continue;
        }
        if line == "+++ /dev/null" {
            current_file = None;
            continue;
        }
        // Ignore metadata until a retained destination file is active.
        let Some(file) = current_file else {
            continue;
        };
        let Some((start, count)) = new_hunk_range(line) else {
            continue;
        };
        if count > 0 {
            result
                .entry(file.to_owned())
                .or_default()
                .push((start, start + count - 1));
        }
    }
    Ok(result)
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

/// Project warning and error diagnostics from Cargo JSON output.
pub(super) fn diagnostics_from_cargo_output(
    tool: &str,
    repo: &Path,
    output: &str,
) -> Vec<Diagnostic> {
    // Ignore non-JSON tool chatter and Cargo messages that carry no compiler diagnostic.
    output
        .lines()
        .filter_map(|line| serde_json::from_str::<CargoMessage>(line).ok())
        .filter(|item| item.reason.as_deref() == Some("compiler-message"))
        .filter_map(|item| item.message)
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
    let path = PathBuf::from(file);
    if path.is_absolute() {
        path.strip_prefix(repo)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned()
    } else {
        file.strip_prefix("./").unwrap_or(file).to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{FixError, MachineFix, apply_fixes_to_source, new_hunk_range, parse_machine_fixes};

    /// Compiler JSON suggestions include both top-level and child diagnostics.
    #[test]
    fn parses_machine_applicable_suggestions() {
        let output = r#"
{"reason":"compiler-message","message":{"level":"warning","message":"replace the first byte","spans":[{"file_name":"/repo/src/lib.rs","byte_start":0,"byte_end":1,"line_start":1,"line_end":1,"is_primary":true,"suggested_replacement":"A","suggestion_applicability":"MachineApplicable"}],"children":[{"level":"help","message":"replace the middle bytes","spans":[{"file_name":"/repo/src/lib.rs","byte_start":3,"byte_end":5,"line_start":1,"line_end":1,"is_primary":true,"suggested_replacement":"DE","suggestion_applicability":"MachineApplicable"}],"children":[]},{"level":"help","message":"do not apply this","spans":[{"file_name":"/repo/src/lib.rs","byte_start":1,"byte_end":2,"line_start":1,"line_end":1,"is_primary":true,"suggested_replacement":"X","suggestion_applicability":"MaybeIncorrect"}],"children":[]}]}}
"#;

        assert_eq!(
            parse_machine_fixes(Path::new("/repo"), output),
            vec![
                MachineFix {
                    file: PathBuf::from("src/lib.rs"),
                    byte_start: 0,
                    byte_end: 1,
                    replacement: b"A".to_vec(),
                },
                MachineFix {
                    file: PathBuf::from("src/lib.rs"),
                    byte_start: 3,
                    byte_end: 5,
                    replacement: b"DE".to_vec(),
                },
            ]
        );
    }

    /// Applying edits from the end preserves every earlier source offset.
    #[test]
    fn applies_non_overlapping_fixes_from_the_end() {
        let mut source = b"abcdef".to_vec();
        let fixes = vec![
            MachineFix {
                file: PathBuf::from("src/lib.rs"),
                byte_start: 0,
                byte_end: 1,
                replacement: b"A".to_vec(),
            },
            MachineFix {
                file: PathBuf::from("src/lib.rs"),
                byte_start: 3,
                byte_end: 5,
                replacement: b"DE".to_vec(),
            },
        ];

        assert!(matches!(apply_fixes_to_source(&mut source, &fixes), Ok(2)));
        assert_eq!(source, b"AbcDEf");
    }

    /// Overlapping replacements fail before either replacement is applied.
    #[test]
    fn rejects_overlapping_fixes() {
        let mut source = b"abcdef".to_vec();
        let fixes = vec![
            MachineFix {
                file: PathBuf::from("src/lib.rs"),
                byte_start: 1,
                byte_end: 4,
                replacement: b"X".to_vec(),
            },
            MachineFix {
                file: PathBuf::from("src/lib.rs"),
                byte_start: 3,
                byte_end: 5,
                replacement: b"Y".to_vec(),
            },
        ];

        assert!(matches!(
            apply_fixes_to_source(&mut source, &fixes),
            Err(FixError::Overlapping)
        ));
        assert_eq!(source, b"abcdef");
    }

    /// A repeated identical suggestion does not create another fixer pass.
    #[test]
    fn ignores_noop_fixes() {
        let mut source = b"abcdef".to_vec();
        let fixes = [MachineFix {
            file: PathBuf::from("src/lib.rs"),
            byte_start: 1,
            byte_end: 3,
            replacement: b"bc".to_vec(),
        }];

        assert!(matches!(apply_fixes_to_source(&mut source, &fixes), Ok(0)));
        assert_eq!(source, b"abcdef");
    }

    /// Unified hunk coordinates retain their inclusive start and count.
    #[test]
    fn parses_new_hunk_range() {
        // Parse an explicit multi-line new-file range.
        assert_eq!(new_hunk_range("@@ -4,2 +9,3 @@"), Some((9, 3)));
        // Default a missing new-file count to one line.
        assert_eq!(new_hunk_range("@@ -1 +7 @@"), Some((7, 1)));
        // Reject text outside the unified hunk grammar.
        assert_eq!(new_hunk_range("not a hunk"), None);
    }
}
