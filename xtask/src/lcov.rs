#![expect(
    clippy::cast_precision_loss,
    reason = "coverage percentages are approximate projections of exact integer line counts"
)]
//! Canonical executable-line coverage from LLVM LCOV export.
//!
//! LLVM's export contains `SF` source paths and `DA` line/hit records.
//! <https://llvm.org/docs/CommandGuide/llvm-cov.html#export-command>

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Write as _},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{error::Error, paths};

/// Canonical source and line identities with maximum hits across lexical
/// aliases.
pub(crate) type Lines = BTreeMap<(PathBuf, u64), u64>;

/// Executable-line counts for a report scope.
#[derive(Debug, Serialize)]
pub(crate) struct Metric {
    /// Number of executable lines.
    pub(crate) count: usize,
    /// Number of executable lines reached by a test.
    pub(crate) covered: usize,
    /// Number of executable lines with zero hits.
    notcovered: usize,
    /// Covered executable lines as a percentage.
    pub(crate) percent: f64,
}

/// LLVM counters shared by all summary metrics; ancillary fields differ by
/// metric.
#[derive(Deserialize)]
struct Counts {
    /// Total measured entities for this metric.
    count: usize,
    /// Measured entities reached by the instrumented run.
    covered: usize,
}

impl<'de> Deserialize<'de> for Metric {
    /// Normalize optional LLVM percentage and gap fields from the authoritative
    /// counts.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let counts = Counts::deserialize(deserializer)?;
        if counts.covered > counts.count {
            return Err(serde::de::Error::custom(
                "covered count exceeds total count",
            ));
        }
        Ok(Self::new(counts.count, counts.covered))
    }
}

impl Metric {
    /// Calculate counts from unique line identities rather than compiled
    /// mappings.
    pub(crate) fn new(count: usize, covered: usize) -> Self {
        Self {
            count,
            covered,
            notcovered: count - covered,
            percent: if count == 0 {
                0.0
            } else {
                100.0 * covered as f64 / count as f64
            },
        }
    }
}

/// Decode LLVM executable-line records and merge canonical path aliases.
pub(crate) fn parse(
    source: &str,
    root: &Path,
    selected: &BTreeSet<PathBuf>,
) -> Result<Lines, Error> {
    // Keep the active source separate from line records and record terminators.
    let mut result = Lines::new();
    let mut file = None;
    for line in source.lines() {
        if let Some(source) = line.strip_prefix("SF:") {
            let path = paths::canonical(Path::new(source), root)?;
            // Exclude fixtures after resolving path aliases.
            file = is_selected(&path, selected).then_some(path);
        } else if line == "end_of_record" {
            file = None;
        } else if let Some(data) = line.strip_prefix("DA:")
            && let Some(path) = &file
            && let Some((number, hits)) = line_hits(data)
        {
            // Different compiled mappings may report different hits for one line.
            let entry = result.entry((path.clone(), number)).or_default();
            *entry = (*entry).max(hits);
        }
    }
    Ok(result)
}

/// Apply the source profile and optional explicit scope to a canonical path.
fn is_selected(path: &Path, selected: &BTreeSet<PathBuf>) -> bool {
    paths::is_source(path) && (selected.is_empty() || selected.contains(path))
}

/// Decode the first two decimal fields and ignore LLVM's optional checksum.
fn line_hits(source: &str) -> Option<(u64, u64)> {
    let mut fields = source.split(',');
    let decimal = |text: &str| {
        (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| text.parse::<u64>().ok())
            .flatten()
    };
    Some((decimal(fields.next()?)?, decimal(fields.next()?)?))
}

/// Aggregate unique executable lines for the configured threshold.
pub(crate) fn metric(lines: &Lines) -> Metric {
    Metric::new(
        lines.len(),
        lines.values().filter(|hits| **hits > 0).count(),
    )
}

