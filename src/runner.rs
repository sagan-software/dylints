//! Process orchestration, embedded compiler-driver phases, and bounded build state.

use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Write as _},
    path::{Component, Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{
    category::{Category, DEFAULT_CATEGORIES},
    cli::Cli,
    code_quality,
    diagnostics::{
        ChangedRanges, apply_machine_fixes, changed_ranges, diagnostics_from_cargo_output,
        filter_diagnostics, format_diagnostic, parse_machine_fixes,
    },
    driver,
    error::{PanicMessage, RunnerError},
    runtime::Runtime,
};
use serde::Serialize;

/// Personal Clippy behavior kept outside target repositories.
const CLIPPY_CONFIG: &str = "allow-indexing-slicing-in-tests = true\n\
allow-panic-in-tests = true\n\
allow-unwrap-in-tests = true\n\
allow-expect-in-tests = true\n\
allow-dbg-in-tests = true\n\
allow-print-in-tests = true\n\
cognitive-complexity-threshold = 15\n\
too-many-lines-threshold = 50\n";

/// Strict rustc lints enabled by default.
const STRICT_RUSTC_LINTS: &str = include_str!("../profiles/strict-rustc.lints");
/// Strict Clippy lints enabled by default.
const STRICT_CLIPPY_LINTS: &str = include_str!("../profiles/strict-clippy.lints");
/// Noisier optional Clippy lints.
const AGGRESSIVE_CLIPPY_LINTS: &str = include_str!("../profiles/aggressive-clippy.txt");
/// Repository-policy private checks disabled for external targets.
const REPO_POLICY_LINTS: &str = include_str!("../profiles/repo-policy-lints.lints");
/// Maximum number of fix-and-rerun passes before the runner reports instability.
const MAX_FIX_PASSES: usize = 10;
/// Default size limit for a runner-managed Cargo target directory.
const DEFAULT_TARGET_CACHE_MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
/// Environment override for the managed Cargo target-directory size limit.
const TARGET_CACHE_MAX_BYTES_ENV: &str = "SAGAN_LINTS_TARGET_CACHE_MAX_BYTES";

/// Cargo target directory and whether the runner owns its lifecycle.
#[derive(Clone, Debug)]
struct TargetDirectory {
    /// Directory passed to Cargo through `CARGO_TARGET_DIR`.
    path: PathBuf,
    /// Whether the runner may prune this directory after a run.
    is_managed: bool,
}

/// Cleanup guard for a runner-managed target directory.
#[derive(Debug)]
struct TargetCacheGuard {
    /// Managed target path retained until cleanup completes.
    path: Option<PathBuf>,
    /// Maximum retained size, or `None` for an explicitly selected directory.
    maximum_bytes: Option<u64>,
}

/// One executable phase and its isolated environment additions.
#[derive(Clone, Debug)]
struct Phase {
    /// Stable log and report name.
    name: String,
    /// Executable path or command name.
    program: OsString,
    /// Ordered process arguments.
    args: Vec<OsString>,
    /// Phase-specific environment additions.
    environment: BTreeMap<OsString, OsString>,
}

/// Selection specific to one aggregate, category, or listing private-lint phase.
#[derive(Clone, Copy, Debug)]
struct PrivateLintPhaseSpec<'category> {
    /// Stable phase name.
    name: &'category str,
    /// Embedded categories registered for the phase.
    categories: &'category [Category],
    /// Whether the phase lists lints instead of checking code.
    is_list: bool,
    /// Whether Cargo must emit JSON for diagnostics or source fixes.
    is_json_mode: bool,
}

/// Inputs shared by one ordered phase pass.
struct PhasePass<'a> {
    /// Parsed runner options.
    args: &'a Cli,
    /// Canonical target repository.
    repo: &'a Path,
    /// Ordered child phases.
    phases: &'a [Phase],
    /// Optional changed-line filter.
    ranges: Option<&'a ChangedRanges>,
    /// Whether child output must be parsed as JSON.
    is_json_mode: bool,
    /// Optional directory for complete phase logs.
    log_root: Option<&'a Path>,
}

/// Captured result of one process phase.
#[derive(Debug)]
struct PhaseResult {
    /// Stable phase name.
    name: String,
    /// Rendered executable and arguments.
    command: String,
    /// Child exit status.
    status: ExitStatus,
    /// Complete standard output.
    stdout: Vec<u8>,
    /// Complete standard error.
    stderr: Vec<u8>,
    /// Wall-clock runtime.
    elapsed: Duration,
}

/// Serializable phase timing entry.
#[derive(Debug, Serialize)]
struct TimingPhase<'a> {
    /// Stable phase name.
    name: &'a str,
    /// Wall-clock phase duration.
    elapsed_seconds: f64,
    /// Numeric child exit status.
    return_code: i32,
}

/// Serializable complete timing report.
#[derive(Debug, Serialize)]
struct TimingReport<'a> {
    /// Complete runner duration.
    total_seconds: f64,
    /// Ordered phase timing entries.
    phases: Vec<TimingPhase<'a>>,
}

impl TargetCacheGuard {
    /// Prepare cleanup only for the default runner-owned target directory.
    fn new(target: &TargetDirectory) -> Result<Self, RunnerError> {
        let maximum_bytes = if target.is_managed {
            target_cache_limit()?
        } else {
            None
        };
        Ok(Self {
            path: target.is_managed.then(|| target.path.clone()),
            maximum_bytes,
        })
    }

    /// Remove an oversized managed target directory and disarm the guard.
    #[expect(
        many_exit_points,
        missing_intent_comments,
        reason = "cache pruning must preserve each filesystem failure boundary"
    )]
    fn prune(&mut self) -> Result<Option<u64>, RunnerError> {
        let Some(path) = self.path.clone() else {
            return Ok(None);
        };
        let Some(maximum_bytes) = self.maximum_bytes else {
            self.path = None;
            return Ok(None);
        };
        let size = match directory_size(&path) {
            Ok(size) => size,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                self.path = None;
                return Ok(None);
            }
            Err(source) => {
                return Err(RunnerError::external(
                    format!(
                        "could not measure managed target directory {}",
                        path.display()
                    ),
                    source,
                ));
            }
        };
        if size <= maximum_bytes {
            self.path = None;
            return Ok(None);
        }
        fs::remove_dir_all(&path).map_err(|source| {
            RunnerError::external(
                format!(
                    "could not prune managed target directory {}",
                    path.display()
                ),
                source,
            )
        })?;
        self.path = None;
        Ok(Some(size))
    }
}

