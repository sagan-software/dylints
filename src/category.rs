//! Closed lint-category vocabulary and static registration dispatch.

use clap::ValueEnum;
use strum::{Display, EnumString, IntoStaticStr};

/// Private lint groups embedded in the unified executable.
#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    EnumString,
    Eq,
    IntoStaticStr,
    Ord,
    PartialEq,
    PartialOrd,
    ValueEnum,
)]
#[strum(
    parse_err_ty = crate::category_parse_error::CategoryParseError,
    parse_err_fn = crate::category_parse_error::CategoryParseError::new
)]
pub(super) enum Category {
    /// Axum-specific lints.
    #[strum(serialize = "axum")]
    Axum,
    /// Bevy-specific lints.
    #[strum(serialize = "bevy")]
    Bevy,
    /// Cargo manifest and workspace lints.
    #[strum(serialize = "cargo")]
    Cargo,
    /// Clap-specific lints.
    #[strum(serialize = "clap")]
    Clap,
    /// Complexity lints.
    #[strum(serialize = "complexity")]
    Complexity,
    /// Correctness lints.
    #[strum(serialize = "correctness")]
    Correctness,
    /// General crate-specific lints.
    #[strum(serialize = "crates")]
    Crates,
    /// Insta-specific lints.
    #[strum(serialize = "insta")]
    Insta,
    /// Quantitative maintainability lints.
    #[strum(serialize = "maintainability")]
    Maintainability,
    /// Performance lints.
    #[strum(serialize = "perf")]
    Perf,
    /// Reqwest-specific lints.
    #[strum(serialize = "reqwest")]
    Reqwest,
    /// Restriction lints.
    #[strum(serialize = "restriction")]
    Restriction,
    /// Schemars-specific lints.
    #[strum(serialize = "schemars")]
    Schemars,
    /// Serde-specific lints.
    #[strum(serialize = "serde")]
    Serde,
    /// SQLx-specific lints.
    #[strum(serialize = "sqlx")]
    Sqlx,
    /// Style lints.
    #[strum(serialize = "style")]
    Style,
    /// Suspicious-code lints.
    #[strum(serialize = "suspicious")]
    Suspicious,
    /// Test-case-specific lints.
    #[strum(serialize = "test-case")]
    #[value(name = "test-case")]
    TestCase,
    /// Thiserror-specific lints.
    #[strum(serialize = "thiserror")]
    Thiserror,
    /// Tokio-specific lints.
    #[strum(serialize = "tokio")]
    Tokio,
    /// Tracing-specific lints.
    #[strum(serialize = "tracing")]
    Tracing,
}

/// Categories enabled when the caller does not request a narrower private suite.
pub(super) const DEFAULT_CATEGORIES: [Category; 9] = [
    Category::Cargo,
    Category::Complexity,
    Category::Correctness,
    Category::Crates,
    Category::Maintainability,
    Category::Perf,
    Category::Restriction,
    Category::Style,
    Category::Suspicious,
];

/// Function pointer used to register one statically linked lint group.
type RegisterLint = fn(&rustc_session::Session, &mut rustc_lint::LintStore);

/// Registration functions kept in the same order as the closed [`Category`] enum.
const REGISTER_LINTS: [RegisterLint; 21] = [
    axum_lints::register_lints,
    bevy_lints::register_lints,
    cargo_lints::register_lints,
    clap_lints::register_lints,
    complexity_lints::register_lints,
    correctness_lints::register_lints,
    crates_lints::register_lints,
    insta_lints::register_lints,
    maintainability_lints::register_lints,
    perf_lints::register_lints,
    reqwest_category_lints::register_lints,
    restriction_lints::register_lints,
    schemars_lints::register_lints,
    serde_lints::register_lints,
    sqlx_lints::register_lints,
    style_lints::register_lints,
    suspicious_lints::register_lints,
    test_case_lints::register_lints,
    thiserror_lints::register_lints,
    tokio_lints::register_lints,
    tracing_lints::register_lints,
];

impl Category {
    /// Return the stable command-line and process-boundary key.
    pub(super) fn key(self) -> &'static str {
        let key: &'static str = self.into();
        key
    }

    /// Register this embedded category with one rustc lint store.
    pub(super) fn register(
        self,
        session: &rustc_session::Session,
        lint_store: &mut rustc_lint::LintStore,
    ) {
        // Indexing preserves the closed enum order while keeping registration exhaustive.
        if let Some(register) = REGISTER_LINTS.get(self as usize) {
            register(session, lint_store);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::{Category, DEFAULT_CATEGORIES};

    #[test]
    fn every_category_key_round_trips() {
        // Enumerate the closed vocabulary so a new variant needs an explicit round-trip case.
        let categories = [
            Category::Axum,
            Category::Bevy,
            Category::Cargo,
            Category::Clap,
            Category::Complexity,
            Category::Correctness,
            Category::Crates,
            Category::Insta,
            Category::Maintainability,
            Category::Perf,
            Category::Reqwest,
            Category::Restriction,
            Category::Schemars,
            Category::Serde,
            Category::Sqlx,
            Category::Style,
            Category::Suspicious,
            Category::TestCase,
            Category::Thiserror,
            Category::Tokio,
            Category::Tracing,
        ];

        for category in categories {
            assert_eq!(Category::from_str(category.key()).ok(), Some(category));
        }
    }

    #[test]
    fn defaults_preserve_the_complete_general_suite_order() {
        // Preserve the user-visible default order independently of enum declaration order.
        assert_eq!(
            DEFAULT_CATEGORIES.map(Category::key),
            [
                "cargo",
                "complexity",
                "correctness",
                "crates",
                "maintainability",
                "perf",
                "restriction",
                "style",
                "suspicious",
            ]
        );
    }

    #[test]
    fn unknown_category_is_rejected() {
        let error = Category::from_str("unknown").err();
        assert_eq!(
            error.map(|error| error.to_string()),
            Some("unknown embedded lint category `unknown`".to_owned())
        );
    }
}