/// Encode one LCOV record per canonical source file, merging every compiled
/// variant before exposing the report to other LCOV consumers.
pub(crate) fn encode(lines: &Lines) -> String {
    // The map gives one source-ordered record per canonical file path.
    let mut files: BTreeMap<&Path, Vec<(u64, u64)>> = BTreeMap::new();
    for ((path, line), hits) in lines {
        // `Lines` has already merged compiled variants by canonical path and line.
        files
            .entry(path.as_path())
            .or_default()
            .push((*line, *hits));
    }

    let mut output = String::new();
    for (path, entries) in files {
        // Keep each file header adjacent to its counters for ordinary LCOV readers.
        output.push_str("SF:");
        output.push_str(&path.to_string_lossy());
        output.push('\n');
        for (line, hits) in entries {
            push_format(&mut output, format_args!("DA:{line},{hits}\n"));
        }
        // A complete record lets downstream tools distinguish file boundaries.
        output.push_str("end_of_record\n");
    }
    output
}

/// Render the same canonical executable-line measurements for people.
pub(crate) fn html(lines: &Lines, root: &Path) -> String {
    // Build the headline and detail rows from the same canonical summary.
    let report = summary(lines);
    let total = &report.totals.lines;
    let mut html = String::from(HTML_START);
    push_format(
        &mut html,
        format_args!(
            "{:.2}% line coverage ({}/{}) across {} canonical source files. Compiler path aliases are merged by source file and line; a line counts as covered when any compiled variant reaches it.</p><div class=\"table-wrap\"><table><thead><tr><th scope=\"col\">Source file</th><th scope=\"col\">Line coverage</th><th scope=\"col\">Covered lines</th><th scope=\"col\">Uncovered executable lines</th></tr></thead><tbody>",
            total.percent,
            total.covered,
            total.count,
            report.files.len()
        ),
    );
    // Each row preserves exact gaps so readers can inspect uncovered work.
    for file in &report.files {
        html.push_str(&html_row(file, root));
    }
    html.push_str(HTML_END);
    html
}

/// Render a plain-text summary from the same canonical line inventory.
pub(crate) fn text_summary(lines: &Lines, root: &Path) -> String {
    // Reuse canonical totals so terminal output agrees with HTML and JSON.
    let report = summary(lines);
    let mut output = String::new();
    push_format(
        &mut output,
        format_args!(
            "Canonical executable line coverage: {:.2}% ({}/{})\nCanonical source files: {}\n",
            report.totals.lines.percent,
            report.totals.lines.covered,
            report.totals.lines.count,
            report.files.len()
        ),
    );
    // Stable source order makes per-file changes easy to compare in CI logs.
    for file in &report.files {
        let display_path = display_path(file.filename, root);
        push_format(
            &mut output,
            format_args!(
                "{:6.2}% ({}/{}) {display_path}\n",
                file.lines.percent, file.lines.covered, file.lines.count
            ),
        );
    }
    output
}

/// Render one canonical file row with exact uncovered-line details.
fn html_row(file: &FileReport<'_>, root: &Path) -> String {
    // Escape paths before inserting repository text into markup.
    let display_path = escape_html(&display_path(file.filename, root));
    let missing = missing_lines(&file.missing_lines);
    let mut row = String::new();
    // Keep percentages, hit totals, and exact gaps together for each source row.
    push_format(
        &mut row,
        format_args!(
            "<tr><th scope=\"row\"><code>{display_path}</code></th><td>{:.2}%</td><td>{}/{}</td><td>{missing}</td></tr>",
            file.lines.percent, file.lines.covered, file.lines.count
        ),
    );
    row
}

/// Format a compact expandable list of executable lines without hits.
fn missing_lines(lines: &[u64]) -> String {
    if lines.is_empty() {
        return "None".to_owned();
    }
    let numbers = lines
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "<details><summary>{} lines</summary><code>{numbers}</code></details>",
        lines.len()
    )
}

/// Shorten canonical report filenames to workspace-relative paths.
fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Append formatted text to a String; writing into a String cannot fail.
fn push_format(output: &mut String, arguments: fmt::Arguments<'_>) {
    output
        .write_fmt(arguments)
        .expect("formatting into a String cannot fail");
}