impl Drop for TargetCacheGuard {
    /// Remove an oversized managed target directory during early failure cleanup.
    #[expect(
        missing_intent_comments,
        reason = "Drop keeps cleanup best-effort and non-panicking"
    )]
    fn drop(&mut self) {
        let Some(path) = self.path.take() else {
            return;
        };
        let Some(maximum_bytes) = self.maximum_bytes else {
            return;
        };
        let Ok(size) = directory_size(&path) else {
            return;
        };
        if size > maximum_bytes {
            drop(fs::remove_dir_all(path));
        }
    }
}

/// Execute the selected lint phases and return their stable process exit code.
#[expect(
    clippy::too_many_lines,
    abc_size,
    cyclomatic_complexity,
    many_exit_points,
    npath_complexity,
    runtime_env_read,
    reason = "the outer runner coordinates the ordered process and cache boundaries"
)]
pub(super) fn run(args: &Cli, cache_roots: &[PathBuf]) -> Result<u8, RunnerError> {
    // Resolve the repository and pinned runtime before any child can mutate its build cache.
    let started = Instant::now();
    let repo = fs::canonicalize(&args.repo).map_err(|source| {
        RunnerError::external(format!("could not resolve {}", args.repo.display()), source)
    })?;
    // Refuse file paths before creating any runner cache state.
    if !repo.is_dir() {
        return Err(RunnerError::NotDirectory { path: repo });
    }
    // Choose writable state outside the target repository.
    let cache = cache_root(cache_roots, &repo)?;
    // Keep the default Cargo target directory outside the target repository.
    let inherited_target = env::var_os("CARGO_TARGET_DIR").map(PathBuf::from);
    let target = target_directory(args, &repo, &cache, inherited_target.as_deref());
    let mut target_cache_guard = TargetCacheGuard::new(&target)?;
    // Changed-range, fix, and report modes all need structured compiler diagnostics.
    let is_json_mode =
        args.changed_range.is_some() || args.fix.is_some() || args.gitlab_code_quality.is_some();
    // Load the Cargo-installed compiler runtime before constructing phase commands.
    let runtime = Runtime::load()?;
    // Preserve the CLI phase order in execution and timing output.
    let phases = build_phases(args, &repo, &cache, &target.path, &runtime, is_json_mode)?;
    // An empty selection succeeds without preparing toolchains or logs.
    if phases.is_empty() {
        if let Some(path) = &args.gitlab_code_quality {
            write_code_quality(path, &[])?;
        }
        write_stdout("No commands selected.\n")?;
        return Ok(0);
    }
    // Dry runs expose exact commands without mutating toolchain or target state.
    if args.dry_run.is_some() {
        write_stdout(&format!(
            "SAGAN_LINTS_TOOLCHAIN={}\nSAGAN_LINTS_CACHE_DIR={}\nCARGO_TARGET_DIR={}\n",
            runtime.toolchain_name,
            cache.display(),
            target.path.display()
        ))?;
        // Print each phase in the same order a real run would execute it.
        for phase in &phases {
            write_stdout(&format!("{}: {}\n", phase.name, render_command(phase)))?;
        }
        return Ok(0);
    }
    // Resolve changed hunks once so every phase uses the same revision boundary.
    let ranges = args
        .changed_range
        .as_deref()
        .map(|range| changed_ranges(&repo, range))
        .transpose()?;
    // Tell users the exact filtering scope before commands begin.
    if let Some(changed) = &ranges {
        let count: usize = changed.values().map(Vec::len).sum();
        write_stdout(&format!(
            "Filtering diagnostics to {count} changed hunks from {}.\n",
            args.changed_range.as_deref().unwrap_or_default()
        ))?;
    }
    // Create one log directory shared by all selected phases.
    let log_root = log_root(args)?;
    let exit_code = run_selected_phases(
        args,
        &repo,
        &phases,
        ranges.as_ref(),
        is_json_mode,
        started,
        log_root.as_deref(),
    )?;
    if let Some(size) = target_cache_guard.prune()? {
        write_stdout(&format!(
            "target-cache: removed the managed target directory after it reached {size} bytes and exceeded its configured limit.\n"
        ))?;
    }
    Ok(exit_code)
}

/// Run all selected phases, including repeated machine-fix verification passes.
#[expect(
    clippy::too_many_lines,
    abc_size,
    cyclomatic_complexity,
    many_exit_points,
    npath_complexity,
    source_cognitive_complexity,
    reason = "fix mode keeps pass control and final reporting in one state machine"
)]
fn run_selected_phases(
    args: &Cli,
    repo: &Path,
    phases: &[Phase],
    ranges: Option<&ChangedRanges>,
    is_json_mode: bool,
    started: Instant,
    log_root: Option<&Path>,
) -> Result<u8, RunnerError> {
    // Retain every phase result for timing output and the final report.
    let mut results = Vec::new();
    // Keep diagnostics separate because code-quality output excludes timing metadata.
    let mut report_diagnostics = Vec::new();
    // Count process failures independently from selected changed-range diagnostics.
    let mut failures = 0_usize;
    // Track selected diagnostics for the changed-range exit contract.
    let mut selected_count = 0_usize;
    // Fix mode may require several passes because one edit can reveal another lint.
    let maximum_passes = if args.fix.is_some() {
        MAX_FIX_PASSES
    } else {
        1
    };
    // Start the first pass at one so logs and user-facing messages are one-based.
    let mut pass = 0_usize;
    loop {
        // Create a distinct log root for each verification pass.
        pass += 1;
        let pass_log_root = prepare_pass_log_root(args, log_root, pass)?;
        // Pass the immutable phase selection and mutable accumulators to the worker.
        let phase_pass = PhasePass {
            args,
            repo,
            phases,
            ranges,
            is_json_mode,
            log_root: pass_log_root.as_deref(),
        };
        // Retain only the counts needed to decide whether another fix pass is required.
        let (applied_suggestions, pass_failures) = run_phase_pass(
            &phase_pass,
            &mut results,
            &mut failures,
            &mut selected_count,
            &mut report_diagnostics,
        )?;
        // A normal lint run executes exactly one pass.
        if args.fix.is_none() {
            break;
        }
        // Stop fix mode when the latest pass made no source changes.
        if applied_suggestions == 0 {
            failures += pass_failures;
            break;
        }
        if pass == maximum_passes {
            failures += pass_failures + 1;
            write_stdout(&format!(
                "fix: reached the maximum of {MAX_FIX_PASSES} passes; inspect the remaining diagnostics.\n"
            ))?;
            break;
        }
        write_stdout(&format!(
            "fix: applied {applied_suggestions} suggestions; starting verification pass {}.\n",
            pass + 1
        ))?;
    }
    // Report aggregate timing only after every phase result has been retained.
    let total = started.elapsed();
    write_stdout(&format!("Total: {:.2}s\n", total.as_secs_f64()))?;
    if let Some(log_root) = log_root {
        write_stdout(&format!("Full logs: {}\n", log_root.display()))?;
    }
    if let Some(path) = &args.timings_json {
        write_timings(path, &results, total)?;
    }
    if let Some(path) = &args.gitlab_code_quality {
        write_code_quality(path, &report_diagnostics)?;
    }
    // In filter mode, only selected diagnostics and infrastructure failures fail the run.
    Ok(u8::from(if ranges.is_some() {
        selected_count > 0 || failures > 0
    } else {
        failures > 0 || (args.gitlab_code_quality.is_some() && !report_diagnostics.is_empty())
    }))
}

