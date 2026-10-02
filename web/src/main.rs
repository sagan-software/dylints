//! Web interface generation for the custom Rust lint catalog.
//!
//! The binary discovers lint README files, validates their shared documentation
//! contract, joins them with the runner's registered lint levels, and writes a
//! static site modeled on the Clippy lint list. Typed category, level, and
//! applicability values keep the client-side filters closed, while filesystem
//! and rendering failures retain the path that caused them.

#![expect(
    clippy::disallowed_methods,
    reason = "the synchronous site generator owns its input and output files"
)]

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fmt, fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::ExitCode,
    str::FromStr,
};

use askama::Template as _;
use clap::Parser;
use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser as MarkdownParser, Tag, TagEnd,
    html,
};
use thiserror::Error;
use walkdir::WalkDir;

/// Public repository that hosts the lint sources and issue tracker.
const REPOSITORY_URL: &str = "https://github.com/sagan-software/dylints";

/// Required README section headings in display order.
const REQUIRED_SECTIONS: [&str; 5] = [
    "## What it does",
    "## Why is this bad?",
    "## Known problems",
    "## Example",
    "## Use instead",
];

/// Static files copied next to the generated page, keyed by output file name.
const STATIC_ASSETS: [(&str, &str); 4] = [
    ("style.css", include_str!("../static/style.css")),
    ("script.js", include_str!("../static/script.js")),
    ("theme.js", include_str!("../static/theme.js")),
    (
        "LICENSE-CLIPPY-MIT",
        include_str!("../static/LICENSE-CLIPPY-MIT"),
    ),
];

/// Command-line interface for building the Rust lint catalog.
#[derive(Debug, Parser)]
#[command(name = "sagan-lints-web")]
struct Cli {
    /// Repository root containing `lints`.
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Output of `sagan-lints --list-private-lints`, used for default lint levels.
    #[arg(long)]
    lint_list: PathBuf,

    /// Directory that receives `index.html` and its static assets.
    #[arg(long, default_value = "public")]
    out_dir: PathBuf,
}

/// Stable lint identity used as both the display name and URL fragment.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LintId(Box<str>);

impl LintId {
    /// Validate a README title as a stable lint identity.
    fn parse(value: &str, path: &Path) -> Result<Self, SiteError> {
        // Reject every spelling that cannot remain a stable file, display, and URL identity.
        if value.is_empty()
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            })
        {
            return Err(SiteError::InvalidLintId {
                path: path.to_path_buf(),
                value: value.to_owned(),
            });
        }
        Ok(Self(value.into()))
    }

    /// The rustc lint name, which spells crate-directory hyphens as underscores.
    fn lint_name(&self) -> String {
        self.0.replace('-', "_")
    }
}

impl fmt::Display for LintId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Closed set of lint categories represented by the repository layout.
///
/// Variants are declared in the alphabetical order of their keys so the derived
/// ordering matches the group filter order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum LintCategory {
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
    Performance,
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
    /// Derive a category from the lint crate's path below `lints/`.
    fn from_relative_path(path: &Path) -> Result<Self, SiteError> {
        let mut components = path.components().map(std::path::Component::as_os_str);
        let first = components.next().and_then(OsStr::to_str);
        let second = components.next().and_then(OsStr::to_str);

        // Crate-specific families live one level deeper, below `lints/crates/<crate>`.
        let category = match (first, second) {
            (Some("crates"), Some(family)) => crate_category(family),
            (Some(family), _) => simple_category(family),
            (None, _) => None,
        };
        category.ok_or_else(|| SiteError::UnknownCategory(path.to_path_buf()))
    }

    /// Stable filter and badge key for the category.
    const fn key(self) -> &'static str {
        match self {
            Self::Axum => "axum",
            Self::Bevy => "bevy",
            Self::Cargo => "cargo",
            Self::Clap => "clap",
            Self::Complexity => "complexity",
            Self::Correctness => "correctness",
            Self::Insta => "insta",
            Self::Maintainability => "maintainability",
            Self::Performance => "perf",
            Self::Reqwest => "reqwest",
            Self::Restriction => "restriction",
            Self::Schemars => "schemars",
            Self::Serde => "serde",
            Self::Sqlx => "sqlx",
            Self::Strum => "strum",
            Self::Style => "style",
            Self::Suspicious => "suspicious",
            Self::TestCase => "test-case",
            Self::Thiserror => "thiserror",
            Self::Tokio => "tokio",
            Self::Tracing => "tracing",
        }
    }
}

