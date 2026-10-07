#![feature(rustc_private)]

//! A lint to check for workspace packages with local dependency sources.
//!
//! It parses the nearest package manifest above the crate root with
//! `toml_edit`, finds its Cargo workspace root, and checks the package's
//! dependency tables only when that root declares `[workspace.dependencies]`.
//! Explicit versions, paths and Git sources are reported at their source values.

extern crate rustc_ast;

use cargo_support::{
    Manifest, dependencies, dependency_tables, emit_with_help, has_workspace_entry,
    package_manifest,
};
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub WORKSPACE_DEPENDENCY_VERSIONS,
    Warn,
    "workspace package dependencies should inherit versions from workspace dependencies",
    WorkspaceDependencyVersions
}

impl EarlyLintPass for WorkspaceDependencyVersions {
    /// Check the crate's package manifest for dependencies with local sources.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        if let Some(package) = package_manifest(cx, krate) {
            check_package(cx, &package);
        }
    }
}

/// Report every dependency of a workspace package that sets its own source.
///
/// Cargo rejects dependency tables in a virtual manifest, so every manifest
/// with dependencies here is a package manifest.
fn check_package(cx: &EarlyContext<'_>, package: &Manifest) {
    // Without `[workspace.dependencies]` there is nothing to inherit from.
    if !has_workspace_entry(cx, package, "dependencies") {
        return;
    }

    // Dependency sources belong to the workspace even when no version is declared.
    for table in dependency_tables(package.root()) {
        for dependency in dependencies(table) {
            let Some((source, range)) = explicit_source(dependency) else {
                continue;
            };

            let name = dependency.name;
            emit_with_help(
                cx,
                WORKSPACE_DEPENDENCY_VERSIONS,
                package.span(range),
                &format!(
                    "dependency `{name}` specifies an explicit {source} instead of inheriting from `[workspace.dependencies]`"
                ),
                &format!(
                    "move the {source} for `{name}` into `[workspace.dependencies]` and use `{name}.workspace = true` here"
                ),
            );
        }
    }
}

/// Locate an explicit dependency source, preferring the existing version diagnostic.
fn explicit_source(
    dependency: cargo_support::Dependency<'_>,
) -> Option<(&'static str, std::ops::Range<usize>)> {
    // Preserve version diagnostics for shorthand and version-plus-path declarations.
    if let Some((_, range)) = dependency.version() {
        return Some(("version", range));
    }
    // Path and Git sources require inheritance even without a registry version.
    let fields = dependency.item.as_table_like()?;
    for source in ["path", "git"] {
        if let Some(value) = fields.get(source).filter(|value| value.is_str()) {
            return Some((source, value.span()?));
        }
    }
    None
}

/// Run the UI tests.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::explicit_source;
    /// Each source form uses its actual TOML value span, with version taking precedence.
    #[test_case::test_case("\"1.0.0\"", Some("version"); "registry version")]
    #[test_case::test_case("{ path = \"../local\" }", Some("path"); "local path")]
    #[test_case::test_case("{ git = \"https://example.invalid/repo\" }", Some("git"); "Git source")]
    #[test_case::test_case("{ version = \"1.0.0\", path = \"../local\" }", Some("version"); "version precedence")]
    #[test_case::test_case("{ workspace = true, features = [\"std\"] }", None; "workspace inheritance")]
    #[test_case::test_case("{ path = 42, git = false }", None; "nontext sources")]
    #[test_case::test_case("false", None; "nontable dependency")]
    fn dependency_source_boundaries(value: &str, expected: Option<&str>) {
        // Retain the original document so source values keep their measured spans.
        let source = format!("[dependencies]\nlocal = {value}\n");
        let document: toml_edit::Document<String> = source.parse().expect("fixture TOML");
        let table = document
            .get("dependencies")
            .and_then(toml_edit::Item::as_table_like)
            .expect("dependency table");
        let dependency = cargo_support::dependencies(table)
            .next()
            .expect("one dependency");
        // Each detected source must identify its quoted TOML value, not the dependency key.
        let found = explicit_source(dependency);
        assert_eq!(found.as_ref().map(|(name, _)| *name), expected);
        if let Some((_, span)) = found {
            assert!(source.get(span).is_some_and(|value| value.starts_with('"')));
        }
    }
}