/// Run one ordered phase pass and retain only timing metadata after processing output.
#[expect(
    abc_size,
    cyclomatic_complexity,
    many_exit_points,
    npath_complexity,
    source_cognitive_complexity,
    missing_intent_comments,
    reason = "phase processing preserves output, fix, changed-range, and report ordering"
)]
fn run_phase_pass(
    pass: &PhasePass<'_>,
    results: &mut Vec<PhaseResult>,
    failures: &mut usize,
    selected_count: &mut usize,
    report_diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
) -> Result<(usize, usize), RunnerError> {
    let mut applied_suggestions = 0_usize;
    let mut pass_failures = 0_usize;
    // Run phases serially to keep compiler caches and terminal output deterministic.
    for phase in pass.phases {
        write_stdout(&format!("$ {}\n", render_command(phase)))?;
        let mut result = run_phase(
            phase,
            pass.repo,
            pass.args.heartbeat_interval,
            pass.is_json_mode || pass.log_root.is_some(),
        )?;
        if let Some(log_root) = pass.log_root {
            write_logs(log_root, &result)?;
        }
        write_stdout(&format!(
            "{}: finished in {:.2}s (exit {})\n",
            result.name,
            result.elapsed.as_secs_f64(),
            status_code(result.status)
        ))?;
        if pass.args.gitlab_code_quality.is_some() {
            report_diagnostics.extend(diagnostics_from_cargo_output(
                &result.name,
                pass.repo,
                &combined_output(&result),
            ));
        }
        if let Some(changed) = pass.ranges {
            process_changed_result(&result, pass.repo, changed, selected_count, failures)?;
        } else if pass.args.fix.is_some() {
            applied_suggestions += process_fix_result(&result, pass.repo, &mut pass_failures)?;
        } else if pass.args.gitlab_code_quality.is_some() {
            let diagnostics =
                diagnostics_from_cargo_output(&result.name, pass.repo, &combined_output(&result));
            for diagnostic in &diagnostics {
                write_stdout(&format!("{}\n", format_diagnostic(diagnostic)))?;
            }
            if !result.status.success() && diagnostics.is_empty() {
                write_raw_output(&result.stdout, &result.stderr)?;
            }
            *failures += usize::from(!result.status.success());
        } else {
            write_raw_output(&result.stdout, &result.stderr)?;
            *failures += usize::from(!result.status.success());
        }
        result.stdout = Vec::new();
        result.stderr = Vec::new();
        results.push(result);
    }
    Ok((applied_suggestions, pass_failures))
}

/// Prepare an optional per-pass log directory.
fn prepare_pass_log_root(
    args: &Cli,
    log_root: Option<&Path>,
    pass: usize,
) -> Result<Option<PathBuf>, RunnerError> {
    // Keep ordinary runs free of implicit persistent logs.
    let Some(root) = log_root else {
        return Ok(None);
    };
    // Give each fix verification pass an isolated log directory.
    let path = if args.fix.is_some() {
        root.join(format!("fix-pass-{pass}"))
    } else {
        root.to_owned()
    };
    // Create the selected directory before the child phase starts.
    create_dir(&path)?;
    Ok(Some(path))
}

/// Process changed-range diagnostics and count infrastructure failures.
#[expect(
    missing_intent_comments,
    reason = "changed-range handling keeps selection and failure policy together"
)]
fn process_changed_result(
    result: &PhaseResult,
    repo: &Path,
    changed: &ChangedRanges,
    selected_count: &mut usize,
    failures: &mut usize,
) -> Result<(), RunnerError> {
    let combined = combined_output(result);
    let diagnostics = diagnostics_from_cargo_output(&result.name, repo, &combined);
    let selected = filter_diagnostics(changed, &diagnostics);
    *selected_count += selected.len();
    if selected.is_empty() {
        write_stdout(&format!(
            "{}: no diagnostics on changed lines.\n",
            result.name
        ))?;
    } else {
        for diagnostic in &selected {
            write_stdout(&format!("{}\n", format_diagnostic(diagnostic)))?;
        }
    }
    if is_changed_range_phase_failed(result.status.success(), &diagnostics, &selected) {
        *failures += 1;
        write_stdout(&format!(
            "{}: command failed before changed-range linting completed; inspect logs.\n",
            result.name
        ))?;
    }
    Ok(())
}

/// Return whether a failed phase prevented complete changed-range analysis.
fn is_changed_range_phase_failed(
    is_success: bool,
    diagnostics: &[crate::diagnostics::Diagnostic],
    selected: &[crate::diagnostics::Diagnostic],
) -> bool {
    // A successful command is complete even when it emitted no selected diagnostics.
    if is_success {
        return false;
    }
    if diagnostics.is_empty() {
        return true;
    }
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_blocking_compiler_error())
        .any(|diagnostic| !selected.contains(diagnostic))
}