/// Map a repository-wide category directory to its closed category value.
fn simple_category(value: &str) -> Option<LintCategory> {
    Some(match value {
        "cargo" => LintCategory::Cargo,
        "complexity" => LintCategory::Complexity,
        "correctness" => LintCategory::Correctness,
        "maintainability" => LintCategory::Maintainability,
        "perf" => LintCategory::Performance,
        "restriction" => LintCategory::Restriction,
        "style" => LintCategory::Style,
        "suspicious" => LintCategory::Suspicious,
        _ => return None,
    })
}

/// Map a crate-family directory to its category.
fn crate_category(value: &str) -> Option<LintCategory> {
    Some(match value {
        "axum" => LintCategory::Axum,
        "bevy" => LintCategory::Bevy,
        "clap" => LintCategory::Clap,
        "insta" => LintCategory::Insta,
        "reqwest" => LintCategory::Reqwest,
        "schemars" => LintCategory::Schemars,
        "serde" => LintCategory::Serde,
        "sqlx" => LintCategory::Sqlx,
        "strum" => LintCategory::Strum,
        "test-case" => LintCategory::TestCase,
        "thiserror" => LintCategory::Thiserror,
        "tracing" => LintCategory::Tracing,
        "tokio" => LintCategory::Tokio,
        _ => return None,
    })
}

/// Default level of a lint as shown by the level filter.
///
/// `None` marks a documented lint that the runner bundle does not register.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum LintLevel {
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

impl FromStr for LintLevel {
    type Err = SiteError;

    /// Parse one rustc default-level spelling from the runner's lint table.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "allow" => Ok(Self::Allow),
            "warn" => Ok(Self::Warn),
            "deny" => Ok(Self::Deny),
            "forbid" => Ok(Self::Forbid),
            _ => Err(SiteError::InvalidLintLevel(value.to_owned())),
        }
    }
}

impl fmt::Display for LintLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
            Self::Forbid => "forbid",
            Self::None => "none",
        })
    }
}

/// Whether a lint can offer a fix that `cargo fix` and `--fix` apply automatically.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Applicability {
    /// The lint never emits a `MachineApplicable` suggestion.
    NotMachineApplicable,
    /// The lint emits at least one `MachineApplicable` suggestion.
    MachineApplicable,
}

impl fmt::Display for Applicability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotMachineApplicable => "NotMachineApplicable",
            Self::MachineApplicable => "MachineApplicable",
        })
    }
}

/// Source text that marks a machine-applicable suggestion.
const MACHINE_APPLICABLE: &str = "Applicability::MachineApplicable";

/// Names of support-crate macros whose expansion emits a machine-applicable suggestion.
///
/// Several lint families declare their lints through `macro_rules!` macros in a
/// sibling `support` crate, so the lint's own source never names an applicability.
#[derive(Debug, Default)]
struct SuggestingMacros(BTreeSet<String>);

impl SuggestingMacros {
    /// Collect every `macro_rules!` definition whose body names a machine-applicable suggestion.
    fn from_source(source: &str) -> Self {
        let mut names = BTreeSet::new();
        for (start, _) in source.match_indices("macro_rules!") {
            let definition = source
                .get(start + "macro_rules!".len()..)
                .unwrap_or_default();
            let Some((name, body)) = definition.split_once('{') else {
                continue;
            };
            // Count braces to find the end of the definition; macro bodies keep braces balanced.
            let mut depth = 1_usize;
            let end = body
                .char_indices()
                .find_map(|(index, character)| {
                    match character {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    (depth == 0).then_some(index)
                })
                .unwrap_or(body.len());
            if body.get(..end).unwrap_or(body).contains(MACHINE_APPLICABLE) {
                let _inserted = names.insert(name.trim().to_owned());
            }
        }
        Self(names)
    }

    /// Classify one lint from its Rust source and its UI test output.
    ///
    /// A lint is machine applicable when its source, or a support macro it invokes,
    /// names `Applicability::MachineApplicable` and its UI output renders at least one
    /// suggestion. Requiring both excludes macro-declared lints whose variant never
    /// builds a replacement.
    fn applicability(&self, source: &str, ui_stderr: &str) -> Applicability {
        let names_suggestion = source.contains(MACHINE_APPLICABLE)
            || self
                .0
                .iter()
                .any(|name| source.contains(&format!("{name}!")));
        if names_suggestion && renders_suggestion(ui_stderr) {
            Applicability::MachineApplicable
        } else {
            Applicability::NotMachineApplicable
        }
    }
}

/// Report whether compiletest output contains a rendered code suggestion.
///
/// rustc renders a short suggestion inline after the primary carets (`^^^ help: ...`)
/// and a longer one as a diff with `+`, `-`, or `~` markers after the line number.
/// A plain `= help:` note is not a suggestion.
fn renders_suggestion(stderr: &str) -> bool {
    stderr.lines().any(|line| {
        let gutter_body = line.split_once('|').map(|(_, body)| body.trim());
        let is_inline =
            gutter_body.is_some_and(|body| body.starts_with('^') && body.contains("^ help: "));
        let is_marker_line = gutter_body.is_some_and(|body| {
            !body.is_empty()
                && body
                    .chars()
                    .all(|character| matches!(character, '+' | '~' | ' '))
        });
        let is_diff_line = ["LL + ", "LL - ", "LL ~ "]
            .iter()
            .any(|marker| line.starts_with(marker));
        is_inline || is_marker_line || is_diff_line
    })
}

/// Lint names and default levels registered by the runner bundle.
#[derive(Debug, Default)]
struct RegisteredLints(BTreeMap<String, LintLevel>);

impl FromStr for RegisteredLints {
    type Err = SiteError;

