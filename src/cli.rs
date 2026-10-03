//! Command-line vocabulary for the Sagan lint runner.

use std::{path::PathBuf, time::Duration};

use clap::{Parser, ValueEnum, builder::TypedValueParser as _};

use crate::category::Category;

/// Strict Sagan-lint command-line interface.
#[derive(Debug, Parser)]
#[command(version, about)]
pub(super) struct Cli {
    /// Target Rust repository or workspace root.
    #[arg(long, default_value = ".")]
    pub(super) repo: PathBuf,
    /// Cargo manifest, absolute or relative to the target repository.
    #[arg(long)]
    pub(super) manifest_path: Option<PathBuf>,
    /// Workspace package to lint; repeat for multiple packages.
    #[arg(short = 'p', long = "package")]
    pub(super) packages: Vec<String>,
    /// Workspace package to exclude; repeat for multiple packages.
    #[arg(long)]
    pub(super) exclude: Vec<String>,
    /// Select the whole workspace when no manifest or package is supplied.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) workspace: Option<Presence>,
    /// Disable implicit whole-workspace selection.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) no_workspace: Option<Presence>,
    /// Check all Cargo targets.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) all_targets: Option<Presence>,
    /// Disable all-target selection.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) no_all_targets: Option<Presence>,
    /// Exclude package dependencies from Clippy and private-lint analysis.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) no_deps: Option<Presence>,
    /// Use the low-disk routine mode: library and binary targets without debug artifacts.
    #[arg(
        long,
        num_args = 0,
        default_missing_value = "enabled",
        conflicts_with = "all_targets"
    )]
    pub(super) fast: Option<Presence>,
    /// Enable all Cargo features.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) all_features: Option<Presence>,
    /// Disable default Cargo features.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) no_default_features: Option<Presence>,
    /// Feature list passed to Cargo; repeat when needed.
    #[arg(long)]
    pub(super) features: Vec<String>,
    /// Cargo target triple.
    #[arg(long)]
    pub(super) target: Option<String>,
    /// Cargo target directory, resolved relative to the target repository.
    #[arg(long)]
    pub(super) target_dir: Option<PathBuf>,
    /// Require an unchanged Cargo lockfile.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) locked: Option<Presence>,
    /// Require Cargo frozen mode.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) frozen: Option<Presence>,
    /// Require Cargo offline mode.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) offline: Option<Presence>,
    /// Skip strict Clippy.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) skip_clippy: Option<Presence>,
    /// Skip the embedded private lints.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) skip_dylint: Option<Presence>,
    /// Run one embedded private-lint category; repeat when needed.
    #[arg(long)]
    pub(super) dylint_category: Vec<Category>,
    /// Include private checks that require target-repository policy files.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) include_repo_policy_lints: Option<Presence>,
    /// Cargo executable or whitespace-separated wrapper command.
    #[arg(long)]
    pub(super) cargo_cmd: Option<String>,
    /// Git revision range used to retain diagnostics on changed lines.
    #[arg(long)]
    pub(super) changed_range: Option<String>,
    /// Directory for complete phase logs.
    #[arg(long)]
    pub(super) log_dir: Option<PathBuf>,
    /// Print commands without running them.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) dry_run: Option<Presence>,
    /// Apply only compiler suggestions marked machine-applicable and repeat until stable.
    #[arg(
        long,
        num_args = 0,
        default_missing_value = "enabled",
        conflicts_with_all = ["list_private_lints", "changed_range"]
    )]
    pub(super) fix: Option<Presence>,
    /// List the embedded default private lints.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) list_private_lints: Option<Presence>,
    /// Let a target repository's Clippy configuration override personal defaults.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) use_repo_clippy_config: Option<Presence>,
    /// Disable the strict rustc lint additions.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) no_strict_rustc: Option<Presence>,
    /// Enable noisier optional Clippy lints.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) aggressive_clippy: Option<Presence>,
    /// Do not promote warnings to errors during strict Clippy.
    #[arg(long, num_args = 0, default_missing_value = "enabled")]
    pub(super) no_deny_warnings: Option<Presence>,
    /// Additional Clippy lint to warn on.
    #[arg(long)]
    pub(super) clippy_lint: Vec<String>,
    /// Clippy lint to allow for a reported partial pass.
    #[arg(long)]
    pub(super) allow_clippy_lint: Vec<String>,
    /// Additional rustc lint to warn on.
    #[arg(long)]
    pub(super) rustc_lint: Vec<String>,
    /// Additional argument before Cargo's rustc separator.
    #[arg(long)]
    pub(super) extra_cargo_arg: Vec<String>,
    /// Additional argument after Cargo's rustc separator.
    #[arg(long)]
    pub(super) extra_rustc_arg: Vec<String>,
    /// Progress heartbeat interval in seconds; zero disables heartbeats.
    #[arg(
        long = "heartbeat-seconds",
        default_value = "20",
        value_parser = clap::value_parser!(u64).map(Duration::from_secs)
    )]
    pub(super) heartbeat_interval: Duration,
    /// Write machine-readable phase timings to this path.
    #[arg(long)]
    pub(super) timings_json: Option<PathBuf>,
    /// Write Rust findings as a GitLab Code Quality report.
    #[arg(long)]
    pub(super) gitlab_code_quality: Option<PathBuf>,
}

impl Cli {
    /// Return whether implicit workspace selection remains enabled.
    pub(super) const fn is_workspace_selected(&self) -> bool {
        self.no_workspace.is_none()
    }

    /// Return whether all targets remain enabled.
    pub(super) const fn is_all_targets_selected(&self) -> bool {
        self.fast.is_none() && self.no_all_targets.is_none()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::Cli;
    use clap::Parser as _;

    #[test]
    fn parses_fast_mode() {
        let cli = Cli::try_parse_from(["sagan-lints", "--fast"]).unwrap();

        assert!(cli.fast.is_some());
        assert!(!cli.is_all_targets_selected());
    }

    #[test]
    fn parses_dependency_exclusion() {
        let cli = Cli::try_parse_from(["sagan-lints", "--no-deps"]).unwrap();

        assert!(cli.no_deps.is_some());
    }

    #[test]
    fn parses_external_target_directory() {
        // Preserve an absolute external target while avoiding a machine-specific fixture path.
        let temporary = tempfile::tempdir().unwrap();
        let target = temporary.path().join("sagan-lints-target");
        let target_text = target.to_str().unwrap();
        let cli = Cli::try_parse_from(["sagan-lints", "--target-dir", target_text]).unwrap();

        assert_eq!(cli.target_dir.as_deref(), Some(target.as_path()));
    }

    #[test]
    fn fast_mode_conflicts_with_all_targets() {
        let error = Cli::try_parse_from(["sagan-lints", "--fast", "--all-targets"]).err();
        assert_eq!(
            error.map(|error| error.kind()),
            Some(clap::error::ErrorKind::ArgumentConflict)
        );
    }

    #[test]
    fn parses_gitlab_code_quality_report_path() {
        // Exercise the public parser boundary with a repository-relative report path.
        let cli = Cli::try_parse_from([
            "sagan-lints",
            "--gitlab-code-quality",
            "gl-code-quality-report.json",
        ])
        .expect("the report path should be accepted");

        // Preserve the path spelling for the later filesystem boundary.
        assert_eq!(
            cli.gitlab_code_quality,
            Some(PathBuf::from("gl-code-quality-report.json"))
        );
    }
}

/// Presence marker for a command-line switch with no value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum Presence {
    /// The switch was supplied.
    Enabled,
}
