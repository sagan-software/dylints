#![feature(rustc_private)]
#![warn(unused_extern_crates)]
//! Coverage-weighted source complexity with explicit missing-data handling.
//!
//! The pass reads one measured LCOV report and combines its executable-line
//! fraction with the shared HIR cyclomatic complexity profile. Configuration
//! values are validated before storage. Missing measurements remain absent;
//! they never imply zero coverage. Macro-generated functions are excluded,
//! and invalid measured inputs prevent scoring and fail compilation.

mod coverage;

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use maintainability_support::{cyclomatic_complexity, is_macro_expansion};
use rustc_errors::DiagDecorator;
use rustc_hir::{Body, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, def_id::LocalDefId};
use serde::Deserialize;
use std::path::PathBuf;

/// Measured coverage input and the strictly exceeded score threshold.
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Config {
    /// LCOV file relative to the compiler working directory, or an absolute path.
    coverage_path: Option<PathBuf>,
    /// Dimensionless CRAP limit; equality passes.
    threshold: f64,
}

impl Default for Config {
    /// Keep measured coverage optional and use the original triage threshold.
    fn default() -> Self {
        Self {
            coverage_path: None,
            threshold: 30.0,
        }
    }
}

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub CRAP_SCORE, Warn, "function exceeds the coverage-weighted complexity limit",
    CrapScore, CrapScore::default()
}

/// Finite, nonnegative dimensionless threshold validated at configuration ingress.
#[derive(Clone, Copy, Debug)]
struct Threshold(f64);

impl TryFrom<f64> for Threshold {
    type Error = coverage::Error;
    /// Reject values that cannot participate in a finite score comparison.
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(coverage::Error::InvalidThreshold)
        }
    }
}

/// Closed report lifecycle; ready scores always have both coverage and a valid threshold.
#[derive(Debug)]
enum State {
    /// No measured report was requested.
    Disabled,
    /// Configuration is valid, but the compiler has not loaded the report yet.
    Pending {
        /// Local measured coverage report.
        path: PathBuf,
        /// Validated score comparison boundary.
        threshold: Threshold,
    },
    /// The report is loaded and ready for source-authored callable checks.
    Ready {
        /// Canonical measured executable lines.
        coverage: coverage::Coverage,
        /// Validated score comparison boundary.
        threshold: Threshold,
    },
    /// An invalid input prevents all scoring.
    Failed(coverage::Error),
}

impl State {
    /// Convert raw configuration immediately into a valid pending or terminal state.
    fn configured(config: Config) -> Self {
        let threshold = match Threshold::try_from(config.threshold) {
            Ok(threshold) => threshold,
            Err(error) => return Self::Failed(error),
        };
        config
            .coverage_path
            .map_or(Self::Disabled, |path| Self::Pending { path, threshold })
    }

    /// Load the report once at the compiler's crate-check transition.
    fn initialize(self) -> Self {
        match self {
            Self::Pending { path, threshold } => match coverage::Coverage::read(&path) {
                Ok(coverage) => Self::Ready {
                    coverage,
                    threshold,
                },
                Err(error) => Self::Failed(error),
            },
            Self::Disabled | Self::Ready { .. } | Self::Failed(_) => self,
        }
    }
}

/// One measured report reused across all callables in a compilation.
#[derive(Debug)]
struct CrapScore {
    /// Typed configuration and measured report lifecycle.
    state: State,
}

impl Default for CrapScore {
    /// Validate configuration without performing compiler or filesystem work yet.
    fn default() -> Self {
        let config = dylint_linting::config_or_default(env!("CARGO_PKG_NAME"));
        Self {
            state: State::configured(config),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for CrapScore {
    /// Validate configured measured inputs before checking function scores.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        // Consume the pending state before loading so one report serves every callable.
        self.state = std::mem::replace(&mut self.state, State::Disabled).initialize();
        if let State::Failed(error) = &self.state {
            let _error = cx.sess().dcx().err(format!("crap_score: {error}"));
        }
    }

    /// Score only source-authored functions with measured executable lines.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _definition: LocalDefId,
    ) {
        // Expansion has no independently editable callable source for this profile.
        if is_macro_expansion(span) {
            return;
        }
        // Scoring is unavailable until configuration and the measured report are valid.
        let State::Ready {
            coverage,
            threshold,
        } = &mut self.state
        else {
            return;
        };
        // A score requires measured executable lines in the callable's own source file.
        let Some(fraction) = measured_fraction(cx, body, coverage) else {
            return;
        };
        let complexity = cyclomatic_complexity(body);
        let score = score(complexity, fraction);
        // Equality passes; emit only scores that strictly exceed the validated threshold.
        if score <= threshold.0 {
            return;
        }
        let percent = fraction * 100.0;
        let threshold = threshold.0;
        // Keep the measured coverage and source revision requirement in the diagnostic.
        cx.emit_span_lint(CRAP_SCORE, span, DiagDecorator(move |diagnostic| {
            let _configured = diagnostic.primary_message(format!(
                "function has CRAP score {score:.2} (complexity {complexity}, measured line coverage {percent:.2}%), which exceeds {threshold:.2}"
            )).help("test the uncovered decisions or split independent responsibilities; verify that the coverage report matches this source revision");
        }));
    }
}

/// Project one source-authored body into an inclusive measured executable-line range.
fn measured_fraction<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Body<'tcx>,
    coverage: &mut coverage::Coverage,
) -> Option<f64> {
    // Resolve both endpoints before selecting a local source identity.
    let source_map = cx.sess().source_map();
    let start = source_map.lookup_char_pos(body.value.span.lo());
    let end = source_map.lookup_char_pos(body.value.span.hi());
    let path = start.file.name.clone().into_local_path()?;
    // A callable must remain in one file to have an unambiguous line projection.
    if start.file.start_pos != end.file.start_pos {
        return None;
    }
    coverage.fraction(&path, start.line..=end.line)
}

