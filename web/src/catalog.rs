//! Lint discovery and the typed catalog page model.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use walkdir::{DirEntry, WalkDir};

use crate::{
    applicability::{Applicability, SuggestingMacros},
    category::LintCategory,
    error::SiteError,
    level::{LintLevel, RegisteredLints},
    markdown::render_markdown,
    readme::Readme,
};

/// Public repository that hosts the lint sources and issue tracker.
pub(crate) const REPOSITORY_URL: &str = "https://github.com/sagan-software/dylints";

/// One lint and its rendered README.
#[derive(Debug)]
pub(crate) struct Lint {
    /// rustc lint name used for display, anchors, and copying.
    pub(crate) name: String,
    /// Semantic category derived from the source path.
    pub(crate) category: LintCategory,
    /// Default level registered by the runner bundle.
    pub(crate) level: LintLevel,
    /// Whether the lint emits machine-applicable suggestions.
    pub(crate) applicability: Applicability,
    /// Repository-relative path of the lint's `src/lib.rs`.
    source_path: PathBuf,
    /// Rendered Markdown body, excluding the title.
    pub(crate) documentation_html: String,
}

impl Lint {
    /// Repository-relative source path with `/` separators for links.
    pub(crate) fn source_href(&self) -> String {
        self.source_path
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// Askama context for the lint list page.
#[derive(askama::Template)]
#[template(path = "index.html")]
pub(crate) struct SiteTemplate<'lint> {
    /// Lints in name order.
    lints: &'lint [Lint],
    /// Levels present in the catalog, in severity order.
    levels: Vec<LintLevel>,
    /// Categories present in the catalog, in key order.
    groups: Vec<LintCategory>,
    /// Applicabilities present in the catalog.
    applicabilities: Vec<Applicability>,
}

impl<'lint> SiteTemplate<'lint> {
    /// Build the page context and derive each filter vocabulary from the lints.
    pub(crate) fn new(lints: &'lint [Lint]) -> Self {
        Self {
            lints,
            levels: present(lints.iter().map(|lint| lint.level)),
            groups: present(lints.iter().map(|lint| lint.category)),
            applicabilities: present(lints.iter().map(|lint| lint.applicability)),
        }
    }
}

/// Collect the distinct values of one filter dimension in their declared order.
fn present<T: Ord>(values: impl Iterator<Item = T>) -> Vec<T> {
    let mut values: Vec<_> = values.collect();
    values.sort_unstable();
    values.dedup();
    values
}

/// Discover and render every lint README below `lints/`.
pub(crate) fn read_lints(
    root: &Path,
    registered: &RegisteredLints,
) -> Result<Vec<Lint>, SiteError> {
    // Find the crates first, then read them with shared registry and macro data.
    let lints_root = root.join("lints");
    let ui_directories = lint_ui_directories(&lints_root)?;
    let context = Context {
        root,
        registered,
        macros: suggesting_macros(&lints_root)?,
    };

    // Every registered lint must have a README among the discovered crates.
    let lints = context.read_all(&ui_directories)?;
    require_documented(&lints, registered).map(|()| lints)
}

/// Find each lint crate's `ui` fixture directory, rejecting an empty tree.
fn lint_ui_directories(lints_root: &Path) -> Result<Vec<PathBuf>, SiteError> {
    let ui_directories = directories_named(lints_root, "ui")?;
    if ui_directories.is_empty() {
        return Err(SiteError::NoLints {
            path: lints_root.to_path_buf(),
        });
    }
    Ok(ui_directories)
}

/// Reject a registry that names a lint without a README.
fn require_documented(lints: &[Lint], registered: &RegisteredLints) -> Result<(), SiteError> {
    // `lints` is sorted by name, so each registered name is one binary search.
    let undocumented: Vec<_> = registered
        .names()
        .filter(|name| {
            lints
                .binary_search_by(|lint| lint.name.as_str().cmp(name))
                .is_err()
        })
        .collect();

    // Name every missing README at once so one run reports the whole gap.
    if undocumented.is_empty() {
        return Ok(());
    }
    Err(SiteError::UndocumentedLints {
        names: undocumented.join(", "),
    })
}

/// Shared inputs for reading each lint crate.
struct Context<'input> {
    /// Canonical repository root.
    root: &'input Path,
    /// Default levels from the runner.
    registered: &'input RegisteredLints,
    /// Support macros that emit machine-applicable suggestions.
    macros: SuggestingMacros,
}

