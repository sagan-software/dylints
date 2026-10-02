//! Embedded rustc driver used when Cargo re-enters the unified lint executable.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
    io::{self, Write as _},
    process::ExitCode,
    str::FromStr as _,
};

use crate::{category::Category, runtime};

/// Marks an invocation as the internal rustc-compatible process.
const MODE_ENV: &str = "SAGAN_LINTS_DRIVER";
/// Comma-separated closed category keys selected by the parent runner.
const CATEGORIES_ENV: &str = "SAGAN_LINTS_DRIVER_CATEGORIES";
/// JSON array of rustc lint-level arguments selected by the parent runner.
const RUSTFLAGS_ENV: &str = "SAGAN_LINTS_DRIVER_RUSTFLAGS";
/// Presence marker for a lint-listing compiler process.
const LIST_ENV: &str = "SAGAN_LINTS_DRIVER_LIST";

/// rustc callback that registers the selected statically linked lint categories.
struct Callbacks {
    /// Categories loaded into this compiler process.
    categories: Vec<Category>,
    /// Whether registration should print only the newly added lints.
    is_list: bool,
}

impl rustc_driver::Callbacks for Callbacks {
    /// Extend rustc's lint store while retaining any earlier compiler callback.
    fn config(&mut self, config: &mut rustc_interface::Config) {
        let previous = config.register_lints.take();
        let categories = self.categories.clone();
        let is_list = self.is_list;
        config.register_lints = Some(Box::new(move |session, lint_store| {
            if let Some(previous) = &previous {
                previous(session, lint_store);
            }

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
    categories: &[Category],
    rustflags: &[String],
    is_list: bool,
) -> Result<(), serde_json::Error> {
    // Encode process-only state so target Cargo arguments remain untouched.
    let category_keys = categories
        .iter()
        .map(|category| category.key())
        .collect::<Vec<_>>()
        .join(",");
    let encoded_rustflags = serde_json::to_string(rustflags)?;
    // Replace only the variables owned by the compiler-wrapper protocol.
    let _previous = environment.insert(MODE_ENV.into(), "1".into());
    let _previous = environment.insert(CATEGORIES_ENV.into(), category_keys.into());
    let _previous = environment.insert(RUSTFLAGS_ENV.into(), encoded_rustflags.into());
    if is_list {
        // Listing mode uses a separate marker so it can exit after registration.
        let _previous = environment.insert(LIST_ENV.into(), "1".into());
    }
    Ok(())
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
    let rustflags = match selected_rustflags() {
        Ok(rustflags) => rustflags,
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
    for category in &categories {
        // Pass each selected category as a rustc cfg consumed by the embedded groups.
        let key = category.key();
        arguments.push(format!(r#"--cfg=dylint_lib="{key}""#));
    }
    arguments.extend(rustflags);

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

/// Decode rustc arguments from the typed JSON process boundary.
#[expect(
    runtime_env_read,
    reason = "the compiler wrapper reads its typed process boundary"
)]
fn selected_rustflags() -> Result<Vec<String>, serde_json::Error> {
    let encoded = env::var(RUSTFLAGS_ENV).unwrap_or_else(|_| "[]".to_owned());
    serde_json::from_str(&encoded)
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