/// Apply the original dimensionless heuristic to the local executable-line profile.
fn score(complexity: u32, coverage_fraction: f64) -> f64 {
    // Complexity is a decision count; fraction is covered lines / measured executable lines.
    // The constants 1, 2 and 3 are dimensionless; no percentage conversion occurs here.
    let complexity = f64::from(complexity);
    complexity
        .powi(2)
        .mul_add((1.0 - coverage_fraction).powi(3), complexity)
}

/// Run measured zero-coverage and missing-coverage compiler fixtures.
#[test]
fn ui() {
    // Serialize the source path as TOML before configuring measured compiler fixtures.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/coverage.info");
    let coverage_path = toml_edit::Value::from(path.to_str().expect("UTF-8 fixture path"));
    let config = format!("[crap-score]\ncoverage_path = {coverage_path}\nthreshold = 30.0\n");
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui")
        .dylint_toml(&config)
        .run();
}

#[cfg(test)]
mod tests {
    use super::score;

    /// Invalid scores fail configuration even when no measured report was requested.
    #[test_case::test_case(-1.0; "negative")]
    #[test_case::test_case(f64::NAN; "NaN")]
    #[test_case::test_case(f64::INFINITY; "positive infinity")]
    #[test_case::test_case(f64::NEG_INFINITY; "negative infinity")]
    fn rejects_invalid_threshold(threshold: f64) {
        // Validate both the numeric type and the configuration transition before I/O.
        assert!(super::Threshold::try_from(threshold).is_err());
        assert!(matches!(
            super::State::configured(super::Config {
                coverage_path: None,
                threshold
            }),
            super::State::Failed(_)
        ));
    }

    /// Finite nonnegative thresholds include zero and the largest finite value.
    #[test_case::test_case(0.0; "zero")]
    #[test_case::test_case(f64::MAX; "largest finite value")]
    fn accepts_finite_threshold(threshold: f64) {
        assert!(super::Threshold::try_from(threshold).is_ok());
    }

    /// Default configuration keeps measured scoring disabled without filesystem work.
    #[test]
    fn default_scoring_stays_disabled() {
        assert!(matches!(
            super::State::configured(super::Config::default()).initialize(),
            super::State::Disabled
        ));
    }

    /// A loaded report retains its ready state when initialization is repeated.
    #[test]
    fn measured_report_stays_ready() {
        // Load the checked-in report through the pending configuration transition.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/coverage.info");
        let state = super::State::configured(super::Config {
            coverage_path: Some(path),
            threshold: 30.0,
        })
        .initialize();
        // Reinitializing ready state must retain its measured report.
        assert!(matches!(state.initialize(), super::State::Ready { .. }));
    }

    /// A report read failure stays terminal when initialization is repeated.
    #[test]
    fn missing_report_stays_failed() {
        // Enter failure through the same configured report load used by the compiler.
        let state = super::State::configured(super::Config {
            coverage_path: Some("missing-report".into()),
            threshold: 30.0,
        })
        .initialize();
        // Reinitialization must not turn a failed measurement into a ready score.
        assert!(matches!(state.initialize(), super::State::Failed(_)));
    }

    /// Preserve exact threshold equality and the original documented score examples.
    #[test_case::test_case(6, 0.0, 42.0; "uncovered above threshold")]
    #[test_case::test_case(5, 0.0, 30.0; "threshold equality")]
    #[test_case::test_case(35, 1.0, 35.0; "full coverage")]
    #[test_case::test_case(35, 0.0, 1260.0; "zero coverage")]
    #[test_case::test_case(35, 0.5, 188.125; "partial coverage")]
    #[test_case::test_case(1, 1.0, 1.0; "minimum complexity")]
    #[test_case::test_case(1, 0.0, 2.0; "simple code with no measured coverage")]
    fn formula_boundaries(complexity: u32, fraction: f64, expected: f64) {
        assert_eq!(score(complexity, fraction).to_bits(), expected.to_bits());
    }
}

/// Without measured input the compiler must not invent function coverage.
#[test]
fn ui_disabled() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "fixtures/disabled");
}

/// Equality with the configured limit and measured full coverage both pass.
#[cfg(test)]
#[test_case::test_case("equal", 42.0; "threshold equality")]
#[test_case::test_case("covered", 30.0; "measured full coverage")]
fn ui_passing_boundaries(directory: &str, threshold: f64) {
    // Select a report that belongs to this compiler fixture's own source.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("fixtures").join(directory).join("coverage.info");
    let coverage_path = toml_edit::Value::from(path.to_str().expect("UTF-8 fixture path"));
    let config =
        format!("[crap-score]\ncoverage_path = {coverage_path}\nthreshold = {threshold}\n");
    // Run each boundary separately so its compiler outcome has a named result.
    let fixture = format!("fixtures/{directory}");
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), &fixture)
        .dylint_toml(&config)
        .run();
}

/// Invalid configured scores fail compilation before function diagnostics are emitted.
#[cfg(test)]
#[test_case::test_case("-1.0"; "negative threshold")]
#[test_case::test_case("nan"; "NaN threshold")]
#[test_case::test_case("inf"; "infinite threshold")]
fn ui_invalid_threshold(threshold: &str) {
    // The measured report exists, so this failure belongs to threshold validation.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/coverage.info");
    let coverage_path = toml_edit::Value::from(path.to_str().expect("UTF-8 fixture path"));
    let config =
        format!("[crap-score]\ncoverage_path = {coverage_path}\nthreshold = {threshold}\n");
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "fixtures/invalid_threshold")
        .dylint_toml(&config)
        .run();
}