/// Process fix diagnostics, apply machine suggestions, and count deferred failures.
#[expect(
    cyclomatic_complexity,
    missing_intent_comments,
    reason = "fix processing keeps diagnostic display and atomic application together"
)]
fn process_fix_result(
    result: &PhaseResult,
    repo: &Path,
    pass_failures: &mut usize,
) -> Result<usize, RunnerError> {
    let combined = combined_output(result);
    let diagnostics = diagnostics_from_cargo_output(&result.name, repo, &combined);
    for diagnostic in &diagnostics {
        write_stdout(&format!("{}\n", format_diagnostic(diagnostic)))?;
    }
    let fixes = parse_machine_fixes(repo, &combined);
    let applied = if fixes.is_empty() {
        0
    } else {
        let applied = apply_machine_fixes(repo, &fixes)?;
        if applied.suggestions > 0 {
            write_stdout(&format!(
                "{}: applied {} machine-applicable suggestions across {} files.\n",
                result.name, applied.suggestions, applied.files
            ))?;
        }
        applied.suggestions
    };
    if !result.status.success() && applied == 0 {
        *pass_failures += 1;
        write_stdout(&format!(
            "{}: command failed without machine-applicable fixes; inspect logs.\n",
            result.name
        ))?;
    }
    Ok(applied)
}

/// Combine captured child streams only for structured diagnostic modes.
#[expect(
    missing_intent_comments,
    reason = "the two streams form one Cargo diagnostic input"
)]
fn combined_output(result: &PhaseResult) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    )
}

/// Build ordered executable phases from typed CLI state.
#[expect(
    clippy::too_many_lines,
    reason = "the phase builder keeps selection policy visible in one ordered definition"
)]
fn build_phases(
    args: &Cli,
    repo: &Path,
    cache: &Path,
    target: &Path,
    runtime: &Runtime,
    is_json_mode: bool,
) -> Result<Vec<Phase>, RunnerError> {
    // List mode starts one compiler process per default category to preserve clear grouping.
    if args.list_private_lints.is_some() {
        return DEFAULT_CATEGORIES
            .iter()
            .map(|category| {
                let name = format!("dylint-list-{category}");
                private_lint_phase(
                    args,
                    repo,
                    target,
                    runtime,
                    PrivateLintPhaseSpec {
                        name: &name,
                        categories: std::slice::from_ref(category),
                        is_list: true,
                        is_json_mode,
                    },
                )
            })
            .collect();
    }
    // Add strict Clippy first unless the caller explicitly omitted it.
    let mut phases = Vec::new();
    if args.skip_clippy.is_none() {
        phases.push(clippy_phase(
            args,
            repo,
            cache,
            target,
            runtime,
            is_json_mode,
        )?);
    }
    // Expand private-lint selection into one aggregate or ordered category phases.
    if args.skip_dylint.is_none() {
        if args.dylint_category.is_empty() {
            phases.push(private_lint_phase(
                args,
                repo,
                target,
                runtime,
                PrivateLintPhaseSpec {
                    name: "dylint",
                    categories: &DEFAULT_CATEGORIES,
                    is_list: false,
                    is_json_mode,
                },
            )?);
        } else {
            // Preserve repeated category arguments as distinct observable phases.
            for category in &args.dylint_category {
                let key = category.key();
                phases.push(private_lint_phase(
                    args,
                    repo,
                    target,
                    runtime,
                    PrivateLintPhaseSpec {
                        name: &format!("dylint-{key}"),
                        categories: std::slice::from_ref(category),
                        is_list: false,
                        is_json_mode,
                    },
                )?);
            }
        }
    }
    Ok(phases)
}

/// Build the strict Clippy phase.
#[expect(
    abc_size,
    reason = "Clippy phase construction keeps Cargo and compiler policy adjacent"
)]
fn clippy_phase(
    args: &Cli,
    repo: &Path,
    cache: &Path,
    target: &Path,
    runtime: &Runtime,
    is_json_mode: bool,
) -> Result<Phase, RunnerError> {
    // Build Cargo selection before applying the runner-owned lint and environment policy.
    let (program, mut command_args) = if let Some(cargo_command) = &args.cargo_cmd {
        // Split the explicit command into its executable and fixed prefix arguments.
        let mut cargo = cargo_command.split_ascii_whitespace();
        let program = cargo.next().ok_or(RunnerError::MissingCargoCommand)?.into();
        (program, cargo.map(OsString::from).collect())
    } else {
        // Default to Cargo from the exact nightly that built this executable.
        (runtime.cargo.as_os_str().to_owned(), Vec::new())
    };
    // Apply Cargo selection before the separator that begins lint arguments.
    command_args.push("clippy".into());
    command_args.extend(selection_args(args, repo));
    if args.no_deps.is_some() {
        command_args.push("--no-deps".into());
    }
    if args.is_all_targets_selected() {
        command_args.push("--all-targets".into());
    }
    if is_json_mode {
        command_args.push("--message-format=json".into());
    }
    // Place rustc and Clippy levels after Cargo's argument separator.
    command_args.push("--".into());
    command_args.extend(lint_level_args(args).into_iter().map(OsString::from));
    let mut environment = BTreeMap::new();
    set_environment(&mut environment, "CARGO_TARGET_DIR", target.as_os_str());
    set_environment(&mut environment, "RUSTC", runtime.rustc.as_os_str());
    configure_fast_environment(args, &mut environment);
    if args.use_repo_clippy_config.is_none() {
        // Materialize the private configuration only when repository policy is disabled.
        let config = cache.join("clippy").join("clippy.toml");
        write_file_if_changed(&config, CLIPPY_CONFIG.as_bytes())?;
        if let Some(parent) = config.parent() {
            set_environment(
                &mut environment,
                "CLIPPY_CONF_DIR",
                parent.as_os_str().to_owned(),
            );
        }
    }
    // Put the exact compiler tools ahead of any inherited Rust installation.
    prepend_path(&mut environment, &runtime.toolchain_bin());
    Ok(Phase {
        name: "clippy".to_owned(),
        program,
        args: command_args,
        environment,
    })
}