    /// Parse `sagan-lints --list-private-lints` output.
    ///
    /// Lint rows are indented by four spaces and hold the name, the level, and a
    /// description. Category headings, command echoes, and timing lines are not
    /// indented and are skipped.
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut lints = BTreeMap::new();
        for row in source.lines().filter_map(|line| line.strip_prefix("    ")) {
            let mut fields = row.split_whitespace();
            let (Some(name), Some(level)) = (fields.next(), fields.next()) else {
                return Err(SiteError::InvalidLintListRow(row.to_owned()));
            };
            // One lint registered twice would make its level ambiguous.
            if lints.insert(name.to_owned(), level.parse()?).is_some() {
                return Err(SiteError::DuplicateRegisteredLint(name.to_owned()));
            }
        }
        Ok(Self(lints))
    }
}

/// One lint and its rendered README.
#[derive(Debug)]
struct Lint {
    /// rustc lint name used for display, anchors, and copying.
    name: String,
    /// Semantic category derived from the source path.
    category: LintCategory,
    /// Default level registered by the runner bundle.
    level: LintLevel,
    /// Least automatable suggestion applicability named in the lint source.
    applicability: Applicability,
    /// Repository-relative path of the lint's `src/lib.rs`, with `/` separators.
    source_path: String,
    /// Rendered Markdown body, excluding the title.
    documentation_html: String,
}

/// Askama context for the lint list page.
#[derive(askama::Template)]
#[template(path = "index.html")]
struct SiteTemplate<'lint> {
    /// Lints in name order.
    lints: &'lint [Lint],
    /// Levels present in the catalog, in severity order.
    levels: Vec<LintLevel>,
    /// Categories present in the catalog, in key order.
    groups: Vec<LintCategory>,
    /// Applicabilities present in the catalog, from least to most automatable.
    applicabilities: Vec<Applicability>,
    /// Repository link used by the issue, source, and corner links.
    repository_url: &'static str,
}

impl<'lint> SiteTemplate<'lint> {
    /// Build the page context and derive each filter vocabulary from the lints.
    fn new(lints: &'lint [Lint]) -> Self {
        Self {
            lints,
            levels: present(lints.iter().map(|lint| lint.level)),
            groups: present(lints.iter().map(|lint| lint.category)),
            applicabilities: present(lints.iter().map(|lint| lint.applicability)),
            repository_url: REPOSITORY_URL,
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

/// Render README Markdown into the lint documentation fragment.
///
/// Level-two README headings become level-three headings to match the lint list
/// styles. Fenced code keeps only its first info token, so `rust,ignore` renders
/// as `language-rust` for client-side highlighting.
fn render_markdown(source: &str) -> String {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let events = MarkdownParser::new_ext(source, options).map(|event| match event {
        Event::Start(Tag::Heading {
            level,
            id,
            classes,
            attrs,
        }) => Event::Start(Tag::Heading {
            level: demote(level),
            id,
            classes,
            attrs,
        }),
        Event::End(TagEnd::Heading(level)) => Event::End(TagEnd::Heading(demote(level))),
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
            let language = info
                .split(|character: char| character == ',' || character.is_whitespace())
                .next()
                .unwrap_or_default()
                .to_owned();
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from(
                language,
            ))))
        }
        other => other,
    });
    let mut rendered = String::new();
    html::push_html(&mut rendered, events);
    rendered
}

