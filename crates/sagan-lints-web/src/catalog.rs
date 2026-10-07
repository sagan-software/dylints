//! Lint discovery and the typed catalog page model.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use walkdir::{DirEntry, WalkDir};

use crate::{
    applicability::{Applicability, SuggestingMacros},
    category::{CrateGroup, LintCategory},
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
    /// Default level registered by the Dylint libraries.
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
#[template(path = "index.html.jinja", escape = "html")]
pub(crate) struct SiteTemplate<'lint> {
    /// Lints in name order.
    lints: &'lint [Lint],
    /// Levels present in the catalog, in severity order.
    levels: Vec<LintLevel>,
    /// Categories present in the catalog, in key order.
    groups: Vec<LintCategory>,
    /// Explanations for the crate-specific categories present in the catalog.
    crate_groups: Vec<&'static CrateGroup>,
    /// Applicabilities present in the catalog.
    applicabilities: Vec<Applicability>,
}

impl<'lint> SiteTemplate<'lint> {
    /// Build the page context and derive each filter vocabulary from the lints.
    pub(crate) fn new(lints: &'lint [Lint]) -> Self {
        let groups = present(lints.iter().map(|lint| lint.category));
        let crate_groups = groups
            .iter()
            .filter_map(|group| group.crate_group())
            .collect();
        Self {
            lints,
            levels: present(lints.iter().map(|lint| lint.level)),
            groups,
            crate_groups,
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

/// Discover and render every lint README in the flat `crates/` workspace.
pub(crate) fn read_lints(
    root: &Path,
    registered: &RegisteredLints,
) -> Result<Vec<Lint>, SiteError> {
    // Find the crates first, then read them with shared registry and macro data.
    let crates_root = root.join("crates");
    let ui_directories = lint_ui_directories(&crates_root)?;
    let context = Context {
        root,
        registered,
        macros: suggesting_macros(&crates_root)?,
    };

    // Every registered lint must have a README among the discovered crates.
    let lints = context.read_all(&ui_directories)?;
    require_documented(&lints, registered).map(|()| lints)
}

/// Find each flat lint crate's `ui` fixture directory, rejecting an empty tree.
fn lint_ui_directories(crates_root: &Path) -> Result<Vec<PathBuf>, SiteError> {
    let mut ui_directories = directories_named(crates_root, "ui")?;
    // Lint crates live directly below `crates/`; group crates and nested UI
    // fixtures have no README at this exact depth and cannot populate the catalog.
    ui_directories.retain(|directory| {
        directory.parent().and_then(Path::parent) == Some(crates_root)
            && directory
                .parent()
                .is_some_and(|crate_dir| crate_dir.join("README.md").is_file())
            && directory
                .parent()
                .is_some_and(|crate_dir| crate_dir.join("Cargo.toml").is_file())
    });
    if ui_directories.is_empty() {
        return Err(SiteError::NoLints {
            path: crates_root.to_path_buf(),
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
    /// Default levels from Dylint.
    registered: &'input RegisteredLints,
    /// Support macros that emit machine-applicable suggestions.
    macros: SuggestingMacros,
}

impl Context<'_> {
    /// Read every lint crate and sort the results by lint name.
    fn read_all(&self, ui_directories: &[PathBuf]) -> Result<Vec<Lint>, SiteError> {
        let mut lints = Vec::new();
        // Group crates also have UI directories; `read_lint` deliberately skips them.
        for ui_directory in ui_directories {
            if let Some(lint) = self.read_lint(ui_directory)? {
                lints.push(lint);
            }
        }
        // Filesystem traversal order must not affect catalog output.
        lints.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(lints)
    }

    /// Parse one lint crate into the typed template model.
    #[expect(
        many_exit_points,
        reason = "Each early return skips a group crate or preserves the exact file-specific parse error."
    )]
    fn read_lint(&self, ui_directory: &Path) -> Result<Option<Lint>, SiteError> {
        // The crate directory is the parent of its UI fixtures.
        let lint_directory = ui_directory.parent().unwrap_or(ui_directory);
        let manifest_path = lint_directory.join("Cargo.toml");
        let manifest_source = read_to_string(&manifest_path)?;
        let manifest = toml::from_str::<toml::Table>(&manifest_source).map_err(|source| {
            SiteError::Manifest {
                path: manifest_path.clone(),
                source,
            }
        })?;
        let Some(value) = manifest
            .get("package")
            .and_then(|package| package.get("metadata"))
            .and_then(|metadata| metadata.get("dylint"))
            .and_then(|dylint| dylint.get("category"))
            .and_then(toml::Value::as_str)
        else {
            // Group crates may have UI tests, but only constituent lint crates
            // declare a catalog category in their package metadata.
            return Ok(None);
        };
        let category = LintCategory::from_metadata(value, &manifest_path)?;
        let document = read_to_string(&lint_directory.join("README.md"))?;
        let readme = Readme::parse(&document, lint_directory)?;
        let relative = strip_prefix(lint_directory, self.root)?;
        self.build_lint(&readme, relative, category).map(Some)
    }

    /// Join the README with the Dylint registry and the lint's suggestion evidence.
    fn build_lint(
        &self,
        readme: &Readme,
        relative: &Path,
        category: LintCategory,
    ) -> Result<Lint, SiteError> {
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

/// Collect suggesting macros from every `*-support/src` directory below `crates_root`.
fn suggesting_macros(crates_root: &Path) -> Result<SuggestingMacros, SiteError> {
    // Support crates keep their sources in `src` and their package names end in `-support`.
    let mut sources = String::new();
    // Applicability is determined by the shared suggestion macros, not lint source spelling.
    for directory in directories_named(crates_root, "src")? {
        let is_support = directory
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .is_some_and(|name| name.ends_with("-support"));
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
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new("."))
    }

    /// Write one flat lint crate with category metadata and catalog inputs.
    fn write_lint(root: &Path, directory: &str, category: &str, library: &str, ui_stderr: &str) {
        // The crate directory name doubles as the README title.
        let lint = root.join("crates").join(directory);
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
        fs::write(
            lint.join("Cargo.toml"),
            format!("[package.metadata.dylint]\ncategory = \"{category}\"\n"),
        )
        .expect("fixture manifest should be writable");
        fs::write(lint.join("src/lib.rs"), library).expect("fixture source should be writable");
        fs::write(lint.join("ui/main.stderr"), ui_stderr)
            .expect("fixture UI output should be writable");
    }

    /// Build a two-lint repository with one fixable lint and one help-only lint.
    pub(crate) fn fixture_repository() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("temporary directory should be available");
        write_lint(
            root.path(),
            "fixable_style",
            "style",
            "fn fix() { let _ = Applicability::MachineApplicable; }\n",
            "   |     ^^^ help: fix it\n",
        );
        write_lint(
            root.path(),
            "serde_unregistered",
            "serde",
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

    /// An empty registry renders every discovered lint with no registered level.
    #[test]
    fn fixture_catalog_empty_registration_snapshot() {
        // Read the same valid fixture through the production discovery path without registrations.
        let root = fixture_repository();
        let lints = read_lints(root.path(), &RegisteredLints::default())
            .expect("fixture lints should load without registrations");

        // Confirm absent registrations retain `none` instead of receiving an invented default.
        let summary: Vec<_> = lints
            .iter()
            .map(|lint| (lint.name.as_str(), lint.level, lint.applicability))
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "fixable_style",
                    LintLevel::None,
                    Applicability::MachineApplicable
                ),
                (
                    "serde_unregistered",
                    LintLevel::None,
                    Applicability::NotMachineApplicable
                ),
            ]
        );

        // Snapshot the complete valid page with the empty registration state.
        let html = SiteTemplate::new(&lints)
            .render()
            .expect("Askama template should render");
        insta::assert_snapshot!("fixture_catalog_empty_registration", html);
    }

    /// A malformed lint manifest retains its path in the catalog error.
    #[test]
    fn invalid_manifest_is_reported_with_its_path() {
        // Keep the lint's UI and README valid so parsing reaches Cargo.toml.
        let root = fixture_repository();
        let manifest = root.path().join("crates/fixable_style/Cargo.toml");
        fs::write(
            &manifest,
            "[package.metadata.dylint\ncategory = \"style\"\n",
        )
        .expect("fixture manifest should be writable");

        // The parse error must identify the manifest that failed.
        let result = read_lints(root.path(), &fixture_registry());
        assert!(
            matches!(&result, Err(SiteError::Manifest { path, .. }) if path == &manifest),
            "{result:?}"
        );
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
        fs::create_dir_all(root.path().join("crates")).expect("crate tree should be writable");

        // Discovery must refuse to render an empty catalog.
        let result = read_lints(root.path(), &RegisteredLints::default());
        assert!(
            matches!(result, Err(SiteError::NoLints { .. })),
            "{result:?}"
        );
    }

    /// Group tests and nested fixtures do not describe individual lints.
    #[test]
    fn group_and_nested_ui_directories_are_ignored() {
        // Add group-level and nested test fixtures beside two real lint crates.
        let root = fixture_repository();
        [
            "restriction/ui",
            "serde/ui",
            "fixable_style/ui/auxiliary/ui",
        ]
        .into_iter()
        .try_for_each(|directory| fs::create_dir_all(root.path().join("crates").join(directory)))
        .expect("fixture UI directories should be writable");

        // Only leaf lint crates contribute catalog entries.
        let lints = read_lints(root.path(), &fixture_registry())
            .expect("group and nested UI directories should be ignored");
        let names: Vec<_> = lints.iter().map(|lint| lint.name.as_str()).collect();
        assert_eq!(names, ["fixable_style", "serde_unregistered"]);
    }

    /// A tree containing only a category test still has no lint crates.
    #[test]
    fn group_only_tree_is_rejected() {
        // A category can test registration without declaring a lint itself.
        let root = tempfile::tempdir().expect("temporary directory should be available");
        fs::create_dir_all(root.path().join("crates/restriction/ui"))
            .expect("fixture UI directory should be writable");

        // Group fixtures must not bypass the empty-catalog guard.
        let result = read_lints(root.path(), &RegisteredLints::default());
        assert!(
            matches!(result, Err(SiteError::NoLints { .. })),
            "{result:?}"
        );
    }

    /// A lint crate outside every known category is rejected.
    #[test]
    fn unknown_category_is_rejected() {
        // Place a valid crate with an unsupported category declaration.
        let root = tempfile::tempdir().expect("temporary directory should be available");
        write_lint(root.path(), "odd_lint", "unknown", "fn check() {}\n", "");

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