/// Build one aggregate, category, or listing private-lint phase.
#[expect(
    clippy::too_many_lines,
    abc_size,
    cyclomatic_complexity,
    npath_complexity,
    missing_intent_comments,
    reason = "private-lint construction keeps Cargo and embedded-driver policy together"
)]
fn private_lint_phase(
    args: &Cli,
    repo: &Path,
    target: &Path,
    runtime: &Runtime,
    spec: PrivateLintPhaseSpec<'_>,
) -> Result<Phase, RunnerError> {
    let (program, mut command_args) = if spec.is_list {
        // Listing re-enters this executable directly and stops after lint registration.
        (
            env::current_exe()
                .map_err(|source| {
                    RunnerError::external("could not resolve current executable", source)
                })?
                .into_os_string(),
            vec!["rustc".into(), "-W".into(), "help".into()],
        )
    } else if let Some(cargo_command) = &args.cargo_cmd {
        // Preserve an explicit target-repository Cargo wrapper for private checks too.
        let mut cargo = cargo_command.split_ascii_whitespace();
        let program = cargo.next().ok_or(RunnerError::MissingCargoCommand)?.into();
        let mut command_args = cargo.map(OsString::from).collect::<Vec<_>>();
        command_args.push(
            if args.no_deps.is_some() {
                "clippy"
            } else {
                "check"
            }
            .into(),
        );
        if args.no_deps.is_some() {
            command_args.push("--no-deps".into());
        }
        (program, command_args)
    } else {
        let mut command_args = vec![OsString::from(if args.no_deps.is_some() {
            "clippy"
        } else {
            "check"
        })];
        if args.no_deps.is_some() {
            command_args.push("--no-deps".into());
        }
        (runtime.cargo.as_os_str().to_owned(), command_args)
    };

    if spec.is_list {
        // rustc's help path activates registration without reading target source.
    } else {
        // Direct Cargo owns target selection; lint arguments travel through typed environment.
        command_args.extend(selection_args(args, repo));
        if args.is_all_targets_selected() {
            command_args.push("--all-targets".into());
        }
        if spec.is_json_mode {
            command_args.push("--message-format=json".into());
        }
    }
    let mut environment = private_lint_environment(args, target, runtime);
    let rustflags = private_lint_args(args);
    driver::configure_environment(&mut environment, spec.categories, &rustflags, spec.is_list)
        .map_err(|source| {
            RunnerError::external("could not encode compiler-driver state", source)
        })?;
    if !spec.is_list {
        set_environment(
            &mut environment,
            "RUSTC_WORKSPACE_WRAPPER",
            env::current_exe()
                .map_err(|source| {
                    RunnerError::external("could not resolve current executable", source)
                })?
                .into_os_string(),
        );
    }
    Ok(Phase {
        name: spec.name.to_owned(),
        program,
        args: command_args,
        environment,
    })
}

/// Build Cargo package, feature, target, and lockfile selection arguments.
#[expect(
    abc_size,
    cyclomatic_complexity,
    npath_complexity,
    reason = "selection flags must retain Cargo's documented ordering"
)]
fn selection_args(args: &Cli, repo: &Path) -> Vec<OsString> {
    // Resolve a relative manifest against the target repository.
    let mut selection = Vec::new();
    if let Some(manifest) = &args.manifest_path {
        let manifest = if manifest.is_absolute() {
            manifest.clone()
        } else {
            repo.join(manifest)
        };
        selection.extend(["--manifest-path".into(), manifest.into_os_string()]);
    }
    // Use workspace selection only when explicit packages do not narrow it.
    if args.packages.is_empty() {
        if args.is_workspace_selected() && args.manifest_path.is_none() {
            selection.push("--workspace".into());
        }
    } else {
        // Preserve caller package order for reproducible command rendering.
        for package in &args.packages {
            selection.extend(["--package".into(), package.into()]);
        }
    }
    // Exclusions follow package selection as required by Cargo.
    for package in &args.exclude {
        selection.extend(["--exclude".into(), package.into()]);
    }
    // Translate the mutually composable feature switches independently.
    if args.all_features.is_some() {
        selection.push("--all-features".into());
    }
    if args.no_default_features.is_some() {
        selection.push("--no-default-features".into());
    }
    for features in &args.features {
        selection.extend(["--features".into(), features.into()]);
    }
    // Apply an explicit compilation target after feature selection.
    if let Some(target) = &args.target {
        selection.extend(["--target".into(), target.into()]);
    }
    // Preserve Cargo's lock and network modes in their stable order.
    selection.extend(
        [
            (args.locked.is_some(), "--locked"),
            (args.frozen.is_some(), "--frozen"),
            (args.offline.is_some(), "--offline"),
        ]
        .into_iter()
        .filter_map(|(enabled, flag)| enabled.then_some(flag.into())),
    );
    // Append escape-hatch Cargo arguments last so their order matches the CLI.
    selection.extend(args.extra_cargo_arg.iter().map(OsString::from));
    selection
}

/// Build strict rustc and Clippy lint-level arguments.
#[expect(
    abc_size,
    reason = "lint-level order mirrors the public CLI precedence"
)]
fn lint_level_args(args: &Cli) -> Vec<String> {
    // Deny all emitted warnings unless the caller explicitly requested advisory output.
    let mut levels = Vec::new();
    if args.no_deny_warnings.is_none() {
        levels.extend(["-D".to_owned(), "warnings".to_owned()]);
    }
    // Add the strict rustc baseline before user-specified rustc lints.
    if args.no_strict_rustc.is_none() {
        for lint in STRICT_RUSTC_LINTS
            .lines()
            .chain(args.rustc_lint.iter().map(String::as_str))
        {
            levels.extend(["-W".to_owned(), lint.to_owned()]);
        }
    }
    // Include aggressive Clippy policy only when its presence-only mode is set.
    let aggressive = args
        .aggressive_clippy
        .map(|_| AGGRESSIVE_CLIPPY_LINTS)
        .into_iter()
        .flat_map(str::lines);
    // Apply baseline, aggressive, and caller Clippy warnings in that order.
    for lint in STRICT_CLIPPY_LINTS
        .lines()
        .chain(aggressive)
        .chain(args.clippy_lint.iter().map(String::as_str))
    {
        levels.extend(["-W".to_owned(), lint.to_owned()]);
    }
    // Explicit allows follow warnings so the caller can waive individual Clippy lints.
    for lint in &args.allow_clippy_lint {
        levels.extend(["-A".to_owned(), lint.clone()]);
    }
    // Raw rustc arguments remain last as the lowest-level escape hatch.
    levels.extend(args.extra_rustc_arg.iter().cloned());
    levels
}