/// Lower a heading by one level, keeping level six unchanged.
const fn demote(level: HeadingLevel) -> HeadingLevel {
    match level {
        HeadingLevel::H1 => HeadingLevel::H2,
        HeadingLevel::H2 => HeadingLevel::H3,
        HeadingLevel::H3 => HeadingLevel::H4,
        HeadingLevel::H4 => HeadingLevel::H5,
        HeadingLevel::H5 | HeadingLevel::H6 => HeadingLevel::H6,
    }
}

/// Errors produced while discovering or rendering the catalog.
#[derive(Debug, Error)]
enum SiteError {
    /// A filesystem operation failed.
    #[error("could not access {path}: {source}")]
    Io {
        /// Affected path.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },

    /// Recursive directory traversal failed.
    #[error("could not traverse {path}: {source}")]
    Walk {
        /// Directory being traversed.
        path: PathBuf,
        /// Underlying traversal error.
        source: walkdir::Error,
    },

    /// A lint title is not a stable URL-safe identity.
    #[error("lint README {path} has invalid title {value:?}")]
    InvalidLintId {
        /// README path.
        path: PathBuf,
        /// Invalid title.
        value: String,
    },

    /// A lint title does not match its crate directory.
    #[error("lint title {title:?} does not match directory {directory:?} in {path}")]
    MismatchedLintId {
        /// README path.
        path: PathBuf,
        /// Parsed title.
        title: String,
        /// Directory name.
        directory: PathBuf,
    },

    /// A README is missing its level-one lint title.
    #[error("lint README has no level-one title: {0}")]
    MissingTitle(PathBuf),

    /// A README does not use the required section contract.
    #[error("lint README {path} is missing or misorders {heading:?}")]
    InvalidReadmeStructure {
        /// README path.
        path: PathBuf,
        /// Missing or misplaced heading.
        heading: &'static str,
    },

    /// A lint path has no supported category.
    #[error("lint path has no supported category: {0}")]
    UnknownCategory(PathBuf),

    /// A discovered path escaped the expected repository root.
    #[error("path {path} is not below {root}: {source}")]
    PathOutsideRoot {
        /// Path being reduced.
        path: PathBuf,
        /// Expected ancestor.
        root: PathBuf,
        /// Underlying prefix error.
        source: std::path::StripPrefixError,
    },

    /// No lint crates were discovered.
    #[error("no lint UI directories found under {0}")]
    NoLints(PathBuf),

    /// A lint list row lacks a name or a level.
    #[error("lint list row has no name and level: {0:?}")]
    InvalidLintListRow(String),

    /// A lint list row names an unknown level.
    #[error("lint list has unknown level {0:?}")]
    InvalidLintLevel(String),

    /// A lint list names one lint twice.
    #[error("lint list registers {0} more than once")]
    DuplicateRegisteredLint(String),

    /// Registered lints have no README in the lint tree.
    #[error("registered lints have no README: {}", .0.join(", "))]
    UndocumentedLints(Vec<String>),

    /// Askama could not render the HTML template.
    #[error("could not render the site template: {0}")]
    Template(#[from] askama::Error),
}

/// Build the web interface and return a process-friendly result.
fn main() -> ExitCode {
    // Convert the domain result into the process exit contract.
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Write one stable diagnostic without panicking on a closed stderr stream.
            let diagnostic = format!("error: {error}\n");
            drop(io::stderr().lock().write_all(diagnostic.as_bytes()));
            ExitCode::FAILURE
        }
    }
}

/// Build the web interface from the parsed paths.
fn run(cli: &Cli) -> Result<(), SiteError> {
    let lint_list = read_to_string(&cli.lint_list)?;
    let registered = lint_list.parse()?;
    generate_site(&cli.root, &registered, &cli.out_dir)
}

