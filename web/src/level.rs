//! Default lint levels reported by the runner bundle.

use std::{collections::BTreeMap, str::FromStr};

use strum::{Display, EnumString};

use crate::error::SiteError;

/// Default level of a lint as shown by the level filter.
///
/// `None` marks a documented lint that the runner bundle does not register.
#[derive(Clone, Copy, Debug, Display, EnumString, Eq, Ord, PartialEq, PartialOrd)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum LintLevel {
    /// Registered with the `allow` default level.
    Allow,
    /// Registered with the `warn` default level.
    Warn,
    /// Registered with the `deny` default level.
    Deny,
    /// Registered with the `forbid` default level.
    Forbid,
    /// Documented but absent from the runner's registered lints.
    None,
}

/// Lint names and default levels registered by the runner bundle.
#[derive(Debug, Default)]
pub(crate) struct RegisteredLints(BTreeMap<String, LintLevel>);

impl RegisteredLints {
    /// Look up a lint's registered level, or `None` when the bundle omits it.
    pub(crate) fn level(&self, name: &str) -> LintLevel {
        self.0.get(name).copied().unwrap_or(LintLevel::None)
    }

    /// Iterate over registered lint names in sorted order.
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

impl FromStr for RegisteredLints {
    type Err = SiteError;

    /// Parse `sagan-lints --list-private-lints` output.
    ///
    /// Lint rows are indented by four spaces and hold the name, the level, and a
    /// description. Category headings, command echoes, and timing lines are not
    /// indented and are skipped.
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        // Only indented lines are lint rows.
        let mut lints = BTreeMap::new();
        for line in source.lines().filter_map(|line| line.strip_prefix("    ")) {
            let row: RegistryRow = line.parse()?;
            // One lint registered twice would make its level ambiguous.
            if let Some(_previous) = lints.insert(row.name.clone(), row.level) {
                return Err(SiteError::DuplicateRegisteredLint { name: row.name });
            }
        }
        Ok(Self(lints))
    }
}

/// One lint row of the runner's lint table.
#[derive(Debug)]
struct RegistryRow {
    /// rustc lint name.
    name: String,
    /// Default level.
    level: LintLevel,
}

impl FromStr for RegistryRow {
    type Err = SiteError;

    /// Parse the leading name and level fields; the description is ignored.
    fn from_str(row: &str) -> Result<Self, Self::Err> {
        // The first two whitespace-separated fields are required.
        let mut fields = row.split_whitespace();
        let (Some(name), Some(level)) = (fields.next(), fields.next()) else {
            return Err(SiteError::InvalidLintListRow {
                row: row.to_owned(),
            });
        };

        // Keep the strum error as the source of an unknown level.
        let level = level
            .parse()
            .map_err(|source| SiteError::InvalidLintLevel {
                level: level.to_owned(),
                source,
            })?;
        Ok(Self {
            name: name.to_owned(),
            level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{LintLevel, RegisteredLints};
    use crate::error::SiteError;

    /// Parse runner output in tests.
    fn parse(source: &str) -> Result<RegisteredLints, SiteError> {
        source.parse()
    }

    /// Runner output parses into names and levels while skipping non-row lines.
    #[test]
    fn runner_rows_parse() {
        // Mix rows with the heading, command echo, and timing lines the runner prints.
        let source = "$ sagan-lints rustc -W help\nstyle\n    first_lint    warn    Does a thing\n    second_lint   allow   Does another\n\nTotal: 1.0s\n";
        let registered = parse(source).expect("runner rows should parse");

        // Listed lints keep their level; absent lints report `none`.
        assert_eq!(registered.level("first_lint"), LintLevel::Warn);
        assert_eq!(registered.level("second_lint"), LintLevel::Allow);
        assert_eq!(registered.level("absent_lint"), LintLevel::None);
    }

    /// Malformed, unknown-level, and duplicate rows are rejected independently.
    #[test]
    fn invalid_rows_are_rejected() {
        // Each input breaks exactly one row rule.
        let missing_level = parse("    lonely\n");
        let unknown_level = parse("    lint loud desc\n");
        let duplicate = parse("    lint warn a\n    lint deny b\n");

        // Each rule has its own error.
        assert!(matches!(
            missing_level,
            Err(SiteError::InvalidLintListRow { .. })
        ));
        assert!(matches!(
            unknown_level,
            Err(SiteError::InvalidLintLevel { .. })
        ));
        assert!(matches!(
            duplicate,
            Err(SiteError::DuplicateRegisteredLint { .. })
        ));
    }

    /// Every level spelling round-trips through its display form.
    #[test]
    fn levels_round_trip() {
        // Display and parsing must agree for every variant, including `none`.
        let levels = [
            LintLevel::Allow,
            LintLevel::Warn,
            LintLevel::Deny,
            LintLevel::Forbid,
            LintLevel::None,
        ];
        let parsed: Vec<_> = levels
            .iter()
            .map(|level| level.to_string().parse::<LintLevel>().ok())
            .collect();
        assert_eq!(parsed, levels.map(Some));
    }
}