/// Construct the isolated environment used only by embedded private-lint phases.
fn private_lint_environment(
    args: &Cli,
    target: &Path,
    runtime: &Runtime,
) -> BTreeMap<OsString, OsString> {
    // Keep target artifacts outside the repository and use the build-time nightly exactly.
    let mut environment = BTreeMap::new();
    set_environment(&mut environment, "CARGO_INCREMENTAL", "0");
    set_environment(&mut environment, "CARGO_TARGET_DIR", target.as_os_str());
    set_environment(&mut environment, "RUSTC", runtime.rustc.as_os_str());
    configure_fast_environment(args, &mut environment);

    // Cargo and rustc must resolve from the same sysroot as the statically linked driver.
    prepend_path(&mut environment, &runtime.toolchain_bin());
    environment
}

/// Build private rustc levels while optionally excluding repository-policy lints.
#[expect(
    missing_intent_comments,
    reason = "private lint levels preserve policy and opt-out order"
)]
fn private_lint_args(args: &Cli) -> Vec<String> {
    let mut rustflags = vec!["-D".to_owned(), "warnings".to_owned()];
    if args.include_repo_policy_lints.is_none() {
        rustflags.extend(["-A".to_owned(), "unknown_lints".to_owned()]);
        // Allow each private repository-policy lint by its configured rustc name.
        for lint in REPO_POLICY_LINTS.lines() {
            rustflags.extend(["-A".to_owned(), lint.to_owned()]);
        }
    }
    rustflags
}

/// Disable incremental and debug-heavy profiles in the explicit fast mode.
#[expect(
    missing_intent_comments,
    reason = "fast mode applies one profile policy to every phase"
)]
fn configure_fast_environment(args: &Cli, environment: &mut BTreeMap<OsString, OsString>) {
    if args.fast.is_none() {
        return;
    }
    set_environment(environment, "CARGO_INCREMENTAL", "0");
    for profile in ["DEV", "TEST", "BENCH"] {
        set_environment(environment, format!("CARGO_PROFILE_{profile}_DEBUG"), "0");
    }
}

/// Prepend one directory to a child process's inherited executable path.
fn prepend_path(environment: &mut BTreeMap<OsString, OsString>, path: &Path) {
    // Prefer phase-local PATH state and fall back to the parent process once.
    let mut paths = vec![path.to_path_buf()];
    let existing = environment
        .get(OsStr::new("PATH"))
        .cloned()
        .or_else(|| env::var_os("PATH"));
    if let Some(existing) = existing {
        paths.extend(env::split_paths(&existing));
    }
    // Retain the old PATH unchanged when platform joining rejects a component.
    if let Ok(joined) = env::join_paths(&paths) {
        set_environment(environment, "PATH", joined);
    }
}

/// Run one process while draining both pipes and emitting progress heartbeats.
#[expect(
    clippy::too_many_lines,
    abc_size,
    cyclomatic_complexity,
    many_exit_points,
    npath_complexity,
    missing_intent_comments,
    reason = "the process boundary must set up pipes, wait, and retain both streams together"
)]
fn run_phase(
    phase: &Phase,
    repo: &Path,
    heartbeat_interval: Duration,
    is_capture_output: bool,
) -> Result<PhaseResult, RunnerError> {
    // In ordinary mode, stream child output directly to avoid retaining large diagnostics.
    let started = Instant::now();
    let mut command = Command::new(&phase.program);
    let _command = command
        .args(&phase.args)
        .envs(&phase.environment)
        .current_dir(repo)
        .stdout(if is_capture_output {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stderr(if is_capture_output {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });
    let mut child = command.spawn().map_err(|source| {
        RunnerError::external(format!("could not start phase `{}`", phase.name), source)
    })?;
    // Start both stream readers before polling so a verbose child cannot fill either pipe.
    let readers = if is_capture_output {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RunnerError::PipeUnavailable {
                phase: phase.name.clone(),
                stream: "stdout",
            })?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| RunnerError::PipeUnavailable {
                phase: phase.name.clone(),
                stream: "stderr",
            })?;
        Some((
            thread::spawn(move || read_pipe(stdout)),
            thread::spawn(move || read_pipe(stderr)),
        ))
    } else {
        None
    };
    // Use a blocking wait when heartbeats are disabled to avoid polling overhead on warm runs.
    let status = if heartbeat_interval.is_zero() {
        child.wait().map_err(|source| {
            RunnerError::external(format!("could not wait for phase `{}`", phase.name), source)
        })?
    } else {
        let mut next_heartbeat = heartbeat_interval;
        // Poll the child while emitting bounded progress messages.
        loop {
            if let Some(status) = child.try_wait().map_err(|source| {
                RunnerError::external(format!("could not wait for phase `{}`", phase.name), source)
            })? {
                break status;
            }
            // Advance the deadline by one interval to preserve heartbeat cadence.
            let elapsed = started.elapsed();
            if elapsed >= next_heartbeat {
                write_stdout(&format!(
                    "{}: still running ({:.0}s elapsed)\n",
                    phase.name,
                    elapsed.as_secs_f64()
                ))?;
                next_heartbeat += heartbeat_interval;
            }
            thread::sleep(Duration::from_millis(100));
        }
    };
    // Join both readers after the child closes its stream handles.
    let (stdout, stderr) = if let Some((stdout_reader, stderr_reader)) = readers {
        (
            join_reader(stdout_reader, &phase.name, "stdout")?,
            join_reader(stderr_reader, &phase.name, "stderr")?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(PhaseResult {
        name: phase.name.clone(),
        command: render_command(phase),
        status,
        stdout,
        stderr,
        elapsed: started.elapsed(),
    })
}

/// Drain one child pipe without blocking its sibling stream.
fn read_pipe(mut pipe: impl io::Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let _byte_count = pipe.read_to_end(&mut output)?;
    Ok(output)
}

/// Set one child environment value while deliberately replacing inherited policy.
fn set_environment(
    environment: &mut BTreeMap<OsString, OsString>,
    key: impl Into<OsString>,
    value: impl Into<OsString>,
) {
    let _previous = environment.insert(key.into(), value.into());
}

/// Join one child pipe reader and retain its typed failure source.
fn join_reader(
    reader: thread::JoinHandle<io::Result<Vec<u8>>>,
    phase: &str,
    stream: &'static str,
) -> Result<Vec<u8>, RunnerError> {
    reader
        .join()
        .map_err(|panic_payload| {
            let message = panic_payload
                .downcast_ref::<&str>()
                .map_or_else(
                    || {
                        panic_payload
                            .downcast_ref::<String>()
                            .map_or("non-string panic payload", String::as_str)
                    },
                    |message| *message,
                )
                .to_owned()
                .into_boxed_str();
            RunnerError::ReaderPanicked {
                phase: phase.to_owned(),
                stream,
                source: PanicMessage(message),
            }
        })?
        .map_err(|source| {
            RunnerError::external(
                format!("could not read {stream} for phase `{phase}`"),
                source,
            )
        })
}

/// Resolve a writable cache directory outside the target repository.
#[expect(
    missing_intent_comments,
    reason = "cache selection keeps every safety rejection visible"
)]
fn cache_root(candidates: &[PathBuf], repo: &Path) -> Result<PathBuf, RunnerError> {
    let mut attempted = Vec::new();
    // Use the first caller-prioritized directory that can be created safely.
    for candidate in candidates {
        let root = if candidate.is_absolute() {
            candidate.clone()
        } else {
            env::current_dir()
                .map_err(|source| {
                    RunnerError::external("could not resolve cache directory", source)
                })?
                .join(candidate)
        };
        attempted.push(root.clone());
        if root.starts_with(repo) {
            continue;
        }
        if create_dir(&root).is_err() {
            continue;
        }
        // Reject a symlink or mount that resolves back into the target repository.
        if fs::canonicalize(&root).is_ok_and(|resolved| resolved.starts_with(repo)) {
            continue;
        }
        return Ok(root);
    }
    Err(RunnerError::NoWritableCache {
        candidates: attempted,
    })
}