/// Discover lint documentation and write the rendered site.
fn generate_site(
    root: &Path,
    registered: &RegisteredLints,
    out_dir: &Path,
) -> Result<(), SiteError> {
    // Resolve the input boundary and render everything before touching the destination.
    let root = fs::canonicalize(root).map_err(|source| SiteError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let lints = read_lints(&root, registered)?;
    let rendered_html = SiteTemplate::new(&lints).render()?;

    // Write the page last so a partial run never leaves a page without its assets.
    fs::create_dir_all(out_dir).map_err(|source| SiteError::Io {
        path: out_dir.to_path_buf(),
        source,
    })?;
    for (name, contents) in STATIC_ASSETS {
        write_file(&out_dir.join(name), contents)?;
    }
    let index = out_dir.join("index.html");
    write_file(&index, &rendered_html)?;
    report_site(&index, &lints)
}

/// Write one output file with path-aware diagnostics.
fn write_file(path: &Path, contents: &str) -> Result<(), SiteError> {
    fs::write(path, contents).map_err(|source| SiteError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Report the page and any documented lint that the runner does not register.
fn report_site(index: &Path, lints: &[Lint]) -> Result<(), SiteError> {
    // Unregistered lints still render, so name them for the maintainer.
    let unregistered: Vec<_> = lints
        .iter()
        .filter(|lint| lint.level == LintLevel::None)
        .map(|lint| lint.name.as_str())
        .collect();
    if !unregistered.is_empty() {
        let names = unregistered.join(", ");
        let warning =
            format!("warning: documented lints are not registered by the runner: {names}\n");
        drop(io::stderr().lock().write_all(warning.as_bytes()));
    }

    let index_path = index.display();
    let lint_count = lints.len();
    let summary = format!("Generated {index_path} from {lint_count} lint README files\n");
    io::stdout()
        .lock()
        .write_all(summary.as_bytes())
        .map_err(|source| SiteError::Io {
            path: index.to_path_buf(),
            source,
        })
}

/// Read one UTF-8 file with path-aware diagnostics.
fn read_to_string(path: &Path) -> Result<String, SiteError> {
    fs::read_to_string(path).map_err(|source| SiteError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Discover and render every lint README below `lints/`.
fn read_lints(root: &Path, registered: &RegisteredLints) -> Result<Vec<Lint>, SiteError> {
    // Discover fixture directories because each lint crate must contain one.
    let lints_root = root.join("lints");
    let mut ui_directories = Vec::new();
    for entry in WalkDir::new(&lints_root).follow_links(false) {
        let entry = entry.map_err(|source| SiteError::Walk {
            path: lints_root.clone(),
            source,
        })?;
        // Directory type and exact name jointly identify a lint fixture root.
        if entry.file_type().is_dir() && entry.file_name() == OsStr::new("ui") {
            ui_directories.push(entry.into_path());
        }
    }
    if ui_directories.is_empty() {
        return Err(SiteError::NoLints(lints_root));
    }

    // Parse every README through the same validation and rendering boundary.
    let macros = suggesting_macros(&lints_root)?;
    let mut lints = ui_directories
        .iter()
        .map(|ui_directory| read_lint(root, ui_directory, registered, &macros))
        .collect::<Result<Vec<_>, _>>()?;
    lints.sort_by(|left, right| left.name.cmp(&right.name));

    // Every registered lint needs a README, or the catalog would silently omit it.
    let undocumented: Vec<_> = registered
        .0
        .keys()
        .filter(|name| {
            lints
                .binary_search_by(|lint| lint.name.as_str().cmp(name.as_str()))
                .is_err()
        })
        .cloned()
        .collect();
    if !undocumented.is_empty() {
        return Err(SiteError::UndocumentedLints(undocumented));
    }
    Ok(lints)
}

/// Parse one lint crate into the typed template model.
fn read_lint(
    root: &Path,
    ui_directory: &Path,
    registered: &RegisteredLints,
    macros: &SuggestingMacros,
) -> Result<Lint, SiteError> {
    // Resolve the README beside the discovered UI directory.
    let lint_directory = ui_directory
        .parent()
        .ok_or_else(|| SiteError::UnknownCategory(ui_directory.to_path_buf()))?;
    let readme = lint_directory.join("README.md");
    let document = read_to_string(&readme)?;

    // Validate the document structure before deriving identity and location.
    let (title, body) = split_title(&document, &readme)?;
    validate_readme_sections(body, &readme)?;
    let id = LintId::parse(title, &readme)?;
    validate_directory_name(&id, lint_directory, &readme)?;
    let relative_directory = strip_prefix(lint_directory, root)?;
    let category =
        LintCategory::from_relative_path(strip_prefix(relative_directory, Path::new("lints"))?)?;

    // Join the runner registry and the lint source for the filter metadata.
    let name = id.lint_name();
    let level = registered.0.get(&name).copied().unwrap_or(LintLevel::None);
    let applicability = macros.applicability(
        &rust_sources(&lint_directory.join("src"))?,
        &ui_outputs(ui_directory)?,
    );
    let source_path = relative_directory
        .join("src")
        .join("lib.rs")
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");

    Ok(Lint {
        name,
        category,
        level,
        applicability,
        source_path,
        documentation_html: render_markdown(body),
    })
}

/// Concatenate every Rust file below `directory` in path order.
fn rust_sources(directory: &Path) -> Result<String, SiteError> {
    let mut sources = String::new();
    for entry in WalkDir::new(directory)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = entry.map_err(|source| SiteError::Walk {
            path: directory.to_path_buf(),
            source,
        })?;
        if entry.path().extension() == Some(OsStr::new("rs")) {
            sources.push_str(&read_to_string(entry.path())?);
            sources.push('\n');
        }
    }
    Ok(sources)
}

/// Concatenate the expected compiler output of every UI test case.
fn ui_outputs(ui_directory: &Path) -> Result<String, SiteError> {
    let mut outputs = String::new();
    for entry in WalkDir::new(ui_directory).max_depth(1).sort_by_file_name() {
        let entry = entry.map_err(|source| SiteError::Walk {
            path: ui_directory.to_path_buf(),
            source,
        })?;
        if entry.path().extension() == Some(OsStr::new("stderr")) {
            outputs.push_str(&read_to_string(entry.path())?);
        }
    }
    Ok(outputs)
}

/// Collect suggesting macros from every family `support/src` directory below `lints_root`.
fn suggesting_macros(lints_root: &Path) -> Result<SuggestingMacros, SiteError> {
    let mut sources = String::new();
    for entry in WalkDir::new(lints_root)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = entry.map_err(|source| SiteError::Walk {
            path: lints_root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if entry.file_type().is_dir()
            && path.file_name() == Some(OsStr::new("src"))
            && path.parent().and_then(Path::file_name) == Some(OsStr::new("support"))
        {
            sources.push_str(&rust_sources(path)?);
        }
    }
    Ok(SuggestingMacros::from_source(&sources))
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

/// Split the required first-line title from its Markdown body.
fn split_title<'source>(
    source: &'source str,
    path: &Path,
) -> Result<(&'source str, &'source str), SiteError> {
    // Parse and validate the title before exposing the remaining Markdown body.
    let (title_line, body) = source
        .split_once('\n')
        .ok_or_else(|| SiteError::MissingTitle(path.to_path_buf()))?;
    let title = title_line
        .strip_prefix("# ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SiteError::MissingTitle(path.to_path_buf()))?;
    Ok((title, body.trim_start()))
}

/// Enforce the same level-two section structure for every lint document.
fn validate_readme_sections(source: &str, path: &Path) -> Result<(), SiteError> {
    // Track the last heading so duplicates cannot satisfy the required order.
    let lines: Vec<_> = source.lines().collect();
    let mut previous_index = None;

    // Require every shared section once in its declared sequence.
    for heading in REQUIRED_SECTIONS {
        let index = lines
            .iter()
            .position(|line| line.trim_end() == heading)
            .filter(|index| previous_index.is_none_or(|previous| *index > previous))
            .ok_or_else(|| SiteError::InvalidReadmeStructure {
                path: path.to_path_buf(),
                heading,
            })?;
        previous_index = Some(index);
    }
    Ok(())
}

/// Confirm the README title matches the lint crate directory exactly.
fn validate_directory_name(
    id: &LintId,
    lint_directory: &Path,
    readme: &Path,
) -> Result<(), SiteError> {
    // Resolve the final path component before comparing it with the validated lint identity.
    let directory = lint_directory
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| SiteError::UnknownCategory(lint_directory.to_path_buf()))?;
    if directory == id.0.as_ref() {
        return Ok(());
    }
    Err(SiteError::MismatchedLintId {
        path: readme.to_path_buf(),
        title: id.to_string(),
        directory: lint_directory.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Applicability, LintCategory, LintId, LintLevel, RegisteredLints, STATIC_ASSETS, SiteError,
        SiteTemplate, SuggestingMacros, generate_site, read_lints, render_markdown,
        validate_readme_sections,
    };
    use askama::Template as _;
    use std::{fs, path::Path};

    /// README body that satisfies the shared section contract.
    const README_BODY: &str = "## What it does\n\nChecks things.\n\n## Why is this bad?\n\nReasons.\n\n## Known problems\n\nNone.\n\n## Example\n\n```rust,ignore\nlet value = 1;\n```\n\n## Use instead\n\n```rust\nlet value = 2;\n```\n";

    /// Resolve the repository root from the web crate.
    fn repository_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("web must remain directly below the repository root")
    }

    /// Write one lint crate with a README, UI expectation, and library source.
    fn write_lint(root: &Path, directory: &str, title: &str, library: &str, ui_stderr: &str) {
        let lint = root.join("lints").join(directory);
        fs::create_dir_all(lint.join("ui")).expect("fixture UI directory should be writable");
        fs::create_dir_all(lint.join("src")).expect("fixture source directory should be writable");
        fs::write(
            lint.join("README.md"),
            format!("# {title}\n\n{README_BODY}"),
        )
        .expect("fixture README should be writable");
        fs::write(lint.join("src/lib.rs"), library).expect("fixture source should be writable");
        fs::write(lint.join("ui/main.stderr"), ui_stderr)
            .expect("fixture UI expectation should be writable");
    }

    /// Build a two-lint repository with one registered and one unregistered lint.
    fn fixture_repository() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("temporary directory should be available");
        write_lint(
            root.path(),
            "style/fixable_style",
            "fixable_style",
            "fn fix() { let _ = Applicability::MachineApplicable; }\n",
            "   |     ^^^ help: fix it\n",
        );
        write_lint(
            root.path(),
            "crates/serde/serde-unregistered",
            "serde-unregistered",
            "fn check() {}\n",
            "   = help: rewrite it\n",
        );
        root
    }

    /// The URL identity contract accepts repository lint names.
    #[test]
    fn lint_id_accepts_canonical_name() {
        let id = LintId::parse("serde-cow-missing-borrow", Path::new("README.md"))
            .expect("canonical lint name should be valid");
        assert_eq!(id.to_string(), "serde-cow-missing-borrow");
        assert_eq!(id.lint_name(), "serde_cow_missing_borrow");
    }

    /// Lint titles reject characters that cannot be URL fragments.
    #[test]
    fn lint_id_rejects_uppercase() {
        assert!(matches!(
            LintId::parse("Bad_Name", Path::new("README.md")),
            Err(SiteError::InvalidLintId { .. })
        ));
    }

    /// Category paths map both repository-wide and crate-specific families.
    #[test]
    fn category_paths_map_to_closed_categories() {
        let category = |path: &str| LintCategory::from_relative_path(Path::new(path)).ok();
        assert_eq!(
            category("crates/strum/strum-enum-representation"),
            Some(LintCategory::Strum)
        );
        assert_eq!(
            category("perf/boxed_future_return"),
            Some(LintCategory::Performance)
        );
        assert_eq!(category("crates/unknown/lint"), None);
        assert_eq!(category("unknown/lint"), None);
    }

    /// The README contract rejects the older level-three structure.
    #[test]
    fn readme_structure_requires_level_two_sections() {
        assert!(matches!(
            validate_readme_sections("### What it does\n\nText", Path::new("README.md")),
            Err(SiteError::InvalidReadmeStructure { .. })
        ));
    }

    /// Runner output parses into names and levels while skipping non-row lines.
    #[test]
    fn registered_lints_parse_runner_rows() {
        let source = "$ sagan-lints rustc -W help\nstyle\n    first_lint    warn    Does a thing\n    second_lint   allow   Does another\n\nTotal: 1.0s\n";
        let registered: RegisteredLints = source.parse().expect("runner rows should parse");
        assert_eq!(registered.0.get("first_lint"), Some(&LintLevel::Warn));
        assert_eq!(registered.0.get("second_lint"), Some(&LintLevel::Allow));
        assert_eq!(registered.0.len(), 2);
    }

    /// Malformed, unknown-level, and duplicate rows are rejected independently.
    #[test]
    fn registered_lints_reject_invalid_rows() {
        let parse = |source: &str| source.parse::<RegisteredLints>();
        assert!(matches!(
            parse("    lonely\n"),
            Err(SiteError::InvalidLintListRow(_))
        ));
        assert!(matches!(
            parse("    lint loud desc\n"),
            Err(SiteError::InvalidLintLevel(_))
        ));
        assert!(matches!(
            parse("    lint warn a\n    lint deny b\n"),
            Err(SiteError::DuplicateRegisteredLint(_))
        ));
    }

    /// Every rustc level spelling round-trips through its display form.
    #[test]
    fn lint_levels_round_trip() {
        for level in [
            LintLevel::Allow,
            LintLevel::Warn,
            LintLevel::Deny,
            LintLevel::Forbid,
        ] {
            assert_eq!(level.to_string().parse::<LintLevel>().ok(), Some(level));
        }
        assert_eq!(LintLevel::None.to_string(), "none");
    }

    /// Applicability follows direct suggestions and suggesting support macros.
    #[test]
    fn applicability_follows_support_macros() {
        let macros = SuggestingMacros::from_source(
            "macro_rules! suggests { () => { fix(Applicability::MachineApplicable) }; }\n\
             macro_rules! only_helps { () => { help() }; }\n\
             fn helper() { let _ = Applicability::MachineApplicable; }\n",
        );
        assert_eq!(macros.0.iter().collect::<Vec<_>>(), ["suggests"]);
        let inline = "   |     ^^^ help: remove it\n";
        let classify = |source: &str| macros.applicability(source, inline);
        assert_eq!(
            classify("fn check() {}"),
            Applicability::NotMachineApplicable
        );
        assert_eq!(
            classify("support::only_helps! {}"),
            Applicability::NotMachineApplicable
        );
        assert_eq!(
            classify("support::suggests! {}"),
            Applicability::MachineApplicable
        );
        assert_eq!(
            classify("diag.span_suggestion(span, msg, fix, Applicability::MachineApplicable)"),
            Applicability::MachineApplicable
        );
    }

    /// Only rendered suggestions count, never plain help notes.
    #[test]
    fn suggestion_rendering_formats() {
        assert!(super::renders_suggestion(
            "LL |     #[a]\n   |     ^^^^ help: remove it\n"
        ));
        assert!(super::renders_suggestion("LL - #[serde(default)]\n"));
        assert!(super::renders_suggestion("LL ~     let value = x?;\n"));
        assert!(super::renders_suggestion(
            "LL |     value.ok()?\n   |          +++++\n"
        ));
        assert!(!super::renders_suggestion(
            "   = help: use `Router::merge`\n"
        ));
        assert!(!super::renders_suggestion(
            "   |     ^^^^ the router is nested here\n"
        ));
        let macros = SuggestingMacros::from_source(
            "macro_rules! suggests { () => { Applicability::MachineApplicable }; }",
        );
        assert_eq!(
            macros.applicability("suggests! {}", "   = help: plain\n"),
            Applicability::NotMachineApplicable
        );
    }

    /// The repository classifies macro-declared fixable lints as machine applicable.
    #[test]
    fn repository_applicability_includes_macro_suggestions() {
        let lints = read_lints(repository_root(), &RegisteredLints::default())
            .expect("repository READMEs should be valid");
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
            applicability("schemars_redundant_serde_rename"),
            Some(Applicability::MachineApplicable)
        );
        assert_eq!(
            applicability("collect_return"),
            Some(Applicability::NotMachineApplicable)
        );
    }

    /// Markdown headings drop one level and fenced code keeps its first language token.
    #[test]
    fn markdown_demotes_headings_and_normalizes_languages() {
        let html = render_markdown("## Title\n\n```rust,ignore\nfn main() {}\n```\n");
        assert!(html.contains("<h3>Title</h3>"));
        assert!(html.contains("<code class=\"language-rust\">"));
    }

    /// A small repository renders the complete Clippy-style page.
    #[test]
    fn fixture_catalog_snapshot() {
        let root = fixture_repository();
        let registered: RegisteredLints = "    fixable_style    warn    Fixable\n"
            .parse()
            .expect("fixture lint list should parse");
        let lints = read_lints(root.path(), &registered).expect("fixture lints should load");

        // The unregistered lint renders with the `none` level.
        let levels: Vec<_> = lints
            .iter()
            .map(|lint| (lint.name.as_str(), lint.level))
            .collect();
        assert_eq!(
            levels,
            [
                ("fixable_style", LintLevel::Warn),
                ("serde_unregistered", LintLevel::None)
            ]
        );

        let html = SiteTemplate::new(&lints)
            .render()
            .expect("Askama template should render");
        insta::assert_snapshot!("fixture_catalog", html);
    }

    /// A registered lint without a README stops generation.
    #[test]
    fn undocumented_registered_lint_is_rejected() {
        let root = fixture_repository();
        let registered: RegisteredLints = "    missing_readme    warn    Missing\n"
            .parse()
            .expect("fixture lint list should parse");
        match read_lints(root.path(), &registered) {
            Err(SiteError::UndocumentedLints(names)) => assert_eq!(names, ["missing_readme"]),
            other => panic!("expected an undocumented lint error, got {other:?}"),
        }
    }

    /// Site generation writes the page and every static asset.
    #[test]
    fn generate_site_writes_page_and_assets() {
        let root = fixture_repository();
        let out_dir = root.path().join("public");
        generate_site(root.path(), &RegisteredLints::default(), &out_dir)
            .expect("site generation should succeed");
        for name in STATIC_ASSETS
            .iter()
            .map(|(name, _)| *name)
            .chain(["index.html"])
        {
            assert!(out_dir.join(name).is_file(), "{name} should be written");
        }
    }

    /// Every lint README in the repository satisfies the catalog contract.
    #[test]
    fn repository_readmes_are_valid() {
        let lints = read_lints(repository_root(), &RegisteredLints::default())
            .expect("repository READMEs should be valid");
        assert!(
            lints.len() > 200,
            "expected the full lint tree, found {}",
            lints.len()
        );
    }
}
