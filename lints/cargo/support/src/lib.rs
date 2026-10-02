#![feature(rustc_private)]
#![doc(hidden)]

//! Shared manifest helpers for the Cargo lint family.
//!
//! The helpers find the Cargo manifest for the crate rustc compiles, load it
//! into rustc's source map, and parse it with `toml_edit` so every lint can
//! report on the exact manifest key or value. They also find the workspace
//! root and list every Cargo dependency table, including `target` tables.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_lint;
extern crate rustc_span;

use std::{
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, Lint, LintContext};
use rustc_span::{BytePos, Pos as _, SourceFile, Span};
pub use toml_edit;
use toml_edit::{Document, Item, Key, Table, TableLike, Value};

use dylint_linting as _;

/// File name Cargo uses for package and workspace manifests.
const MANIFEST_FILE_NAME: &str = "Cargo.toml";

/// Dependency table names Cargo accepts, including the legacy underscore spellings.
const DEPENDENCY_TABLE_NAMES: &[&str] = &[
    "dependencies",
    "dev-dependencies",
    "dev_dependencies",
    "build-dependencies",
    "build_dependencies",
];

/// A Cargo manifest loaded into rustc's source map and parsed with spans.
///
/// Byte ranges from the parsed document index the source map copy of the file,
/// so [`Manifest::span`] turns any `toml_edit` span into a diagnostic span.
#[derive(Debug)]
pub struct Manifest {
    /// Path of the manifest file as rustc's source map knows it.
    path: PathBuf,
    /// Directory that contains the manifest.
    dir: PathBuf,
    /// Source map entry that diagnostics point into.
    file: Arc<SourceFile>,
    /// Parsed manifest that keeps the byte range of every key and value.
    document: Document<String>,
}

impl Manifest {
    /// Load and parse the manifest at `path`.
    ///
    /// Returns `None` when the file cannot be read or is not valid TOML.
    fn load(cx: &EarlyContext<'_>, path: &Path) -> Option<Self> {
        let dir = path.parent()?.to_path_buf();
        let file = cx.sess().source_map().load_file(path).ok()?;

        // Parse the source map copy so byte offsets match the positions rustc reports.
        let text = String::clone(file.src.as_deref()?);
        let document = Document::parse(text).ok()?;

        Some(Self {
            path: path.to_path_buf(),
            dir,
            file,
            document,
        })
    }

    /// Return the top-level table of the manifest.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |manifest| {
    ///     let _ = cargo_support::Manifest::root(manifest);
    /// };
    /// ```
    #[must_use]
    pub fn root(&self) -> &Table {
        self.document.as_table()
    }

    /// Return the manifest text that the document was parsed from.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |manifest| {
    ///     let _ = cargo_support::Manifest::text(manifest);
    /// };
    /// ```
    #[must_use]
    pub fn text(&self) -> &str {
        self.document.raw()
    }

    /// Convert a byte range of the manifest text into a diagnostic span.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |manifest| {
    ///     let _ = cargo_support::Manifest::span(manifest, 0..1);
    /// };
    /// ```
    #[must_use]
    pub fn span(&self, range: Range<usize>) -> Span {
        file_span(&self.file, range)
    }

    /// Return the span of the top-level `key` entry.
    ///
    /// A standard table such as `[package]` returns its header. Dotted keys,
    /// implicit tables, and inline values return the span of the key itself.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |manifest| {
    ///     let _ = cargo_support::Manifest::entry_span(manifest, "package");
    /// };
    /// ```
    #[must_use]
    pub fn entry_span(&self, key: &str) -> Option<Span> {
        entry_range(self.root(), key).map(|range| self.span(range))
    }
}

/// One entry of a Cargo dependency table.
#[derive(Clone, Copy, Debug)]
pub struct Dependency<'a> {
    /// Dependency name as written in the table, with TOML quoting removed.
    pub name: &'a str,
    /// Key of the entry, which carries the span of the dependency name.
    pub key: &'a Key,
    /// Value of the entry: a version string, an inline table, or a table.
    pub item: &'a Item,
}

impl<'a> Dependency<'a> {
    /// Return the version requirement string and the byte range of its literal.
    ///
    /// The string shorthand `name = "1"` and a `version` field in an inline,
    /// dotted, or standard table all count. Non-string versions return `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |dependency| {
    ///     let _ = cargo_support::Dependency::version(dependency);
    /// };
    /// ```
    #[must_use]
    pub fn version(&self) -> Option<(&'a str, Range<usize>)> {
        // Table forms keep the requirement in a `version` field; the shorthand is the value.
        let version = match self.item.as_table_like() {
            Some(fields) => fields.get("version")?,
            None => self.item,
        };

        Some((version.as_str()?, version.span()?))
    }
}