/// Resolve the caller-selected or runner-managed Cargo target directory.
#[expect(
    missing_intent_comments,
    reason = "target selection preserves explicit and inherited paths"
)]
fn target_directory(
    args: &Cli,
    repo: &Path,
    cache: &Path,
    inherited_target: Option<&Path>,
) -> TargetDirectory {
    if let Some(path) = &args.target_dir {
        return TargetDirectory {
            path: resolve_target_path(path, repo),
            is_managed: false,
        };
    }
    // Preserve an absolute external Cargo target directory supplied by a benchmark or CI.
    if let Some(path) = inherited_target {
        let path = resolve_target_path(path, repo);
        let resolves_inside_repo =
            fs::canonicalize(&path).is_ok_and(|resolved| resolved.starts_with(repo));
        if path.is_absolute() && is_external_path(&path, repo) && !resolves_inside_repo {
            return TargetDirectory {
                path,
                is_managed: false,
            };
        }
    }
    let label = repository_label(repo);
    TargetDirectory {
        path: cache
            .join("targets")
            .join(format!("{label}-{:016x}", repository_key(repo))),
        is_managed: true,
    }
}

/// Return a filesystem-safe repository label for the managed target path.
fn repository_label(repo: &Path) -> String {
    let Some(name) = repo
        .file_name()
        .map(|name| name.to_string_lossy())
        .filter(|name| !name.is_empty())
    else {
        return "repo".to_owned();
    };
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

/// Return a stable non-cryptographic key for a canonical repository path.
#[expect(
    missing_intent_comments,
    reason = "the key loop is a deliberately fixed path hash"
)]
fn repository_key(repo: &Path) -> u64 {
    let mut key = 14_695_981_039_346_656_037_u64;
    for byte in repo.to_string_lossy().bytes() {
        key ^= u64::from(byte);
        key = key.wrapping_mul(1_099_511_628_211_u64);
    }
    key
}

/// Resolve a target directory relative to the canonical target repository.
fn resolve_target_path(path: &Path, repo: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        repo.join(path)
    }
}

/// Return whether a target path is external without resolving a missing path.
fn is_external_path(path: &Path, repo: &Path) -> bool {
    !path
        .components()
        .any(|component| component == Component::ParentDir)
        && !path.starts_with(repo)
}

/// Parse the managed target-cache size limit.
#[expect(
    missing_intent_comments,
    runtime_env_read,
    reason = "the cache limit is an explicit runner configuration boundary"
)]
fn target_cache_limit() -> Result<Option<u64>, RunnerError> {
    match env::var(TARGET_CACHE_MAX_BYTES_ENV) {
        Ok(value) if value == "0" => Ok(None),
        Ok(value) => value
            .parse()
            .map(Some)
            .map_err(|_source| RunnerError::InvalidTargetCacheLimit(value)),
        Err(env::VarError::NotPresent) => Ok(Some(DEFAULT_TARGET_CACHE_MAX_BYTES)),
        Err(source) => Err(RunnerError::external(
            format!("could not read {TARGET_CACHE_MAX_BYTES_ENV}"),
            source,
        )),
    }
}

/// Measure a directory without following symbolic links.
#[expect(
    missing_intent_comments,
    reason = "directory measurement preserves symlink safety branches"
)]
fn directory_size(path: &Path) -> io::Result<u64> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Ok(0);
    }
    fs::read_dir(path)?.try_fold(0_u64, |size, entry| {
        let entry = entry?;
        Ok(size.saturating_add(directory_size(&entry.path())?))
    })
}

/// Choose an explicit log directory without creating implicit persistent logs.
fn log_root(args: &Cli) -> Result<Option<PathBuf>, RunnerError> {
    if let Some(path) = &args.log_dir {
        create_dir(path)?;
        return Ok(Some(path.clone()));
    }
    Ok(None)
}

