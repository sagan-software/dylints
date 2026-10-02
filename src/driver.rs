//! Embedded rustc driver used when Cargo re-enters the unified lint executable.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
    io::{self, Write as _},
    process::ExitCode,
    str::FromStr as _,
};

use rustc_span::Symbol;

use crate::{category::Category, runtime};

/// Marks an invocation as the internal rustc-compatible process.
const MODE_ENV: &str = "SAGAN_LINTS_DRIVER";
/// Comma-separated closed category keys selected by the parent runner.
const CATEGORIES_ENV: &str = "SAGAN_LINTS_DRIVER_CATEGORIES";
/// Unit-separator-delimited rustc lint-level arguments selected by the parent runner.
const RUSTFLAGS_ENV: &str = "SAGAN_LINTS_DRIVER_RUSTFLAGS";
/// Presence marker for a lint-listing compiler process.
const LIST_ENV: &str = "SAGAN_LINTS_DRIVER_LIST";
/// Presence marker that limits private lints to the packages Cargo was asked to check.
const NO_DEPS_ENV: &str = "SAGAN_LINTS_DRIVER_NO_DEPS";
/// Cargo's marker for a package selected on the command line.
const PRIMARY_PACKAGE_ENV: &str = "CARGO_PRIMARY_PACKAGE";
/// Separator between encoded rustc arguments, matching `CARGO_ENCODED_RUSTFLAGS`.
const RUSTFLAGS_SEPARATOR: char = '\x1f';

/// Selection encoded into one private-lint child phase.
#[derive(Clone, Copy, Debug)]
pub(super) struct DriverSelection<'a> {
    /// Categories registered by the re-entered compiler.
    pub(super) categories: &'a [Category],
    /// Lint-level arguments appended to each wrapped rustc invocation.
    pub(super) rustflags: &'a [String],
    /// Whether the process lists lints instead of compiling target code.
    pub(super) is_list: bool,
    /// Whether lints apply only to packages selected on Cargo's command line.
    pub(super) is_no_deps: bool,
}

/// rustc callback that registers the selected statically linked lint categories.
struct Callbacks {
    /// Categories loaded into this compiler process.
    categories: Vec<Category>,
    /// Whether registration should print only the newly added lints.
    is_list: bool,
}

impl rustc_driver::Callbacks for Callbacks {
    /// Register the selected lint groups and record the state that selects them.
    fn config(&mut self, config: &mut rustc_interface::Config) {
        let categories = self.categories.clone();
        let is_list = self.is_list;
        config.register_lints = Some(Box::new(move |session, lint_store| {
            // Cargo replays cached results unless the lint selection is part of dep-info.
            track_lint_selection(session);

            // Snapshot existing names so list mode emits only this binary's additions.
            let before = is_list.then(|| {
                lint_store
                    .get_lints()
                    .iter()
                    .map(|lint| lint.name)
                    .collect::<BTreeSet<_>>()
            });
            for category in &categories {
                category.register(session, lint_store);
            }

            // A listing process finishes after registration and never compiles target code.
            if let Some(before) = before {
                let status = write_lint_list(&categories, lint_store, &before)
                    .map_or(ExitCode::FAILURE, |()| ExitCode::SUCCESS);
                std::process::exit(exit_code(status));
            }
        }));

        // Match Dylint's compiler profile for predictable lint analysis.
        config.opts.unstable_opts.mir_opt_level = Some(0);
    }
}

/// Return whether the process was started as Cargo's internal compiler wrapper.
#[expect(
    runtime_env_read,
    reason = "the compiler wrapper reads its process marker"
)]
pub(super) fn is_enabled() -> bool {
    env::var_os(MODE_ENV).is_some()
}

/// Configure one child phase to re-enter this executable as a compiler driver.
pub(super) fn configure_environment(
    environment: &mut BTreeMap<OsString, OsString>,
    selection: DriverSelection<'_>,
) {
    // Encode process-only state so target Cargo arguments remain untouched.
    let category_keys = selection
        .categories
        .iter()
        .map(|category| category.key())
        .collect::<Vec<_>>()
        .join(",");
    let encoded_rustflags = selection.rustflags.join(&RUSTFLAGS_SEPARATOR.to_string());
    // Replace only the variables owned by the compiler-wrapper protocol.
    let _previous = environment.insert(MODE_ENV.into(), "1".into());
    let _previous = environment.insert(CATEGORIES_ENV.into(), category_keys.into());
    let _previous = environment.insert(RUSTFLAGS_ENV.into(), encoded_rustflags.into());
    // Presence markers select the listing and primary-package-only modes.
    for (is_enabled, marker) in [
        (selection.is_list, LIST_ENV),
        (selection.is_no_deps, NO_DEPS_ENV),
    ] {
        if is_enabled {
            let _previous = environment.insert(marker.into(), "1".into());
        }
    }
}