impl Context<'_> {
    /// Read every lint crate and sort the results by lint name.
    fn read_all(&self, ui_directories: &[PathBuf]) -> Result<Vec<Lint>, SiteError> {
        let mut lints = ui_directories
            .iter()
            .map(|ui_directory| self.read_lint(ui_directory))
            .collect::<Result<Vec<_>, _>>()?;
        lints.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(lints)
    }

    /// Parse one lint crate into the typed template model.
    fn read_lint(&self, ui_directory: &Path) -> Result<Lint, SiteError> {
        // The crate directory is the parent of its UI fixtures.
        let lint_directory = ui_directory.parent().unwrap_or(ui_directory);
        let document = read_to_string(&lint_directory.join("README.md"))?;
        let readme = Readme::parse(&document, lint_directory)?;
        let relative = strip_prefix(lint_directory, self.root)?;
        self.build_lint(&readme, relative)
    }

    /// Join the README with the runner registry and the lint's suggestion evidence.
    fn build_lint(&self, readme: &Readme, relative: &Path) -> Result<Lint, SiteError> {
        // The category comes from the crate's position below `lints/`.
        let category =
            LintCategory::from_relative_path(strip_prefix(relative, Path::new("lints"))?)?;

        // Suggestion evidence comes from the crate sources and its expected UI output.
        let lint_directory = self.root.join(relative);
        let source = concatenated_files(&lint_directory.join("src"), "rs", usize::MAX)?;
        let ui_stderr = concatenated_files(&lint_directory.join("ui"), "stderr", 1)?;

        // The registry is keyed by the rustc spelling of the lint name.
        let name = readme.id.lint_name();
        Ok(Lint {
            level: self.registered.level(&name),
            applicability: self.macros.applicability(&source, &ui_stderr),
            name,
            category,
            source_path: relative.join("src").join("lib.rs"),
            documentation_html: render_markdown(&readme.body),
        })
    }
}

/// Collect suggesting macros from every family `support/src` directory below
/// `lints_root`.
fn suggesting_macros(lints_root: &Path) -> Result<SuggestingMacros, SiteError> {
    // Support crates are named `support` and keep their sources in `src`.
    let mut sources = String::new();
    for directory in directories_named(lints_root, "src")? {
        // Only a `src` directory directly inside a `support` crate qualifies.
        let is_support =
            directory.parent().and_then(Path::file_name) == Some(OsStr::new("support"));
        if is_support {
            sources.push_str(&concatenated_files(&directory, "rs", usize::MAX)?);
        }
    }
    Ok(SuggestingMacros::from(sources.as_str()))
}

/// Find every directory below `root` with exactly this name, in path order.
fn directories_named(root: &Path, name: &str) -> Result<Vec<PathBuf>, SiteError> {
    // Do not follow links, because repository links could escape the lint tree.
    let entries = walk(root, usize::MAX)?;
    Ok(entries
        .into_iter()
        .filter(|entry| entry.file_type().is_dir() && entry.file_name() == OsStr::new(name))
        .map(DirEntry::into_path)
        .collect())
}

/// Concatenate the files with one extension below `directory`, in path order.
fn concatenated_files(
    directory: &Path,
    extension: &str,
    max_depth: usize,
) -> Result<String, SiteError> {
    // Path order keeps the concatenation deterministic across platforms.
    let mut contents = String::new();
    for entry in walk(directory, max_depth)? {
        // Skip directories and files of other types.
        let has_extension = entry.path().extension() == Some(OsStr::new(extension));
        if has_extension {
            contents.push_str(&read_to_string(entry.path())?);
            contents.push('\n');
        }
    }
    Ok(contents)
}

/// Walk a tree in file-name order without following links.
fn walk(root: &Path, max_depth: usize) -> Result<Vec<DirEntry>, SiteError> {
    WalkDir::new(root)
        .follow_links(false)
        .max_depth(max_depth)
        .sort_by_file_name()
        .into_iter()
        .collect::<Result<_, _>>()
        .map_err(|source| SiteError::Walk {
            path: root.to_path_buf(),
            source,
        })
}