/// Return the local path of the crate root source file rustc compiles.
///
/// Virtual or remapped inputs have no local path and return `None`, so callers
/// skip them instead of guessing from the process working directory.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, krate| {
///     let _ = cargo_support::crate_root_path(cx, krate);
/// };
/// ```
#[must_use]
pub fn crate_root_path(cx: &EarlyContext<'_>, krate: &Crate) -> Option<PathBuf> {
    cx.sess()
        .source_map()
        .span_to_filename(krate.spans.inner_span)
        .into_local_path()
}

/// Load the nearest manifest above the crate root file.
///
/// Returns `None` when no manifest exists or it is not valid TOML.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, krate| {
///     let _ = cargo_support::package_manifest(cx, krate);
/// };
/// ```
#[must_use]
pub fn package_manifest(cx: &EarlyContext<'_>, krate: &Crate) -> Option<Manifest> {
    let crate_root = crate_root_path(cx, krate)?;
    let manifest_path = nearest_manifest_path(&crate_root)?;
    Manifest::load(cx, &manifest_path)
}

/// Return the nearest `Cargo.toml` in an ancestor directory of `crate_root`.
fn nearest_manifest_path(crate_root: &Path) -> Option<PathBuf> {
    // Cargo uses the nearest manifest above a target's source file as its package.
    crate_root
        .ancestors()
        .skip(1)
        .map(|directory| directory.join(MANIFEST_FILE_NAME))
        .find(|candidate| candidate.is_file())
}

/// Return whether the package's workspace root has a `workspace.<key>` entry.
///
/// Returns `false` when the package has no workspace root.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, package| {
///     let _ = cargo_support::has_workspace_entry(cx, package, "dependencies");
/// };
/// ```
#[must_use]
pub fn has_workspace_entry(cx: &EarlyContext<'_>, package: &Manifest, key: &str) -> bool {
    workspace_root(cx, package).is_some_and(|root| {
        workspace_table(root.root()).is_some_and(|workspace| workspace.contains_key(key))
    })
}

/// Return whether any of `names` is a file in the package directory or an
/// ancestor up to the package's workspace root.
///
/// Without a workspace root, only the package directory is searched. Files
/// above the workspace root belong to another project and do not count.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, package| {
///     let _ = cargo_support::package_has_file(cx, package, &["clippy.toml"]);
/// };
/// ```
#[must_use]
pub fn package_has_file(cx: &EarlyContext<'_>, package: &Manifest, names: &[&str]) -> bool {
    let root = workspace_root(cx, package);
    let stop = root.as_ref().map_or_else(|| &package.dir, |root| &root.dir);
    file_exists_up_to(&package.dir, stop, names)
}

/// Load the workspace root manifest for `package`.
///
/// The root is the package manifest itself when it has a `[workspace]` table.
/// Otherwise it is the nearest ancestor manifest with a `[workspace]` table
/// whose `exclude` list does not contain the package, as Cargo resolves it.
fn workspace_root(cx: &EarlyContext<'_>, package: &Manifest) -> Option<Manifest> {
    if workspace_table(package.root()).is_some() {
        return Manifest::load(cx, &package.path);
    }

    // Walk upward and skip workspaces that exclude this package, as Cargo does.
    package
        .dir
        .ancestors()
        .skip(1)
        .map(|directory| directory.join(MANIFEST_FILE_NAME))
        .filter(|candidate| candidate.is_file())
        .filter_map(|candidate| Manifest::load(cx, &candidate))
        .find(|candidate| {
            workspace_table(candidate.root())
                .is_some_and(|workspace| !is_excluded(&candidate.dir, workspace, &package.dir))
        })
}

/// Return the `[workspace]` table of a manifest, in any TOML table form.
fn workspace_table(root: &Table) -> Option<&dyn TableLike> {
    root.get("workspace").and_then(Item::as_table_like)
}

/// Return whether the workspace `exclude` list contains `package_dir`.
fn is_excluded(root_dir: &Path, workspace: &dyn TableLike, package_dir: &Path) -> bool {
    // Cargo matches exclude entries as path prefixes relative to the workspace root.
    workspace
        .get("exclude")
        .and_then(Item::as_array)
        .is_some_and(|paths| {
            paths
                .iter()
                .filter_map(Value::as_str)
                .any(|path| package_dir.starts_with(root_dir.join(path)))
        })
}

