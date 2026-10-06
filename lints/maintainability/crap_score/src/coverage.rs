#![expect(
    clippy::disallowed_methods,
    reason = "a compiler lint reads one measured local coverage report synchronously"
)]

//! Read the documented SF/DA executable-line LCOV profile.
//! Missing measured hits remain absent.

use std::{
    collections::BTreeMap,
    fs,
    num::NonZeroUsize,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

/// Canonical source identities and maximum observed hits for each executable line.
#[derive(Debug, Default)]
pub(super) struct Coverage(BTreeMap<PathBuf, BTreeMap<NonZeroUsize, u64>>);

/// Report failures preserve their source and distinguish malformed measured input.
#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    /// Reading a report or resolving a source failed.
    #[error("cannot access {path}: {source}")]
    Io {
        /// Report or source whose filesystem operation failed.
        path: PathBuf,
        /// Original filesystem failure.
        source: std::io::Error,
    },
    /// A measured line appeared outside a source record.
    #[error("DA record has no SF source")]
    MissingSource,
    /// A line number or hit count violates the documented input profile.
    #[error("invalid DA record: {0}")]
    InvalidLine(String),
    /// The configured score cannot be compared meaningfully.
    #[error("threshold must be finite and nonnegative")]
    InvalidThreshold,
}

impl Coverage {
    /// Read a report and resolve SF paths relative to the compiler working directory.
    pub(super) fn read(path: &Path) -> Result<Self, Error> {
        let source = fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        source.parse()
    }

    /// Return measured coverage; absent files and empty ranges have no score.
    pub(super) fn fraction(&self, path: &Path, lines: RangeInclusive<usize>) -> Option<f64> {
        // Resolve each callable once; line records borrow the per-source mapping.
        let path = fs::canonicalize(path).ok()?;
        let lines = NonZeroUsize::new(*lines.start())?..=NonZeroUsize::new(*lines.end())?;
        let mut measured = 0_u32;
        let mut covered = 0_u32;
        // Missing measured lines stay absent rather than becoming invented zero hits.
        for (_line, hits) in self.0.get(&path)?.range(lines) {
            measured += 1;
            covered += u32::from(*hits > 0);
        }
        (measured > 0).then(|| f64::from(covered) / f64::from(measured))
    }
}

impl std::str::FromStr for Coverage {
    type Err = Error;
    /// Accept source and line records; reject malformed DA instead of guessing coverage.
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        // Borrow one source mapping until its explicit record terminator.
        let mut result = Self::default();
        let mut file = None;
        for record in source.lines() {
            if let Some(path) = record.strip_prefix("SF:") {
                // A source record must resolve before accepting its executable lines.
                let canonical = fs::canonicalize(path).map_err(|source| Error::Io {
                    path: PathBuf::from(path),
                    source,
                })?;
                file = Some(result.0.entry(canonical).or_default());
            } else if let Some(data) = record.strip_prefix("DA:") {
                // Alias records merge maximum hits without cloning a source path per line.
                let lines = file.as_mut().ok_or(Error::MissingSource)?;
                let (line, hits) =
                    line_hits(data).ok_or_else(|| Error::InvalidLine(record.to_owned()))?;
                let _previous = lines
                    .entry(line)
                    .and_modify(|previous| *previous = (*previous).max(hits))
                    .or_insert(hits);
            } else if record == "end_of_record" {
                file = None;
            }
        }
        Ok(result)
    }
}

/// Accept two decimal fields and one optional checksum with a positive line number.
fn line_hits(source: &str) -> Option<(NonZeroUsize, u64)> {
    // The checksum is opaque; no other field is part of this local line profile.
    let mut fields = source.split(',');
    let measured = measured_fields(&mut fields)?;
    let _checksum = fields.next();
    // Reject additional fields instead of silently narrowing their meaning.
    fields.next().is_none().then_some(measured)
}

/// Parse the line and hit identities through their destination numeric types.
fn measured_fields(fields: &mut std::str::Split<'_, char>) -> Option<(NonZeroUsize, u64)> {
    let (line, hits) = fields.next().zip(fields.next())?;
    let line = decimal(line)?;
    let hits = decimal(hits)?;
    line.parse().ok().zip(hits.parse().ok())
}

/// Preserve the unsigned decimal grammar independently of Rust integer parser extensions.
fn decimal(source: &str) -> Option<&str> {
    (!source.is_empty() && source.bytes().all(|byte| byte.is_ascii_digit())).then_some(source)
}

#[cfg(test)]
mod tests {
    use super::{Coverage, Error};

