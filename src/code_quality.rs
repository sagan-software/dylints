//! GitLab Code Quality report projection for Rust diagnostics.

use std::path::PathBuf;

use serde::Serialize;

use crate::diagnostics::Diagnostic;

/// One GitLab Code Quality report entry.
#[derive(Debug, Eq, PartialEq, Serialize)]
pub(super) struct CodeQualityViolation<'diagnostic> {
    /// Human-readable violation text.
    description: &'diagnostic str,
    /// Stable lint, compiler, or phase name.
    check_name: &'diagnostic str,
    /// Stable identity for one finding at one source location.
    fingerprint: String,
    /// GitLab's closed severity vocabulary.
    severity: CodeQualitySeverity,
    /// Repository-relative source location.
    location: CodeQualityLocation,
}

/// GitLab's accepted Code Quality severity values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum CodeQualitySeverity {
    /// A compiler error or denied lint.
    Blocker,
    /// A compiler warning.
    Major,
}

/// Repository-relative source location for one violation.
#[derive(Debug, Eq, PartialEq, Serialize)]
struct CodeQualityLocation {
    /// Path without a leading `./`.
    path: PathBuf,
    /// Inclusive source line coordinates.
    lines: CodeQualityLines,
}

/// First line of one Code Quality violation.
#[derive(Debug, Eq, PartialEq, Serialize)]
struct CodeQualityLines {
    /// Inclusive first source line.
    begin: u64,
}

/// Project every source-backed diagnostic into one GitLab report array.
pub(super) fn violations(diagnostics: &[Diagnostic]) -> Vec<CodeQualityViolation<'_>> {
    // GitLab requires a source location, so infrastructure diagnostics remain in runner logs.
    diagnostics.iter().filter_map(project).collect()
}

/// Project one compiler diagnostic into GitLab's required report fields.
fn project(diagnostic: &Diagnostic) -> Option<CodeQualityViolation<'_>> {
    // GitLab requires both a source path and a one-based line before it can display a finding.
    let (path, line) = diagnostic.primary_location()?;
    // Prefer the stable lint code while retaining the emitting tool for compiler diagnostics.
    let check_name = if diagnostic.code().is_empty() {
        diagnostic.tool()
    } else {
        diagnostic.code()
    };
    // Keep the same finding stable across pipelines while separating repeated source locations.
    let fingerprint = fingerprint(&[
        diagnostic.tool(),
        check_name,
        diagnostic.message(),
        path,
        &line.to_string(),
    ]);
    Some(CodeQualityViolation {
        description: diagnostic.message(),
        check_name,
        fingerprint,
        severity: match diagnostic.level() {
            "error" => CodeQualitySeverity::Blocker,
            _ => CodeQualitySeverity::Major,
        },
        location: CodeQualityLocation {
            path: path.into(),
            lines: CodeQualityLines { begin: line },
        },
    })
}

/// Compute a deterministic FNV-1a fingerprint over length-delimited fields.
fn fingerprint(fields: &[&str]) -> String {
    // Start from the FNV-1a offset basis so every report uses one stable algorithm.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    // Length-prefix each field before hashing its bytes to prevent concatenation aliases.
    for field in fields {
        // Delimit fields so adjacent values cannot alias through concatenation.
        for byte in field.len().to_le_bytes().into_iter().chain(field.bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    // Render fixed-width hexadecimal output for GitLab's stable fingerprint field.
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::violations;
    use crate::diagnostics::diagnostics_from_cargo_output;

    /// Render the report exactly as the runner serializes it.
    fn report(cargo_output: &str, tool: &str) -> String {
        let diagnostics = diagnostics_from_cargo_output(tool, Path::new("/repo"), cargo_output);
        serde_json::to_string_pretty(&violations(&diagnostics))
            .expect("the typed report should serialize")
    }

    #[test]
    fn serializes_required_gitlab_fields() {
        // Start from Cargo's public JSON message format rather than constructing internals.
        let report = report(
            r#"{"reason":"compiler-message","message":{"level":"warning","message":"prefer a borrowed input","code":{"code":"ownership_at_boundaries"},"spans":[{"file_name":"src/lib.rs","line_start":7,"line_end":7,"is_primary":true}]}}"#,
            "dylint",
        );

        // Verify GitLab's required fields and UTF-8 array framing together.
        assert!(report.starts_with('['));
        assert!(report.contains("\"check_name\": \"ownership_at_boundaries\""));
        assert!(report.contains("\"severity\": \"major\""));
        assert!(report.contains("\"path\": \"src/lib.rs\""));
        assert!(report.contains("\"begin\": 7"));
        assert!(!report.starts_with('\u{feff}'));
    }

    #[test]
    fn uncoded_errors_use_the_tool_name_and_blocker_severity() {
        // A hard compiler error without a lint code still needs a stable check name.
        let report = report(
            r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot find value","code":null,"spans":[{"file_name":"src/main.rs","line_start":3,"line_end":3,"is_primary":true}]}}"#,
            "clippy",
        );

        assert!(report.contains("\"check_name\": \"clippy\""), "{report}");
        assert!(report.contains("\"severity\": \"blocker\""), "{report}");
    }

    #[test]
    fn omits_diagnostics_without_source_locations() {
        // GitLab cannot represent a compiler diagnostic without a source location.
        let report = report(
            r#"{"reason":"compiler-message","message":{"level":"error","message":"build failed","code":null,"spans":[]}}"#,
            "clippy",
        );

        // Retain a valid empty JSON report rather than inventing a location.
        assert_eq!(report, "[]");
    }
}
