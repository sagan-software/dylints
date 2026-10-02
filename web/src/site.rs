//! Site generation: render the catalog and write it with its static assets.

use std::{
    fs,
    io::{self, Write as _},
    path::Path,
};

use askama::Template as _;

use crate::{
    catalog::{Lint, SiteTemplate, read_lints},
    error::SiteError,
    level::{LintLevel, RegisteredLints},
};

/// Static files copied next to the generated page, keyed by output file name.
pub(crate) const STATIC_ASSETS: [(&str, &str); 4] = [
    ("style.css", include_str!("../static/style.css")),
    ("script.js", include_str!("../static/script.js")),
    ("theme.js", include_str!("../static/theme.js")),
    (
        "LICENSE-CLIPPY-MIT",
        include_str!("../static/LICENSE-CLIPPY-MIT"),
    ),
];

/// Discover lint documentation and write the rendered site into `out_dir`.
pub(crate) fn generate_site(
    root: &Path,
    registered: &RegisteredLints,
    out_dir: &Path,
) -> Result<Vec<Lint>, SiteError> {
    // Render everything before touching the destination.
    let (lints, rendered_html) = render_site(root, registered)?;
    write_site(out_dir, &rendered_html).map(|()| lints)
}

/// Read the lints under `root` and render the page.
fn render_site(
    root: &Path,
    registered: &RegisteredLints,
) -> Result<(Vec<Lint>, String), SiteError> {
    // Canonicalize so source links are relative to the real repository root.
    let root = fs::canonicalize(root).map_err(|source| io_error(root, source))?;
    let lints = read_lints(&root, registered)?;
    let rendered_html = SiteTemplate::new(&lints).render()?;
    Ok((lints, rendered_html))
}

/// Write the static assets and then the page into `out_dir`.
fn write_site(out_dir: &Path, rendered_html: &str) -> Result<(), SiteError> {
    // Write the page last so a partial run never leaves a page without its assets.
    fs::create_dir_all(out_dir).map_err(|source| io_error(out_dir, source))?;
    STATIC_ASSETS
        .iter()
        .try_for_each(|(name, contents)| write_file(&out_dir.join(name), contents))?;
    write_file(&out_dir.join("index.html"), rendered_html)
}

/// Write one output file with path-aware diagnostics.
fn write_file(path: &Path, contents: &str) -> Result<(), SiteError> {
    fs::write(path, contents).map_err(|source| io_error(path, source))
}

/// Attach a path to an I/O error.
fn io_error(path: &Path, source: io::Error) -> SiteError {
    SiteError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Describe the generated page and name documented lints that the runner does
/// not register.
pub(crate) fn report(index: &Path, lints: &[Lint]) -> (String, Option<String>) {
    // Unregistered lints still render, so name them for the maintainer.
    let unregistered: Vec<_> = lints
        .iter()
        .filter(|lint| lint.level == LintLevel::None)
        .map(|lint| lint.name.as_str())
        .collect();

    // Warn only when at least one lint is unregistered.
    let warning = (!unregistered.is_empty()).then(|| {
        let names = unregistered.join(", ");
        format!("warning: documented lints are not registered by the runner: {names}\n")
    });

    // The summary names the page and the number of documented lints.
    let index_path = index.display();
    let lint_count = lints.len();
    (
        format!("Generated {index_path} from {lint_count} lint README files\n"),
        warning,
    )
}

/// Write the report to the standard streams.
pub(crate) fn print_report(summary: &str, warning: Option<&str>) -> io::Result<()> {
    // Warnings go to stderr so stdout carries only the summary.
    if let Some(warning) = warning {
        io::stderr().lock().write_all(warning.as_bytes())?;
    }
    io::stdout().lock().write_all(summary.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{STATIC_ASSETS, generate_site, report};
    use crate::catalog::tests::{fixture_registry, fixture_repository};

    /// Site generation writes the page and every static asset.
    #[test]
    fn writes_page_and_assets() {
        // Generate into a fresh directory inside the fixture.
        let root = fixture_repository();
        let out_dir = root.path().join("public");
        let lints = generate_site(root.path(), &fixture_registry(), &out_dir)
            .expect("site generation should succeed");
        assert_eq!(lints.len(), 2);

        // Every asset and the page must exist afterward.
        let names = STATIC_ASSETS
            .iter()
            .map(|(name, _)| *name)
            .chain(["index.html"]);
        assert!(names.into_iter().all(|name| out_dir.join(name).is_file()));
    }

    /// The report counts lints and warns about the unregistered fixture lint.
    #[test]
    fn reports_unregistered_lints() {
        // Generate the fixture site to get its lints.
        let root = fixture_repository();
        let lints = generate_site(
            root.path(),
            &fixture_registry(),
            &root.path().join("public"),
        )
        .expect("site generation should succeed");

        // The summary counts both lints; the warning names the unregistered one.
        let (summary, warning) = report(Path::new("public/index.html"), &lints);
        assert_eq!(
            summary,
            "Generated public/index.html from 2 lint README files\n"
        );
        assert_eq!(
            warning.as_deref(),
            Some(
                "warning: documented lints are not registered by the runner: serde_unregistered\n"
            )
        );
    }

    /// A missing repository root is reported with its path.
    #[test]
    fn missing_root_is_reported() {
        // Point generation at a root that does not exist.
        let root = fixture_repository();
        let missing = root.path().join("missing");
        let result = generate_site(&missing, &fixture_registry(), &root.path().join("public"));
        assert!(result.is_err_and(|error| error.to_string().contains("missing")));
    }
}