/// Escape source paths before placing them in HTML text.
#[expect(
    one_use_private_helper,
    reason = "The named helper centralizes the HTML escaping invariant for report paths."
)]
fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Opening document markup and styles for the standalone coverage report.
const HTML_START: &str = r#"<!doctype html>
<html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Dylints coverage</title>
<style>
:root { font-family: system-ui, sans-serif; color: #202b36; background: #fff; }
body { max-width: 1100px; margin: 0 auto; padding: 2rem; }
h1 { margin: .2rem 0; }
.summary { color: #465564; margin: 0 0 1.5rem; }
table { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; }
th, td { border-bottom: 1px solid #d6dde4; padding: .65rem .75rem; text-align: left; vertical-align: top; }
thead th { position: sticky; top: 0; background: #f3f6f8; }
td:nth-child(2), td:nth-child(3) { white-space: nowrap; }
code { font-family: ui-monospace, monospace; overflow-wrap: anywhere; }
details summary { cursor: pointer; color: #155b87; }
details[open] summary { margin-bottom: .4rem; }
@media (max-width: 700px) {
  body { padding: 1rem; }
  .table-wrap { overflow-x: auto; }
  table { min-width: 650px; }
  th, td { padding: .55rem; }
}
</style></head><body><main><h1>Dylints coverage</h1><p class="summary">"#;

/// Closing document markup shared by the coverage report renderer.
const HTML_END: &str = "</tbody></table></div></main></body></html>\n";

/// Per-file canonical line coverage and exact uncovered lines.
#[derive(Serialize)]
struct FileReport<'path> {
    /// Canonical Rust source file.
    filename: &'path Path,
    /// Unique executable-line metric.
    lines: Metric,
    /// Executable source lines with no hits.
    missing_lines: Vec<u64>,
}

/// Canonical JSON coverage summary and its unique source-file rows.
#[derive(Serialize)]
pub(crate) struct Summary<'path> {
    /// Measurement scope for this report.
    scope: &'static str,
    /// One entry for each canonical Rust source file.
    files: Vec<FileReport<'path>>,
    /// Aggregate line measurements.
    totals: Totals,
}

/// Aggregate canonical metrics.
#[derive(Serialize)]
struct Totals {
    /// Executable-line counts across canonical sources.
    lines: Metric,
}

/// Serialize the canonical summary while retaining stable filename and line
/// order.
pub(crate) fn summary(lines: &Lines) -> Summary<'_> {
    // Group canonical lines by borrowed source identity in stable path order.
    let mut files: BTreeMap<&Path, Vec<(u64, u64)>> = BTreeMap::new();
    for ((path, line), hits) in lines {
        files
            .entry(path.as_path())
            .or_default()
            .push((*line, *hits));
    }
    // Each file retains its exact zero-hit lines alongside approximate percentages.
    let reports: Vec<_> = files
        .into_iter()
        .map(|(filename, entries)| FileReport {
            filename,
            lines: Metric::new(
                entries.len(),
                entries.iter().filter(|(_, hits)| *hits > 0).count(),
            ),
            missing_lines: entries
                .iter()
                .filter_map(|(line, hits)| (*hits == 0).then_some(*line))
                .collect(),
        })
        .collect();
    Summary {
        scope: "canonical executable LCOV DA lines from production and test targets; cfg(test) included; examples and fixtures excluded",
        files: reports,
        totals: Totals {
            lines: metric(lines),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{metric, parse, summary};
    use std::{collections::BTreeSet, path::Path};

    /// Real LLVM function and line summaries omit notcovered; branch summaries
    /// retain it.
    #[test_case::test_case(r#"{"count":2,"covered":1,"percent":50}"#; "without gaps")]
    #[test_case::test_case(r#"{"count":2,"covered":1,"notcovered":1,"percent":50}"#; "with gaps")]
    fn llvm_counter_profiles(input: &str) {
        let metric: super::Metric = serde_json::from_str(input).expect("LLVM counters");
        assert_eq!((metric.count, metric.covered, metric.notcovered), (2, 1, 1));
    }

    /// Invalid counters fail before derived percentages and gaps are
    /// constructed.
    #[test_case::test_case(r#"{"count":1,"covered":2}"#; "covered exceeds count")]
    #[test_case::test_case(r#"{"count":1}"#; "missing covered count")]
    fn rejects_invalid_counters(input: &str) {
        assert!(serde_json::from_str::<super::Metric>(input).is_err());
    }

    /// Lexical aliases merge disjoint hits and preserve the exact uncovered
    /// line.
    #[test]
    #[expect(
        many_assertions_in_test,
        reason = "The test separately protects merged counts, canonical identity, LCOV, HTML, and gaps."
    )]
    fn merges_aliases_with_maximum_hits() {
        let root = Path::new("/workspace");
        // Aliases contribute complementary hits to the same physical file.
        let input = "SF:/workspace/src/../shared.rs\nDA:10,1\nDA:20,0\nDA:30,0\nend_of_record\nSF:/workspace/shared.rs\nDA:10,0\nDA:20,3,checksum\nDA:30,0\nend_of_record\n";
        let lines = parse(input, root, &BTreeSet::new()).unwrap();
        let counts = metric(&lines);
        // Both aliases must contribute to the canonical file's executable-line counts.
        assert_eq!((counts.count, counts.covered), (3, 2));
        // The output exposes both canonical identity and exact uncovered lines.
        let report = summary(&lines);
        let file = report.files.first().expect("canonical file");
        assert_eq!(file.missing_lines, [30]);
        assert_eq!(file.filename, Path::new("/workspace/shared.rs"));
        // LCOV consumers receive one record with the maximum hit count per line.
        let encoded = super::encode(&lines);
        assert_eq!(encoded.matches("SF:").count(), 1);
        assert!(encoded.contains("SF:/workspace/shared.rs\n"));
        assert!(encoded.contains("DA:10,1\nDA:20,3\nDA:30,0\n"));
        let html = super::html(&lines, root);
        assert_eq!(html.matches("shared.rs</code>").count(), 1);
        assert!(html.contains("66.67%"));
        assert!(html.contains("30</code>"));
    }

    /// Fixture aliases and unselected sources cannot enter canonical totals.
    #[test]
    #[expect(
        many_assertions_in_test,
        reason = "The selected file, aggregate rate, and rendered scope are separate selection guarantees."
    )]
    fn filters_selected_sources_after_normalization() {
        let root = Path::new("/workspace");
        // Source selection operates after canonicalization and fixture exclusion.
        let selected = BTreeSet::from([root.join("crates/sqlx/src/lib.rs")]);
        let input = "SF:crates/sqlx-support/../fixture/src/lib.rs\nDA:1,0\nend_of_record\nSF:crates/sqlx/src/lib.rs\nDA:2,7\nend_of_record\nSF:other.rs\nDA:3,0\nend_of_record\n";
        let lines = parse(input, root, &selected).unwrap();
        // Counts and percentages must use the same selected executable lines.
        assert_eq!((metric(&lines).count, metric(&lines).covered), (1, 1));
        assert!((summary(&lines).totals.lines.percent - 100.0).abs() < f64::EPSILON);
        let html = super::html(&lines, root);
        assert!(html.contains("crates/sqlx/src/lib.rs"));
        assert!(!html.contains("fixture"));
        assert!(!html.contains("other.rs"));
    }

    /// Malformed DA fields and records without a source cannot become
    /// executable lines.
    #[test]
    #[expect(
        many_assertions_in_test,
        reason = "Malformed input handling and empty-report output are independent parser contracts."
    )]
    fn malformed_and_empty_records_do_not_add_lines() {
        // Invalid records and records without an active source cannot add lines.
        let input = "DA:1,1\nSF:/workspace/source.rs\nDA:a,1\nDA:1,-1\nDA:2\nDA:3,NaN\nDA:4,1\nend_of_record\nDA:5,1\n";
        let lines = parse(input, Path::new("/workspace"), &BTreeSet::new()).unwrap();
        // The valid record contributes one line; empty reports still render zero coverage.
        assert_eq!(lines.len(), 1);
        assert!(metric(&super::Lines::new()).percent.abs() < f64::EPSILON);
        assert!(summary(&super::Lines::new()).files.is_empty());
        assert!(super::html(&super::Lines::new(), Path::new("/workspace")).contains("0.00%"));
    }
}