/// Return whether any of `names` is a file in `start` or an ancestor up to `stop`.
///
/// The walk includes `stop`. When `stop` is not an ancestor of `start`, the
/// walk continues to the file system root.
fn file_exists_up_to(start: &Path, stop: &Path, names: &[&str]) -> bool {
    for directory in start.ancestors() {
        if names.iter().any(|name| directory.join(name).is_file()) {
            return true;
        }

        // Files above the workspace root belong to another project.
        if directory == stop {
            break;
        }
    }

    false
}

/// Return the byte range that best identifies the top-level entry `key`.
fn entry_range(table: &dyn TableLike, key: &str) -> Option<Range<usize>> {
    let (key, item) = table.get_key_value(key)?;
    item.as_table().and_then(Table::span).or_else(|| key.span())
}

/// Return the package dependency tables of a manifest.
///
/// Package tables precede target tables. Within each package or target, the
/// order is `dependencies`, `dev-dependencies`, `dev_dependencies`,
/// `build-dependencies`, then `build_dependencies`. Target tables and entries
/// within each dependency table retain their manifest order.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |root| {
///     let _ = cargo_support::dependency_tables(root);
/// };
/// ```
pub fn dependency_tables(root: &Table) -> impl Iterator<Item = &dyn TableLike> {
    // Each `target.<cfg>` table can hold the same dependency tables as the package.
    let target_tables = root
        .get("target")
        .and_then(Item::as_table_like)
        .into_iter()
        .flat_map(|targets| targets.iter())
        .filter_map(|(_, target)| target.as_table_like())
        .flat_map(named_dependency_tables);

    named_dependency_tables(root).chain(target_tables)
}

/// Return the `[workspace.dependencies]` table of a manifest.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |root| {
///     let _ = cargo_support::workspace_dependency_table(root);
/// };
/// ```
#[must_use]
pub fn workspace_dependency_table(root: &Table) -> Option<&dyn TableLike> {
    workspace_table(root)?
        .get("dependencies")
        .and_then(Item::as_table_like)
}

/// Return the dependency tables directly inside `table`.
fn named_dependency_tables(table: &dyn TableLike) -> impl Iterator<Item = &dyn TableLike> {
    DEPENDENCY_TABLE_NAMES
        .iter()
        .filter_map(|name| table.get(name).and_then(Item::as_table_like))
}

/// Return the entries of one dependency table in table order.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |table| {
///     let _ = cargo_support::dependencies(table).count();
/// };
/// ```
pub fn dependencies(table: &dyn TableLike) -> impl Iterator<Item = Dependency<'_>> {
    table.iter().filter_map(|(name, item)| {
        Some(Dependency {
            name,
            key: table.key(name)?,
            item,
        })
    })
}

/// Return whether an optional TOML item is the boolean `true`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// assert!(!cargo_support::is_true(None));
/// ```
#[must_use]
pub fn is_true(item: Option<&Item>) -> bool {
    item.and_then(Item::as_bool) == Some(true)
}

/// Return an empty span at the start of a text file loaded into rustc's source map.
///
/// Returns `None` when the file cannot be read as UTF-8 text.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, path| {
///     let _ = cargo_support::file_start_span(cx, path);
/// };
/// ```
#[must_use]
pub fn file_start_span(cx: &EarlyContext<'_>, path: &Path) -> Option<Span> {
    let file = cx.sess().source_map().load_file(path).ok()?;
    Some(file_span(&file, 0..0))
}

/// Convert a byte range of a loaded source file into a diagnostic span.
fn file_span(file: &SourceFile, range: Range<usize>) -> Span {
    Span::with_root_ctxt(
        file.start_pos + BytePos::from_usize(range.start),
        file.start_pos + BytePos::from_usize(range.end),
    )
}

