//! Closed lint-category vocabulary derived from the repository layout.

use std::path::Path;

use strum::{Display, EnumString, IntoStaticStr};

use crate::error::SiteError;

/// Public description and upstream documentation link for one crate-specific
/// lint group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CrateGroup {
    /// Canonical project or crate spelling.
    pub(crate) name: &'static str,
    /// Official project or API documentation.
    #[expect(
        url_string_field,
        reason = "These immutable catalog URLs are trusted source metadata, not parsed user input."
    )]
    pub(crate) url: &'static str,
    /// The domain-specific checks collected by this group.
    pub(crate) description: &'static str,
}

/// Descriptions and canonical upstream links for crate-specific lint groups.
const CRATE_GROUPS: [(LintCategory, CrateGroup); 13] = [
    (
        LintCategory::Axum,
        CrateGroup {
            name: "axum",
            url: "https://github.com/tokio-rs/axum",
            description: "router paths, nesting and service configuration for the Axum web framework",
        },
    ),
    (
        LintCategory::Bevy,
        CrateGroup {
            name: "Bevy",
            url: "https://bevy.org/",
            description: "ECS queries, systems, schedules and selected engine API usage",
        },
    ),
    (
        LintCategory::Clap,
        CrateGroup {
            name: "clap",
            url: "https://docs.rs/clap/latest/clap/",
            description: "derive attributes and command-line argument configuration",
        },
    ),
    (
        LintCategory::Insta,
        CrateGroup {
            name: "insta",
            url: "https://insta.rs/",
            description: "snapshot assertions, filters and snapshot file handling",
        },
    ),
    (
        LintCategory::Reqwest,
        CrateGroup {
            name: "reqwest",
            url: "https://github.com/seanmonstar/reqwest",
            description: "HTTP client construction, request loops, retries and TLS settings",
        },
    ),
    (
        LintCategory::Schemars,
        CrateGroup {
            name: "Schemars",
            url: "https://github.com/GREsau/schemars",
            description: "derives and schema metadata for generating JSON Schema",
        },
    ),
    (
        LintCategory::Serde,
        CrateGroup {
            name: "Serde",
            url: "https://serde.rs/",
            description: "serialization and deserialization attributes and round-trip behavior",
        },
    ),
    (
        LintCategory::Sqlx,
        CrateGroup {
            name: "SQLx",
            url: "https://github.com/launchbadge/sqlx",
            description: "query-builder use, row access and connection-pool settings",
        },
    ),
    (
        LintCategory::Strum,
        CrateGroup {
            name: "strum",
            url: "https://github.com/Peternator7/strum",
            description: "enum representation and derive behavior",
        },
    ),
    (
        LintCategory::TestCase,
        CrateGroup {
            name: "test-case",
            url: "https://github.com/frondeus/test-case",
            description: "parameterized test declarations and case matrices",
        },
    ),
    (
        LintCategory::Thiserror,
        CrateGroup {
            name: "thiserror",
            url: "https://github.com/dtolnay/thiserror",
            description: "error derives, source fields and display formatting",
        },
    ),
    (
        LintCategory::Tokio,
        CrateGroup {
            name: "Tokio",
            url: "https://tokio.rs/",
            description: "runtime, task, channel and blocking-call patterns",
        },
    ),
    (
        LintCategory::Tracing,
        CrateGroup {
            name: "tracing",
            url: "https://github.com/tokio-rs/tracing",
            description: "spans, fields and instrumentation",
        },
    ),
];

/// Closed set of lint categories represented by each lint crate's manifest.
///
/// The kebab-case spelling is the category metadata value and filter key.
/// Variants are declared alphabetically so their ordering matches the filters.
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

impl LintCategory {
    /// Domain-specific documentation for a crate group, if this is one.
    pub(crate) fn crate_group(self) -> Option<&'static CrateGroup> {
        CRATE_GROUPS
            .iter()
            .find_map(|(category, group)| (*category == self).then_some(group))
    }

    /// Parse the declared category, retaining the manifest path for diagnostics.
    pub(crate) fn from_metadata(value: &str, path: &Path) -> Result<Self, SiteError> {
        value
            .parse::<Self>()
            .map_err(|_| SiteError::UnknownCategory {
                path: path.to_path_buf(),
            })
    }

    /// Stable filter and badge key for the category.
    pub(crate) fn key(self) -> &'static str {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{CRATE_GROUPS, LintCategory};

    /// Resolve a category from its manifest value.
    fn category(value: &str) -> Option<LintCategory> {
        LintCategory::from_metadata(value, Path::new("crates/example/Cargo.toml")).ok()
    }

    /// Repository-wide and crate-specific categories both parse from metadata.
    #[test]
    fn layouts_map_to_categories() {
        assert_eq!(category("test-case"), Some(LintCategory::TestCase));
        assert_eq!(category("perf"), Some(LintCategory::Perf));
    }

    /// Unknown names and families in the wrong position are rejected.
    #[test]
    fn misplaced_or_unknown_names_are_rejected() {
        assert_eq!(category("unknown"), None);
    }

    /// Keys are lowercase kebab case.
    #[test]
    fn keys_are_kebab_case() {
        // The filter key and display form share one spelling.
        assert_eq!(LintCategory::TestCase.key(), "test-case");
        assert_eq!(LintCategory::Perf.to_string(), "perf");
    }

    /// Every crate-specific category has a useful explanation and official link.
    #[test]
    #[expect(
        many_assertions_in_test,
        reason = "Each assertion checks a distinct category identity, link, or domain description."
    )]
    fn crate_groups_have_complete_documentation() {
        // The full list catches categories accidentally omitted from the generated catalog.
        let groups: Vec<_> = [
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
        ]
        .into_iter()
        .map(|category| category.crate_group().expect("crate group documentation"))
        .collect();
        // Canonical crate spellings and the Schemars domain text are public documentation.
        assert_eq!(groups.len(), CRATE_GROUPS.len());
        assert!(groups.iter().all(|group| {
            group.url.starts_with("https://")
                && !group.description.is_empty()
                && !group.name.is_empty()
        }));
        assert_eq!(groups[5].name, "Schemars");
        assert!(groups[5].description.contains("JSON Schema"));
        assert_eq!(groups[7].name, "SQLx");
    }
}