/// Write one phase's complete command and streams.
fn write_logs(root: &Path, result: &PhaseResult) -> Result<(), RunnerError> {
    // Persist stdout before stderr to keep the file set predictable after partial failure.
    write_file(root.join(format!("{}.stdout", result.name)), &result.stdout)?;
    // Preserve stderr separately so raw tool diagnostics remain inspectable.
    write_file(root.join(format!("{}.stderr", result.name)), &result.stderr)?;
    // Record the exact rendered command that produced both streams.
    // Derive every filename from the phase name within this run's log root.
    // End the command file with a newline so shell and editor views remain complete.
    write_file(
        root.join(format!("{}.command", result.name)),
        format!("{}\n", result.command).as_bytes(),
    )
}

/// Write a machine-readable timing report.
fn write_timings(path: &Path, results: &[PhaseResult], total: Duration) -> Result<(), RunnerError> {
    let report = TimingReport {
        total_seconds: total.as_secs_f64(),
        phases: results
            .iter()
            .map(|result| TimingPhase {
                name: &result.name,
                elapsed_seconds: result.elapsed.as_secs_f64(),
                return_code: status_code(result.status),
            })
            .collect(),
    };
    let output = serde_json::to_vec_pretty(&report)
        .map_err(|source| RunnerError::external("could not serialize timing report", source))?;
    write_file(path, &[output, b"\n".to_vec()].concat())
}

/// Write one GitLab Code Quality JSON array without a byte order mark.
fn write_code_quality(
    path: &Path,
    diagnostics: &[crate::diagnostics::Diagnostic],
) -> Result<(), RunnerError> {
    // Terminate the JSON document for command-line tools that consume line-oriented files.
    let mut output = code_quality::serialize(diagnostics).map_err(|source| {
        RunnerError::external("could not serialize GitLab Code Quality report", source)
    })?;
    output.push(b'\n');
    write_file(path, &output)
}

/// Create a directory tree with its affected path in any failure.
fn create_dir(path: &Path) -> Result<(), RunnerError> {
    fs::create_dir_all(path).map_err(|source| {
        RunnerError::external(
            format!("filesystem operation failed for {}", path.display()),
            source,
        )
    })
}

/// Write a complete file and create its parent directory first.
fn write_file(path: impl AsRef<Path>, bytes: &[u8]) -> Result<(), RunnerError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        create_dir(parent)?;
    }
    fs::write(path, bytes).map_err(|source| {
        RunnerError::external(
            format!("filesystem operation failed for {}", path.display()),
            source,
        )
    })
}

/// Avoid touching stable configuration files when their contents are unchanged.
fn write_file_if_changed(path: &Path, bytes: &[u8]) -> Result<(), RunnerError> {
    if fs::read(path).is_ok_and(|current| current == bytes) {
        return Ok(());
    }
    write_file(path, bytes)
}

/// Render a process command with shell-safe debug quoting.
fn render_command(phase: &Phase) -> String {
    std::iter::once(phase.program.as_os_str())
        .chain(phase.args.iter().map(OsString::as_os_str))
        .map(quote_argument)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quote one process argument for readable copy-paste output.
fn quote_argument(value: &OsStr) -> String {
    // Leave a conservative shell-safe byte set unquoted for readable commands.
    let value = value.to_string_lossy();
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_=+.,/:@".contains(&byte))
    {
        value.into_owned()
    } else {
        // Use POSIX single-quote escaping for every other display value.
        format!("'{escaped}'", escaped = value.replace('\'', "'\\''"))
    }
}

/// Convert platform exit status to the runner's numeric report value.
fn status_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

/// Write normal runner output without panic-capable print macros.
fn write_stdout(text: &str) -> Result<(), RunnerError> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(text.as_bytes())
        .map_err(|source| RunnerError::external("could not write runner output", source))?;
    stdout
        .flush()
        .map_err(|source| RunnerError::external("could not write runner output", source))
}

/// Forward captured child output to its original stream.
fn write_raw_output(stdout: &[u8], stderr: &[u8]) -> Result<(), RunnerError> {
    io::stdout()
        .lock()
        .write_all(stdout)
        .map_err(|source| RunnerError::external("could not write runner output", source))?;
    io::stderr()
        .lock()
        .write_all(stderr)
        .map_err(|source| RunnerError::external("could not write runner output", source))
}

#[cfg(test)]
mod tests {
    use super::{CLIPPY_CONFIG, STRICT_CLIPPY_LINTS, is_external_path, log_root, target_directory};
    use crate::cli::Cli;
    use clap::Parser as _;
    use std::path::Path;

    #[test]
    fn personal_clippy_policy_enables_cognitive_complexity_at_fifteen() {
        assert!(CLIPPY_CONFIG.contains("cognitive-complexity-threshold = 15\n"));
        assert!(
            STRICT_CLIPPY_LINTS
                .lines()
                .any(|lint| lint == "clippy::cognitive_complexity")
        );
    }

    #[test]
    fn explicit_target_directory_is_resolved_against_the_repository() {
        let args = Cli::try_parse_from(["sagan-lints", "--target-dir", "build-target"]).unwrap();
        let target = target_directory(
            &args,
            Path::new("/tmp/example-repository"),
            Path::new("/tmp/sagan-cache"),
            None,
        );

        assert_eq!(
            target.path,
            Path::new("/tmp/example-repository/build-target")
        );
        assert!(!target.is_managed);
    }

    #[test]
    fn default_target_directory_is_namespaced_under_the_cache() {
        let args = Cli::try_parse_from(["sagan-lints", "--fast"]).unwrap();
        let cache = Path::new("/tmp/sagan-cache");
        let target = target_directory(&args, Path::new("/tmp/example-repository"), cache, None);

        assert!(target.is_managed);
        assert!(target.path.starts_with(cache.join("targets")));
    }

    #[test]
    fn ordinary_runs_do_not_create_a_log_directory() {
        let args = Cli::try_parse_from(["sagan-lints"]).unwrap();

        assert!(log_root(&args).unwrap().is_none());
    }

    #[test]
    fn parent_directory_components_are_not_treated_as_external() {
        assert!(!is_external_path(
            Path::new("/tmp/example-repository/../outside"),
            Path::new("/tmp/example-repository")
        ));
    }
}