/// Read one UTF-8 file with path-aware diagnostics.
pub(crate) fn read_to_string(path: &Path) -> Result<String, SiteError> {
    fs::read_to_string(path).map_err(|source| SiteError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Reduce a path to its suffix below `root`.
fn strip_prefix<'path>(path: &'path Path, root: &Path) -> Result<&'path Path, SiteError> {
    path.strip_prefix(root)
        .map_err(|source| SiteError::PathOutsideRoot {
            path: path.to_path_buf(),
            root: root.to_path_buf(),
            source,
        })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{fs, path::Path};

    use askama::Template as _;

    use super::{SiteTemplate, read_lints};
    use crate::{
        applicability::Applicability, error::SiteError, level::LintLevel, level::RegisteredLints,
    };

    /// README body that satisfies the shared section contract.
    const README_BODY: &str = "## What it does\n\nChecks things.\n\n## Why is this bad?\n\nReasons.\n\n## Known problems\n\nNone.\n\n## Example\n\n```rust\nlet value = 1;\n```\n\n## Use instead\n\n```rust\nlet value = 2;\n```\n";

    /// Resolve the repository root from the web crate.
    pub(crate) fn repository_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| Path::new("."))
    }

    /// Write one lint crate with a README, UI expectation, and library source.
    fn write_lint(root: &Path, directory: &str, library: &str, ui_stderr: &str) {
        // The crate directory name doubles as the README title.
        let lint = root.join("lints").join(directory);
        let title = lint
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();

        // Create the directories that discovery and evidence collection read.
        fs::create_dir_all(lint.join("ui")).expect("fixture UI directory should be writable");
        fs::create_dir_all(lint.join("src")).expect("fixture source directory should be writable");

        // Write the README, library source, and expected UI output.
        fs::write(
            lint.join("README.md"),
            format!("# {title}\n\n{README_BODY}"),
        )
        .expect("fixture README should be writable");
        fs::write(lint.join("src/lib.rs"), library).expect("fixture source should be writable");
        fs::write(lint.join("ui/main.stderr"), ui_stderr)
            .expect("fixture UI output should be writable");
    }

    /// Build a two-lint repository with one fixable lint and one help-only lint.
    pub(crate) fn fixture_repository() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("temporary directory should be available");
        write_lint(
            root.path(),
            "style/fixable_style",
            "fn fix() { let _ = Applicability::MachineApplicable; }\n",
            "   |     ^^^ help: fix it\n",
        );
        write_lint(
            root.path(),
            "crates/serde/serde-unregistered",
            "fn check() {}\n",
            "   = help: rewrite it\n",
        );
        root
    }

    /// Parse a registry that registers only the fixable fixture lint.
    pub(crate) fn fixture_registry() -> RegisteredLints {
        "    fixable_style    warn    Fixable\n"
            .parse()
            .expect("fixture lint list should parse")
    }

    /// A small repository renders the complete Clippy-style page.
    #[test]
    fn fixture_catalog_snapshot() {
        // Read the fixture through the production discovery path.
        let root = fixture_repository();
        let lints =
            read_lints(root.path(), &fixture_registry()).expect("fixture lints should load");

        // Reduce each lint to the metadata that drives the filters.
        let summary: Vec<_> = lints
            .iter()
            .map(|lint| (lint.name.as_str(), lint.level, lint.applicability))
            .collect();

        // The unregistered, help-only lint renders as `none` and not machine applicable.
        assert_eq!(
            summary,
            [
                (
                    "fixable_style",
                    LintLevel::Warn,
                    Applicability::MachineApplicable
                ),
                (
                    "serde_unregistered",
                    LintLevel::None,
                    Applicability::NotMachineApplicable
                ),
            ]
        );

        // Snapshot the complete page so markup changes are reviewed.
        let html = SiteTemplate::new(&lints)
            .render()
            .expect("Askama template should render");
        insta::assert_snapshot!("fixture_catalog", html);
    }

    /// A registered lint without a README stops generation.
    #[test]
    fn undocumented_registered_lint_is_rejected() {
        // Register a lint that has no crate in the fixture tree.
        let root = fixture_repository();
        let registered: RegisteredLints = "    missing_readme    warn    Missing\n"
            .parse()
            .expect("fixture lint list should parse");

        // Generation must name the undocumented lint.
        let result = read_lints(root.path(), &registered);
        assert!(
            matches!(&result, Err(SiteError::UndocumentedLints { names }) if names == "missing_readme"),
            "{result:?}"
        );
    }

    /// A tree without lint crates is rejected.
    #[test]
    fn empty_tree_is_rejected() {
        // An existing but empty lint tree has no UI directories.
        let root = tempfile::tempdir().expect("temporary directory should be available");
        fs::create_dir_all(root.path().join("lints")).expect("lint tree should be writable");

        // Discovery must refuse to render an empty catalog.
        let result = read_lints(root.path(), &RegisteredLints::default());
        assert!(
            matches!(result, Err(SiteError::NoLints { .. })),
            "{result:?}"
        );
    }

    /// A lint crate outside every known category is rejected.
    #[test]
    fn unknown_category_is_rejected() {
        // Place a valid crate under a directory that names no category.
        let root = tempfile::tempdir().expect("temporary directory should be available");
        write_lint(root.path(), "unknown/odd_lint", "fn check() {}\n", "");

        // Discovery must reject the crate rather than guess its category.
        let result = read_lints(root.path(), &RegisteredLints::default());
        assert!(
            matches!(result, Err(SiteError::UnknownCategory { .. })),
            "{result:?}"
        );
    }

    /// Every lint README in the repository satisfies the catalog contract.
    #[test]
    fn repository_readmes_are_valid() {
        // Read the real tree with an empty registry so only README validation applies.
        let lints = read_lints(repository_root(), &RegisteredLints::default())
            .expect("repository READMEs should be valid");
        // A short list would mean discovery silently skipped crates.
        assert!(
            lints.len() > 200,
            "expected the full lint tree, found {}",
            lints.len()
        );
    }

    /// The repository classifies macro-declared lints by their rendered suggestions.
    #[test]
    fn repository_applicability_follows_rendered_suggestions() {
        // Axum route lints build replacements; nest-at-root lints only print help.
        let lints = read_lints(repository_root(), &RegisteredLints::default())
            .expect("repository READMEs should be valid");

        // Look up one lint's classification by name.
        let applicability = |name: &str| {
            lints
                .iter()
                .find(|lint| lint.name == name)
                .map(|lint| lint.applicability)
        };
        assert_eq!(
            applicability("axum_route_empty_path"),
            Some(Applicability::MachineApplicable)
        );
        assert_eq!(
            applicability("axum_nest_at_root"),
            Some(Applicability::NotMachineApplicable)
        );
        assert_eq!(
            applicability("collect_return"),
            Some(Applicability::NotMachineApplicable)
        );
    }
}