/// Run rustc with the selected embedded lint callbacks and preserve its exit status.
#[expect(
    abc_size,
    manual_arg_parsing,
    runtime_env_read,
    reason = "rustc driver arguments and environment are an intentional compiler boundary"
)]
pub(super) fn run() -> ExitCode {
    let categories = match selected_categories() {
        Ok(categories) => categories,
        Err(error) => return driver_error(&error),
    };
    let sysroot = match runtime::sysroot() {
        Ok(sysroot) => sysroot,
        Err(error) => return driver_error(&error),
    };

    // Preserve Cargo's argument vector because rustc_driver owns its command-line grammar.
    // Cargo supplies the real rustc path first; rustc_driver treats it as argv[0].
    let mut arguments = env::args_os()
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if arguments.is_empty() {
        arguments.push("rustc".to_owned());
    }
    arguments.extend([
        "--sysroot".to_owned(),
        sysroot.to_string_lossy().into_owned(),
        "--check-cfg=cfg(dylint_lib,values(any()))".to_owned(),
    ]);
    // In no-deps mode, a workspace dependency compiles as plain rustc, as under Clippy's driver.
    let is_linted =
        env::var_os(NO_DEPS_ENV).is_none() || env::var_os(PRIMARY_PACKAGE_ENV).is_some();
    let categories = if is_linted { categories } else { Vec::new() };
    for category in &categories {
        // Pass each selected category as a rustc cfg consumed by the embedded groups.
        let key = category.key();
        arguments.push(format!(r#"--cfg=dylint_lib="{key}""#));
    }
    if is_linted {
        arguments.extend(selected_rustflags());
    }

    // Listing mode exits after callbacks register the requested category.
    let is_list = env::var_os(LIST_ENV).is_some();
    let mut callbacks = Callbacks {
        categories,
        is_list,
    };
    rustc_driver::catch_with_exit_code(|| {
        rustc_driver::run_compiler(&arguments, &mut callbacks);
    })
}

/// Record the environment and executable that select lints so Cargo reruns stale checks.
///
/// Cargo fingerprints the wrapper path but not the wrapper's environment. Without
/// these dep-info entries, a check that passed under one category selection is
/// fresh under another, and Cargo skips the newly selected lints.
#[expect(
    runtime_env_read,
    reason = "the compiler wrapper records its process boundary for Cargo"
)]
fn track_lint_selection(session: &rustc_session::Session) {
    // Record every selection variable, including absent ones, as an env-dep entry.
    for variable in [
        CATEGORIES_ENV,
        RUSTFLAGS_ENV,
        NO_DEPS_ENV,
        PRIMARY_PACKAGE_ENV,
    ] {
        let value = env::var(variable).ok();
        let _is_new = session.env_depinfo.lock().insert((
            Symbol::intern(variable),
            value.as_deref().map(Symbol::intern),
        ));
    }
    // Rebuilding this executable changes the registered lint code itself.
    if let Ok(executable) = env::current_exe() {
        let _is_new = session
            .file_depinfo
            .lock()
            .insert(Symbol::intern(&executable.to_string_lossy()));
    }
}

/// Parse and validate the closed category list supplied by the parent process.
#[expect(
    runtime_env_read,
    reason = "the compiler wrapper reads its typed process boundary"
)]
fn selected_categories() -> Result<Vec<Category>, crate::category_parse_error::CategoryParseError> {
    // Empty segments cannot name a category; every remaining value must match the closed type.
    let encoded = env::var(CATEGORIES_ENV).unwrap_or_default();
    encoded
        .split(',')
        .filter(|value| !value.is_empty())
        .map(Category::from_str)
        .collect()
}

/// Decode rustc arguments from the separator-delimited process boundary.
#[expect(
    runtime_env_read,
    reason = "the compiler wrapper reads its typed process boundary"
)]
fn selected_rustflags() -> Vec<String> {
    // An empty value encodes no arguments, as in Cargo's encoded flags.
    env::var(RUSTFLAGS_ENV)
        .unwrap_or_default()
        .split(RUSTFLAGS_SEPARATOR)
        .filter(|argument| !argument.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Emit one stable category heading and every newly registered lint.
#[expect(
    abc_size,
    reason = "the list output keeps width calculation and row rendering together"
)]
fn write_lint_list(
    categories: &[Category],
    lint_store: &rustc_lint::LintStore,
    before: &BTreeSet<&'static str>,
) -> io::Result<()> {
    // Retain only callback additions and sort them for stable output across compiler runs.
    let mut lints = lint_store
        .get_lints()
        .iter()
        .copied()
        .filter(|lint| !before.contains(lint.name))
        .collect::<Vec<_>>();
    // Stable sorting keeps list output reproducible across rustc callback order.
    lints.sort_by_key(|lint| lint.name);
    // Compute widths once so the human-facing table remains aligned.
    let name_width = lints.iter().map(|lint| lint.name.len()).max().unwrap_or(0);
    let level_width = lints
        .iter()
        .map(|lint| lint.default_level.as_str().len())
        .max()
        .unwrap_or(0);

    let mut stdout = io::stdout().lock();
    let heading = categories
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    // Write the heading as bytes so the driver avoids interpolated logging syntax.
    let heading_line = format!("{heading}\n");
    stdout.write_all(heading_line.as_bytes())?;
    for lint in lints {
        // Normalize rustc's lint name while retaining its declared description.
        let name = lint.name.to_lowercase();
        let level = lint.default_level.as_str();
        let description = lint.desc;
        let row = format!("    {name:<name_width$}    {level:<level_width$}    {description}\n");
        // Flush each stable row before moving to the next registered lint.
        stdout.write_all(row.as_bytes())?;
    }
    writeln!(stdout)?;
    Ok(())
}

/// Write a process-boundary setup failure without invoking the compiler.
fn driver_error(error: &dyn std::fmt::Display) -> ExitCode {
    // Keep setup failures readable even when rustc has not started.
    let diagnostic = format!("sagan-lints compiler driver: {error}\n");
    // Ignore a closed stderr pipe because the exit code remains the process contract.
    drop(io::stderr().lock().write_all(diagnostic.as_bytes()));
    ExitCode::FAILURE
}

/// Convert a stable process exit value for `process::exit`.
fn exit_code(code: ExitCode) -> i32 {
    i32::from(code != ExitCode::SUCCESS)
}

/// Keep the shared linker support crate in the final static dependency graph.
use dylint_support as _;
