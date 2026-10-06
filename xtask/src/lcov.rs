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

/// Serialize the canonical summary while retaining stable filename and line
/// order.
pub(crate) fn summary(lines: &Lines) -> serde_json::Value {
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
    serde_json::json!({
        "scope": "canonical executable LCOV DA lines from production and test targets; cfg(test) included; examples and fixtures excluded",
        "files": reports,
        "totals": {"lines": metric(lines)}
    })
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
    fn merges_aliases_with_maximum_hits() {
        let root = Path::new("/workspace");
        // Aliases contribute complementary hits to the same physical file.
        let input = "SF:/workspace/src/../shared.rs\nDA:10,1\nDA:20,0\nDA:30,0\nend_of_record\nSF:/workspace/shared.rs\nDA:10,0\nDA:20,3,checksum\nDA:30,0\nend_of_record\n";
        let lines = parse(input, root, &BTreeSet::new()).unwrap();
        let counts = metric(&lines);
        assert_eq!((counts.count, counts.covered), (3, 2));
        // The output exposes both canonical identity and exact uncovered lines.
        let report = summary(&lines);
        assert_eq!(report["files"][0]["missing_lines"], serde_json::json!([30]));
        assert_eq!(report["files"][0]["filename"], "/workspace/shared.rs");
    }

    /// Fixture aliases and unselected sources cannot enter canonical totals.
    #[test]
    fn filters_selected_sources_after_normalization() {
        let root = Path::new("/workspace");
        // Source selection operates after canonicalization and fixture exclusion.
        let selected = BTreeSet::from([root.join("lints/crates/sqlx/src/lib.rs")]);
        let input = "SF:lints/crates/sqlx/support/../fixture/src/lib.rs\nDA:1,0\nend_of_record\nSF:lints/crates/sqlx/src/lib.rs\nDA:2,7\nend_of_record\nSF:other.rs\nDA:3,0\nend_of_record\n";
        let lines = parse(input, root, &selected).unwrap();
        // Counts and percentages must use the same selected executable lines.
        assert_eq!((metric(&lines).count, metric(&lines).covered), (1, 1));
        assert_eq!(summary(&lines)["totals"]["lines"]["percent"], 100.0);
    }

    /// Malformed DA fields and records without a source cannot become
    /// executable lines.
    #[test]
    fn malformed_and_empty_records_do_not_add_lines() {
        // Invalid records and records without an active source cannot add lines.
        let input = "DA:1,1\nSF:/workspace/source.rs\nDA:a,1\nDA:1,-1\nDA:2\nDA:3,NaN\nDA:4,1\nend_of_record\nDA:5,1\n";
        let lines = parse(input, Path::new("/workspace"), &BTreeSet::new()).unwrap();
        assert_eq!(lines.len(), 1);
        assert!(metric(&super::Lines::new()).percent.abs() < f64::EPSILON);
        assert_eq!(
            summary(&super::Lines::new())["files"],
            serde_json::json!([])
        );
    }
}