    /// Resolve the checked-in compiler fixture used by measured reports and range
    /// tests, keeping source identity consistent across independently named cases.
    fn source_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/main.rs")
    }

    /// Build two alias records whose second measurement covers one previously
    /// uncovered line, so range tests observe maximum hits rather than duplicates.
    fn fixture_coverage() -> (Coverage, std::path::PathBuf) {
        // Both records resolve to one source identity before executable lines merge.
        let path = source_path();
        let display = path.display();
        let report = format!(
            "SF:{display}\nDA:1,0\nDA:2,0\nend_of_record\nSF:{display}\nDA:2,4,checksum\nend_of_record\n"
        );
        // Return the measured mapping with the source used to construct its records.
        let coverage = report.parse().expect("valid measured lines");
        (coverage, path)
    }

    /// Missing ranges stay absent while measured zero and full hits retain their fractions.
    #[test_case::test_case(1, 2, Some(0.5); "merged alias fraction")]
    #[test_case::test_case(1, 1, Some(0.0); "measured zero hits")]
    #[test_case::test_case(2, 2, Some(1.0); "measured full hits")]
    #[test_case::test_case(3, 3, None; "unmeasured range")]
    #[test_case::test_case(0, 2, None; "zero start")]
    #[test_case::test_case(1, 0, None; "zero end")]
    fn measured_fraction(first: usize, last: usize, expected: Option<f64>) {
        let (coverage, path) = fixture_coverage();
        assert_eq!(coverage.fraction(&path, first..=last), expected);
    }

    /// A missing source path cannot acquire coverage from another file's measurements.
    #[test]
    fn missing_source_has_no_fraction() {
        let (coverage, path) = fixture_coverage();
        let missing = path.with_file_name("missing-source");
        assert_eq!(coverage.fraction(&missing, 1..=2), None);
    }

    /// Line data requires an active source even after a record has terminated.
    #[test_case::test_case("DA:1,0"; "missing source")]
    #[test_case::test_case("end_of_record\nDA:1,0"; "terminated empty record")]
    fn rejects_lines_without_source(report: &str) {
        assert!(matches!(
            report.parse::<Coverage>(),
            Err(Error::MissingSource)
        ));
    }

    /// An explicit terminator clears a source that previously accepted measured lines.
    #[test]
    fn rejects_lines_after_source_terminates() {
        // Accept one measured line before proving that the terminator clears its source.
        let path = source_path();
        let display = path.display();
        let report = format!("SF:{display}\nDA:1,0\nend_of_record\nDA:2,0");
        assert!(matches!(
            report.parse::<Coverage>(),
            Err(Error::MissingSource)
        ));
    }

    /// An unresolved source preserves its filesystem failure instead of producing a score.
    #[test]
    fn rejects_unresolved_source() {
        assert!(matches!(
            "SF:/nonexistent/crap-source".parse::<Coverage>(),
            Err(Error::Io { .. })
        ));
    }

    /// Each malformed executable-line field fails independently at the line boundary.
    #[test_case::test_case("DA:0,0"; "zero line")]
    #[test_case::test_case("DA:bad,0"; "nondecimal line")]
    #[test_case::test_case("DA:,0"; "empty line")]
    #[test_case::test_case("DA:1,-1"; "negative hits")]
    #[test_case::test_case("DA:1"; "missing hits")]
    #[test_case::test_case("DA:1,NaN"; "nondecimal hits")]
    #[test_case::test_case("DA:+1,0"; "signed line")]
    #[test_case::test_case("DA:1,+1"; "signed hits")]
    #[test_case::test_case("DA:1,0,x,extra"; "extra field")]
    #[test_case::test_case("DA:1,18446744073709551616"; "hits overflow")]
    fn rejects_invalid_line(record: &str) {
        // Resolve a valid source first so the failure belongs to its measured line.
        let path = source_path();
        let display = path.display();
        let report = format!("SF:{display}\n{record}");
        assert!(matches!(
            report.parse::<Coverage>(),
            Err(Error::InvalidLine(_))
        ));
    }

    /// Unrelated LCOV records do not invent executable lines in an empty report.
    #[test]
    fn ignores_unmeasured_records() {
        let coverage: Coverage = "TN:ignored\nend_of_record".parse().expect("empty profile");
        assert_eq!(coverage.fraction(&source_path(), 1..=2), None);
    }

    /// Reading a missing report preserves the I/O failure category.
    #[test]
    fn rejects_missing_report() {
        assert!(matches!(
            Coverage::read(std::path::Path::new("missing-report")),
            Err(Error::Io { .. })
        ));
    }

    /// A checked-in measured report retains its zero-hit executable line after reading.
    #[test]
    fn reads_measured_report() {
        let report = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/coverage.info");
        let coverage = Coverage::read(&report).expect("checked-in report");
        assert_eq!(coverage.fraction(&source_path(), 1..=1), Some(0.0));
    }
}