/// Emit `lint` at `span` with a primary message and one help line.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, lint, span| {
///     cargo_support::emit_with_help(cx, lint, span, "message", "help");
/// };
/// ```
#[expect(
    clippy::let_underscore_must_use,
    reason = "the diagnostic builder methods return the builder for chaining"
)]
pub fn emit_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &str,
    help: &str,
) {
    let message = message.to_owned();
    let help = help.to_owned();

    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

#[cfg(test)]
mod tests {
    //! Unit tests for the pure manifest helpers.

    use std::path::Path;

    use toml_edit::Document;

    use super::{
        dependencies, dependency_tables, entry_range, file_exists_up_to, is_excluded,
        nearest_manifest_path, workspace_dependency_table, workspace_table,
    };

    /// Parse a test manifest.
    fn parse(manifest: &str) -> Document<String> {
        Document::parse(manifest.to_owned()).unwrap()
    }

    /// Return the dependency names of each table, one list per table.
    fn table_names(document: &Document<String>) -> Vec<Vec<&str>> {
        let root = document.as_table();
        workspace_dependency_table(root)
            .into_iter()
            .chain(dependency_tables(root))
            .map(|table| {
                dependencies(table)
                    .map(|dependency| dependency.name)
                    .collect()
            })
            .collect()
    }

    /// Return the version text of every `[dependencies]` entry, by name.
    fn versions(document: &Document<String>) -> Vec<(&str, Option<&str>)> {
        dependency_tables(document.as_table())
            .flat_map(dependencies)
            .map(|dependency| {
                let literal = dependency
                    .version()
                    .and_then(|(_, range)| document.raw().get(range));
                (dependency.name, literal)
            })
            .collect()
    }

    /// The nearest manifest is the first `Cargo.toml` above the file.
    #[test]
    fn finds_nearest_manifest_above_the_crate_root() {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let found = nearest_manifest_path(&crate_dir.join("src/lib.rs"));

        assert_eq!(found, Some(crate_dir.join("Cargo.toml")));
    }

    /// The file search includes the stop directory and nothing above it.
    #[test]
    fn file_search_stops_at_the_workspace_root() {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = crate_dir.join("src");
        let results = [
            file_exists_up_to(&src, crate_dir, &["missing", "Cargo.toml"]),
            file_exists_up_to(&src, crate_dir, &["dylint.toml"]),
            file_exists_up_to(&src, &src.join("x"), &["dylint.toml"]),
        ];

        // The workspace's `dylint.toml` is only visible when the walk passes the crate.
        assert_eq!(results, [true, false, true]);
    }

    /// Standard tables return their header; other entries return their key.
    #[test]
    fn entry_range_prefers_table_headers() {
        let document =
            parse("lints.workspace = true\nd = { e = 1 }\n[package]\nname = \"x\"\n[a.b]\nc = 1\n");
        let text =
            |key| entry_range(document.as_table(), key).and_then(|range| document.raw().get(range));

        assert_eq!(
            ["package", "lints", "a", "d", "missing"].map(text),
            [Some("[package]"), Some("lints"), Some("a"), Some("d"), None]
        );
    }

    /// Every dependency table spelling and target table is listed.
    #[test]
    fn lists_every_dependency_table_spelling() {
        let document = parse(
            r#"
            dependencies = { inline = "1" }
            dev_dependencies.dotted = "1"

            [workspace.dependencies]
            shared = "1"

            [dev-dependencies]
            dev = "1"

            [build-dependencies]
            build = "1"

            [build_dependencies]
            build_underscore = "1"

            [target.'cfg(unix)'.dependencies]
            unix = "1"

            [target.wasm32-unknown-unknown.dev-dependencies]
            wasm = "1"

            [target.'cfg(windows)']
            not_a_table = 1
            "#,
        );

        assert_eq!(
            table_names(&document),
            [
                vec!["shared"],
                vec!["inline"],
                vec!["dev"],
                vec!["dotted"],
                vec!["build"],
                vec!["build_underscore"],
                vec!["unix"],
                vec!["wasm"],
            ]
        );
        assert!(table_names(&parse("target = 1\n[workspace]")).is_empty());
    }

    /// Versions come from the shorthand or a `version` field in any table form.
    #[test]
    fn reads_versions_from_every_dependency_form() {
        let document = parse(
            r#"
            [dependencies]
            shorthand = "1"
            inline = { version = '0.1', path = "x" }
            multiline = {
                path = "x",
                version = "2",
            }
            dotted.version = "3"
            path_only = { path = "x" }
            numeric = 1
            numeric_field = { version = 1 }

            [dependencies.subtable]
            version = "4"
            "#,
        );

        assert_eq!(
            versions(&document),
            [
                ("shorthand", Some("\"1\"")),
                ("inline", Some("'0.1'")),
                ("multiline", Some("\"2\"")),
                ("dotted", Some("\"3\"")),
                ("path_only", None),
                ("numeric", None),
                ("numeric_field", None),
                ("subtable", Some("\"4\"")),
            ]
        );
    }

    /// Exclude entries match the package directory as path prefixes.
    #[test]
    fn honors_workspace_exclude_prefixes() {
        let excluding = parse("[workspace]\nexclude = [\"fixtures\", 1]\n");
        let plain = parse("workspace = {}");
        let root = Path::new("repo");
        let excluded = |document: &Document<String>, package: &str| {
            workspace_table(document.as_table())
                .is_some_and(|workspace| is_excluded(root, workspace, Path::new(package)))
        };

        assert_eq!(
            [
                excluded(&excluding, "repo/fixtures/a"),
                excluded(&excluding, "repo/member"),
                excluded(&plain, "repo/fixtures"),
            ],
            [true, false, false]
        );
        assert!(workspace_table(parse("[package]").as_table()).is_none());
    }
}
