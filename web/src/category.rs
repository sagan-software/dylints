//! Closed lint-category vocabulary derived from the repository layout.

use std::path::{Component, Path};

use strum::{Display, EnumString, IntoStaticStr};

use crate::error::SiteError;

/// Closed set of lint categories represented by the repository layout.
///
/// The kebab-case spelling of each variant is both its directory name and its
/// filter key. Variants are declared alphabetically so the derived ordering
/// matches the group filter order.
#[derive(
    Clone, Copy, Debug, Display, EnumString, Eq, IntoStaticStr, Ord, PartialEq, PartialOrd,
)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum LintCategory {
    /// Axum crate APIs.
    Axum,
    /// Bevy crate APIs.
    Bevy,
    /// Cargo project structure and metadata.
    Cargo,
    /// clap crate APIs.
    Clap,
    /// Unnecessarily complicated code.
    Complexity,
    /// Incorrect or failure-prone behavior.
    Correctness,
    /// Insta crate APIs.
    Insta,
    /// Quantitative complexity and coupling limits.
    Maintainability,
    /// Runtime and allocation performance.
    Perf,
    /// Reqwest crate APIs.
    Reqwest,
    /// Deliberately restrictive project policy.
    Restriction,
    /// Schemars crate APIs.
    Schemars,
    /// Serde crate APIs.
    Serde,
    /// `SQLx` crate APIs.
    Sqlx,
    /// Strum crate APIs.
    Strum,
    /// Code style.
    Style,
    /// Suspicious constructs.
    Suspicious,
    /// test-case crate APIs.
    TestCase,
    /// thiserror crate APIs.
    Thiserror,
    /// Tokio crate APIs.
    Tokio,
    /// tracing crate APIs.
    Tracing,
}

/// Categories whose lints live below `lints/crates/<family>/`.
const CRATE_FAMILIES: [LintCategory; 13] = [
    LintCategory::Axum,
    LintCategory::Bevy,
    LintCategory::Clap,
    LintCategory::Insta,
    LintCategory::Reqwest,
    LintCategory::Schemars,
    LintCategory::Serde,
    LintCategory::Sqlx,
    LintCategory::Strum,
    LintCategory::TestCase,
    LintCategory::Thiserror,
    LintCategory::Tokio,
    LintCategory::Tracing,
];

impl LintCategory {
    /// Derive a category from a lint crate's path below `lints/`.
    ///
    /// Repository-wide categories are the first component; crate families are the
    /// second component below `crates`. A family name in the wrong position is
    /// rejected so the layout stays unambiguous.
    pub(crate) fn from_relative_path(path: &Path) -> Result<Self, SiteError> {
        // Read the first two normal components as UTF-8 directory names.
        let mut names = path.components().filter_map(|component| match component {
            Component::Normal(name) => name.to_str(),
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => None,
        });
        let first = names.next();
        let second = names.next();

        // Pick the directory that names the category, then require the matching layout.
        let is_crate_family_path = first == Some("crates");
        let category = if is_crate_family_path { second } else { first }
            .and_then(|name| name.parse::<Self>().ok())
            .filter(|category| category.is_crate_family() == is_crate_family_path);
        category.ok_or_else(|| SiteError::UnknownCategory {
            path: path.to_path_buf(),
        })
    }

    /// Stable filter and badge key for the category.
    pub(crate) fn key(self) -> &'static str {
        self.into()
    }

    /// Report whether the category groups lints for one external crate.
    fn is_crate_family(self) -> bool {
        CRATE_FAMILIES.contains(&self)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::LintCategory;

    /// Resolve a category from a path below `lints/`.
    fn category(path: &str) -> Option<LintCategory> {
        LintCategory::from_relative_path(Path::new(path)).ok()
    }

    /// Repository-wide and crate-family layouts both map to their category.
    #[test]
    fn layouts_map_to_categories() {
        // A crate family is the second component below `crates`.
        assert_eq!(
            category("crates/test-case/test-case-empty-matrix"),
            Some(LintCategory::TestCase)
        );
        // A repository-wide category is the first component.
        assert_eq!(
            category("perf/boxed_future_return"),
            Some(LintCategory::Perf)
        );
    }

    /// Unknown names and families in the wrong position are rejected.
    #[test]
    fn misplaced_or_unknown_names_are_rejected() {
        // Unknown families and categories in the wrong layout position are rejected.
        assert_eq!(category("crates/unknown/lint"), None);
        assert_eq!(category("crates/style/lint"), None);
        assert_eq!(category("serde/lint"), None);
    }

    /// Keys are lowercase kebab case.
    #[test]
    fn keys_are_kebab_case() {
        // The filter key and display form share one spelling.
        assert_eq!(LintCategory::TestCase.key(), "test-case");
        assert_eq!(LintCategory::Perf.to_string(), "perf");
    }
}
