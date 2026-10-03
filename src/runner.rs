//! Process orchestration, embedded compiler-driver phases, and bounded build state.

use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Write as _},
    path::{Component, Path, PathBuf},
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::mpsc,
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
    error::RunnerError,
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
///
/// The guard prunes on the success path through [`TargetCacheGuard::prune`] and
/// on early failure through `Drop`, so an aborted run still bounds the cache.
#[derive(Debug)]
struct TargetCacheGuard {
    /// Managed target path and its size limit, or `None` once disarmed or unlimited.
    armed: Option<(PathBuf, u64)>,
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
    /// Arm cleanup only for the default runner-owned target directory with a size limit.
    fn new(target: &TargetDirectory) -> Result<Self, RunnerError> {
        let maximum_bytes = if target.is_managed {
            target_cache_limit()?
        } else {
            None
        };
        Ok(Self {
            armed: maximum_bytes.map(|maximum_bytes| (target.path.clone(), maximum_bytes)),
        })
    }

    /// Disarm the guard and remove the managed target directory when it is oversized.
    fn prune(&mut self) -> Result<Option<u64>, RunnerError> {
        self.armed.take().map_or(Ok(None), |(path, maximum_bytes)| {
            prune_target(&path, maximum_bytes)
        })
    }
}

impl Drop for TargetCacheGuard {
    /// Bound the managed cache after an early failure without masking that failure.
    fn drop(&mut self) {
        // The original error is already propagating, so cleanup stays best-effort.
        drop(self.prune());
    }
}

/// Remove one target directory and return its size when it exceeds the limit.
fn prune_target(path: &Path, maximum_bytes: u64) -> Result<Option<u64>, RunnerError> {
    // A run that never built anything leaves no directory to measure.
    let size = match directory_size(path) {
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        measured => measured.map_err(|source| {
            RunnerError::external(
                format!(
                    "could not measure managed target directory {}",
                    path.display()
                ),
                source,
            )
        })?,
    };
    // Keep a directory within its limit so later runs reuse the build cache.
    if size <= maximum_bytes {
        return Ok(None);
    }
    fs::remove_dir_all(path).map_err(|source| {
        RunnerError::external(
            format!(
                "could not prune managed target directory {}",
                path.display()
            ),
            source,
        )
    })?;
    Ok(Some(size))
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
    let inherited_path = env::var_os("PATH");
    let target = target_directory(args, &repo, &cache, inherited_target.as_deref());
    // Changed-range, fix, and report modes all need structured compiler diagnostics.
    let is_json_mode =
        args.changed_range.is_some() || args.fix.is_some() || args.gitlab_code_quality.is_some();
    // Load the Cargo-installed compiler runtime before constructing phase commands.
    let runtime = Runtime::load()?;
    // Preserve the CLI phase order in execution and timing output.
    let phases = build_phases(
        args,
        &repo,
        &cache,
        &target.path,
        &runtime,
        inherited_path.as_deref(),
        is_json_mode,
    )?;
    // An empty selection succeeds without preparing toolchains or logs.
    if phases.is_empty() {
        if let Some(path) = &args.gitlab_code_quality {
            write_json(path, &code_quality::violations(&[]))?;
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
    // Arm cache cleanup only after planning modes have returned, because they never build.
    let mut target_cache_guard = TargetCacheGuard::new(&target)?;
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
        // Report only the final pass, because earlier passes describe source that fixes changed.
        report_diagnostics.clear();
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
        write_json(path, &timing_report(&results, total))?;
    }
    if let Some(path) = &args.gitlab_code_quality {
        write_json(path, &code_quality::violations(&report_diagnostics))?;
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
fn process_fix_result(
    result: &PhaseResult,
    repo: &Path,
    pass_failures: &mut usize,
) -> Result<usize, RunnerError> {
    // Parse the phase output before applying edits so diagnostics retain source locations.
    let combined = combined_output(result);
    let diagnostics = diagnostics_from_cargo_output(&result.name, repo, &combined);
    // Display compiler diagnostics before fix summaries so users see the source context first.
    for diagnostic in &diagnostics {
        let rendered = format_diagnostic(diagnostic);
        write_stdout(&format!("{rendered}\n"))?;
    }
    let applied = apply_phase_fixes(result, repo, &combined)?;
    // A failed command without a fix remains a failure after the pass completes.
    if !result.status.success() && applied == 0 {
        *pass_failures += 1;
        let phase_name = &result.name;
        write_stdout(&format!(
            "{phase_name}: command failed without machine-applicable fixes; inspect logs.\n"
        ))?;
    }
    Ok(applied)
}

/// Parse, apply, and summarize one phase's machine-applicable fixes.
fn apply_phase_fixes(
    result: &PhaseResult,
    repo: &Path,
    combined: &str,
) -> Result<usize, RunnerError> {
    // Re-derive machine-applicable edits from the complete phase output for this pass.
    let fixes = parse_machine_fixes(repo, combined);
    if fixes.is_empty() {
        return Ok(0);
    }
    let applied = apply_machine_fixes(repo, &fixes)?;
    // Report applied suggestions before deferred conflicts so the next pass is explicit.
    let phase_name = &result.name;
    let applied_suggestions = applied.suggestions;
    let applied_files = applied.files;
    if applied_suggestions > 0 {
        write_stdout(&format!(
            "{phase_name}: applied {applied_suggestions} machine-applicable suggestions across {applied_files} files.\n"
        ))?;
    }
    // Overlapping suggestions wait for the next pass, which re-derives them from new source.
    let deferred_suggestions = applied.deferred;
    if deferred_suggestions > 0 {
        write_stdout(&format!(
            "{phase_name}: deferred {deferred_suggestions} overlapping suggestions to the next pass.\n"
        ))?;
    }
    Ok(applied_suggestions)
}

/// Combine captured child streams only for structured diagnostic modes.
fn combined_output(result: &PhaseResult) -> String {
    // Preserve stdout before stderr because Cargo diagnostics can span both streams.
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    format!("{stdout}\n{stderr}")
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
    inherited_path: Option<&OsStr>,
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
                    inherited_path,
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
            inherited_path,
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
                inherited_path,
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
                    inherited_path,
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
    inherited_path: Option<&OsStr>,
    is_json_mode: bool,
) -> Result<Phase, RunnerError> {
    // Build Cargo selection before applying the runner-owned lint and environment policy.
    let (program, mut command_args) = cargo_invocation(args, runtime)?;
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
    // Separate Cargo's own arguments from the strict compiler policy arguments.
    // Place rustc and Clippy levels after Cargo's argument separator.
    command_args.push("--".into());
    command_args.extend(lint_level_args(args).into_iter().map(OsString::from));
    let mut environment = BTreeMap::new();
    set_environment(&mut environment, "CARGO_TARGET_DIR", target.as_os_str());
    set_environment(&mut environment, "RUSTC", runtime.rustc.as_os_str());
    configure_fast_environment(args, &mut environment);
    if args.use_repo_clippy_config.is_none() {
        // Materialize the private configuration only when repository policy is disabled.
        let config_dir = cache.join("clippy");
        write_file_if_changed(&config_dir.join("clippy.toml"), CLIPPY_CONFIG.as_bytes())?;
        set_environment(&mut environment, "CLIPPY_CONF_DIR", config_dir);
    }
    // Put the exact compiler tools ahead of any inherited Rust installation.
    prepend_path(&mut environment, &runtime.toolchain_bin(), inherited_path);
    Ok(Phase {
        name: "clippy".to_owned(),
        program,
        args: command_args,
        environment,
    })
}

/// Split the caller's Cargo wrapper into its executable and fixed prefix arguments.
fn cargo_invocation(
    args: &Cli,
    runtime: &Runtime,
) -> Result<(OsString, Vec<OsString>), RunnerError> {
    // Default to Cargo from the exact nightly that built this executable.
    let Some(cargo_command) = &args.cargo_cmd else {
        return Ok((runtime.cargo.as_os_str().to_owned(), Vec::new()));
    };
    // An explicit wrapper must still name an executable before its prefix arguments.
    let mut cargo = cargo_command.split_ascii_whitespace();
    let program = cargo.next().ok_or(RunnerError::MissingCargoCommand)?;
    Ok((program.into(), cargo.map(OsString::from).collect()))
}

/// Build one aggregate, category, or listing private-lint phase.
#[expect(
    abc_size,
    reason = "private-lint construction keeps Cargo and embedded-driver policy together"
)]
fn private_lint_phase(
    args: &Cli,
    repo: &Path,
    target: &Path,
    runtime: &Runtime,
    spec: PrivateLintPhaseSpec<'_>,
    inherited_path: Option<&OsStr>,
) -> Result<Phase, RunnerError> {
    // Every private phase re-enters this executable as the compiler driver.
    let executable = env::current_exe()
        .map_err(|source| RunnerError::external("could not resolve current executable", source))?;
    let mut environment = private_lint_environment(args, target, runtime, inherited_path);
    driver::configure_environment(
        &mut environment,
        driver::DriverSelection {
            categories: spec.categories,
            rustflags: &private_lint_args(args),
            is_list: spec.is_list,
            is_no_deps: args.no_deps.is_some() && !spec.is_list,
        },
    );
    // Listing runs rustc's help path, which registers lints without reading target source.
    if spec.is_list {
        return Ok(Phase {
            name: spec.name.to_owned(),
            program: executable.into_os_string(),
            args: vec!["rustc".into(), "-W".into(), "help".into()],
            environment,
        });
    }
    // Cargo check owns target selection; the workspace wrapper limits lints to local packages.
    let (program, mut command_args) = cargo_invocation(args, runtime)?;
    command_args.push("check".into());
    command_args.extend(selection_args(args, repo));
    if args.is_all_targets_selected() {
        command_args.push("--all-targets".into());
    }
    if spec.is_json_mode {
        command_args.push("--message-format=json".into());
    }
    set_environment(&mut environment, "RUSTC_WORKSPACE_WRAPPER", executable);
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
    inherited_path: Option<&OsStr>,
) -> BTreeMap<OsString, OsString> {
    // Keep target artifacts outside the repository and use the build-time nightly exactly.
    let mut environment = BTreeMap::new();
    set_environment(&mut environment, "CARGO_INCREMENTAL", "0");
    set_environment(&mut environment, "CARGO_TARGET_DIR", target.as_os_str());
    set_environment(&mut environment, "RUSTC", runtime.rustc.as_os_str());
    configure_fast_environment(args, &mut environment);

    // Cargo and rustc must resolve from the same sysroot as the statically linked driver.
    prepend_path(&mut environment, &runtime.toolchain_bin(), inherited_path);
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
fn prepend_path(
    environment: &mut BTreeMap<OsString, OsString>,
    path: &Path,
    inherited_path: Option<&OsStr>,
) {
    // Prefer phase-local PATH state and fall back to the captured parent value once.
    let mut paths = vec![path.to_path_buf()];
    let existing = environment
        .get(OsStr::new("PATH"))
        .cloned()
        .or_else(|| inherited_path.map(OsStr::to_os_string));
    if let Some(existing) = existing {
        paths.extend(env::split_paths(&existing));
    }
    // Retain the old PATH unchanged when platform joining rejects a component.
    if let Ok(joined) = env::join_paths(&paths) {
        set_environment(environment, "PATH", joined);
    }
}

/// Run one process, capture its streams when requested, and emit progress heartbeats.
fn run_phase(
    phase: &Phase,
    repo: &Path,
    heartbeat_interval: Duration,
    is_capture_output: bool,
) -> Result<PhaseResult, RunnerError> {
    // In ordinary mode, stream child output directly to avoid retaining large diagnostics.
    let started = Instant::now();
    let stream = || {
        if is_capture_output {
            Stdio::piped()
        } else {
            Stdio::inherit()
        }
    };
    let output = Command::new(&phase.program)
        .args(&phase.args)
        .envs(&phase.environment)
        .current_dir(repo)
        .stdout(stream())
        .stderr(stream())
        .spawn()
        .and_then(|child| wait_with_heartbeats(child, &phase.name, started, heartbeat_interval))
        .map_err(|source| {
            RunnerError::external(format!("could not run phase `{}`", phase.name), source)
        })?;
    Ok(PhaseResult {
        name: phase.name.clone(),
        command: render_command(phase),
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
        elapsed: started.elapsed(),
    })
}

/// Drain a child's pipes while a scoped sibling thread reports progress.
///
/// The heartbeat thread runs until the child wait ends.
fn wait_with_heartbeats(
    child: Child,
    name: &str,
    started: Instant,
    interval: Duration,
) -> io::Result<Output> {
    thread::scope(|scope| {
        let (finished, is_finished) = mpsc::channel::<()>();
        // A zero interval disables progress output and the extra thread.
        if !interval.is_zero() {
            let _heartbeat = scope.spawn(move || {
                emit_heartbeats(name, started, interval, &is_finished);
            });
        }
        let output = child.wait_with_output();
        // Closing the channel stops the heartbeat thread before the scope joins it.
        drop(finished);
        output
    })
}

/// Print one progress line per interval until the phase's completion channel closes.
fn emit_heartbeats(
    name: &str,
    started: Instant,
    interval: Duration,
    is_finished: &mpsc::Receiver<()>,
) {
    // A closed channel ends the loop; a timeout means the child is still running.
    while is_finished.recv_timeout(interval) == Err(mpsc::RecvTimeoutError::Timeout) {
        let elapsed = started.elapsed().as_secs_f64();
        // Progress output is advisory, so a closed stdout must not abort the running phase.
        drop(write_stdout(&format!(
            "{name}: still running ({elapsed:.0}s elapsed)\n"
        )));
    }
}

/// Set one child environment value while deliberately replacing inherited policy.
fn set_environment(
    environment: &mut BTreeMap<OsString, OsString>,
    key: impl Into<OsString>,
    value: impl Into<OsString>,
) {
    let _previous = environment.insert(key.into(), value.into());
}

/// Resolve a writable cache directory outside the target repository.
///
/// Relative candidates are skipped, as the XDG base-directory specification
/// requires for a relative `XDG_CACHE_HOME`, so the cache never depends on the
/// caller's working directory.
fn cache_root(candidates: &[PathBuf], repo: &Path) -> Result<PathBuf, RunnerError> {
    // Use the first caller-prioritized directory that can be created safely.
    for candidate in candidates {
        // Reject relative and repository-contained paths before creating directories.
        let is_safe = candidate.is_absolute() && !resolves_inside(candidate, repo);
        if is_safe && fs::create_dir_all(candidate).is_ok() {
            return Ok(candidate.clone());
        }
    }
    Err(RunnerError::NoWritableCache {
        candidates: candidates.to_vec(),
    })
}

/// Return whether a possibly missing path resolves inside the canonical repository.
///
/// The deepest existing ancestor is resolved, so a symbolic link into the
/// repository is rejected before the runner creates anything below it.
fn resolves_inside(path: &Path, repo: &Path) -> bool {
    path.ancestors()
        .find_map(|ancestor| fs::canonicalize(ancestor).ok())
        .is_some_and(|resolved| resolved.starts_with(repo))
}

/// Resolve the caller-selected or runner-managed Cargo target directory.
fn target_directory(
    args: &Cli,
    repo: &Path,
    cache: &Path,
    inherited_target: Option<&Path>,
) -> TargetDirectory {
    // An explicit `--target-dir` always wins and is never pruned.
    if let Some(path) = &args.target_dir {
        return TargetDirectory {
            path: resolve_target_path(path, repo),
            is_managed: false,
        };
    }
    // Preserve an external Cargo target directory supplied by a benchmark or CI.
    if let Some(path) = inherited_target {
        let path = resolve_target_path(path, repo);
        if is_external_path(&path, repo) && !resolves_inside(&path, repo) {
            return TargetDirectory {
                path,
                is_managed: false,
            };
        }
    }
    // Otherwise, namespace one runner-owned directory per canonical repository.
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

/// Parse the managed target-cache size limit, where zero disables pruning.
#[expect(
    runtime_env_read,
    reason = "the cache limit is an explicit runner configuration boundary"
)]
fn target_cache_limit() -> Result<Option<u64>, RunnerError> {
    // Keep the default limit when the caller did not configure one.
    let Some(value) = env::var_os(TARGET_CACHE_MAX_BYTES_ENV) else {
        return Ok(Some(DEFAULT_TARGET_CACHE_MAX_BYTES));
    };
    // Accept only an unsigned decimal byte count; every zero spelling disables pruning.
    value
        .to_str()
        .and_then(|text| text.parse::<u64>().ok())
        .map(|limit| (limit != 0).then_some(limit))
        .ok_or_else(|| RunnerError::InvalidTargetCacheLimit(value.to_string_lossy().into_owned()))
}

/// Measure a directory without following symbolic links.
fn directory_size(path: &Path) -> io::Result<u64> {
    // A symbolic link or special file counts only its own metadata length.
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() {
        return Ok(metadata.len());
    }
    // Saturate instead of overflowing on implausibly large trees.
    fs::read_dir(path)?.try_fold(0_u64, |size, entry| {
        Ok(size.saturating_add(directory_size(&entry?.path())?))
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
    write_file(
        &root.join(format!("{}.stdout", result.name)),
        &result.stdout,
    )?;
    // Preserve stderr separately so raw tool diagnostics remain inspectable.
    write_file(
        &root.join(format!("{}.stderr", result.name)),
        &result.stderr,
    )?;
    // End the command file with a newline so shell and editor views remain complete.
    write_file(
        &root.join(format!("{}.command", result.name)),
        format!("{}\n", result.command).as_bytes(),
    )
}

/// Build the machine-readable timing report for every retained phase result.
fn timing_report(results: &[PhaseResult], total: Duration) -> TimingReport<'_> {
    let phases = results
        .iter()
        .map(|result| TimingPhase {
            name: &result.name,
            elapsed_seconds: result.elapsed.as_secs_f64(),
            return_code: status_code(result.status),
        })
        .collect();
    TimingReport {
        total_seconds: total.as_secs_f64(),
        phases,
    }
}

/// Write one pretty JSON document without a byte order mark.
///
/// The document ends with a newline for shell and editor consumers.
fn write_json(path: &Path, value: &impl Serialize) -> Result<(), RunnerError> {
    // Serialization and filesystem failures share the report path as their context.
    serde_json::to_vec_pretty(value)
        .map_err(io::Error::from)
        .and_then(|mut bytes| {
            bytes.push(b'\n');
            write_bytes(path, &bytes)
        })
        .map_err(filesystem_error(path))
}

/// Create a directory tree with its affected path in any failure.
fn create_dir(path: &Path) -> Result<(), RunnerError> {
    fs::create_dir_all(path).map_err(filesystem_error(path))
}

/// Write a complete file and create its parent directory first.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), RunnerError> {
    write_bytes(path, bytes).map_err(filesystem_error(path))
}

/// Write a complete file after creating its parent directory.
fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

/// Return a converter that attaches one affected path to a filesystem failure.
fn filesystem_error(path: &Path) -> impl FnOnce(io::Error) -> RunnerError + '_ {
    move |source| {
        RunnerError::external(
            format!("filesystem operation failed for {}", path.display()),
            source,
        )
    }
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
    let is_shell_safe = !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_=+.,/:@".contains(&byte));
    if is_shell_safe {
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
        .and_then(|()| stdout.flush())
        .map_err(output_error)
}

/// Forward captured child output to its original stream.
fn write_raw_output(stdout: &[u8], stderr: &[u8]) -> Result<(), RunnerError> {
    io::stdout()
        .lock()
        .write_all(stdout)
        .and_then(|()| io::stderr().lock().write_all(stderr))
        .map_err(output_error)
}

/// Attach runner-output context to a closed or failing standard stream.
fn output_error(source: io::Error) -> RunnerError {
    RunnerError::external("could not write runner output", source)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        ffi::{OsStr, OsString},
        path::{Path, PathBuf},
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    use clap::Parser as _;

    use super::{
        AGGRESSIVE_CLIPPY_LINTS, CLIPPY_CONFIG, Phase, STRICT_CLIPPY_LINTS, build_phases,
        is_external_path, lint_level_args, log_root, prepend_path, private_lint_args,
        quote_argument, repository_label, selection_args, target_directory, wait_with_heartbeats,
    };
    use crate::{cli::Cli, runtime::Runtime};

    /// Parse runner arguments after the executable name.
    fn cli(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("sagan-lints").chain(args.iter().copied())).unwrap()
    }

    /// Convert rendered process arguments to UTF-8 strings.
    fn strings(values: &[OsString]) -> Vec<String> {
        values
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect()
    }

    /// Return one phase environment value as UTF-8.
    fn variable<'phase>(phase: &'phase Phase, key: &str) -> Option<&'phase OsStr> {
        phase
            .environment
            .get(OsStr::new(key))
            .map(OsString::as_os_str)
    }

    /// Build phases against a fixed runtime and a temporary cache.
    fn phases(args: &[&str], is_json_mode: bool) -> (tempfile::TempDir, Vec<Phase>) {
        let cache = tempfile::tempdir().unwrap();
        let runtime = Runtime {
            cargo: PathBuf::from("/toolchain/bin/cargo"),
            rustc: PathBuf::from("/toolchain/bin/rustc"),
            sysroot: PathBuf::from("/toolchain"),
            toolchain_name: "nightly-test",
        };
        let phases = build_phases(
            &cli(args),
            Path::new("/repo"),
            cache.path(),
            Path::new("/target"),
            &runtime,
            Some(OsStr::new("usr/bin")),
            is_json_mode,
        )
        .unwrap();
        (cache, phases)
    }

    /// A zero heartbeat interval still waits for the child and captures its output.
    #[test]
    fn wait_with_heartbeats_captures_zero_interval_output() {
        // Exercise the disabled-heartbeat branch without introducing a background thread.
        let child = Command::new("printf")
            .arg("runner-test")
            .stdout(Stdio::piped())
            .spawn()
            .expect("printf should be available");
        let output = wait_with_heartbeats(child, "unit", Instant::now(), Duration::ZERO)
            .expect("the child should finish");
        assert!(output.status.success());
        assert_eq!(output.stdout, b"runner-test");
    }

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
        // Resolve the caller's relative target against an isolated absolute repository.
        let args = cli(&["--target-dir", "build-target"]);
        // Keep the repository and cache paths absolute while avoiding shared state.
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("example-repository");
        let cache = root.path().join("sagan-cache");
        let target = target_directory(&args, &repo, &cache, None);

        assert_eq!(target.path, repo.join("build-target"));
        assert!(!target.is_managed);
    }

    #[test]
    fn default_target_directory_is_namespaced_under_the_cache() {
        // Managed targets must remain under the cache namespace for cleanup.
        let args = cli(&["--fast"]);
        // Use an absolute temporary root so namespace checks match production paths.
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("example-repository");
        let cache = root.path().join("sagan-cache");
        let target = target_directory(&args, &repo, &cache, None);

        assert!(target.is_managed);
        assert!(target.path.starts_with(cache.join("targets")));
    }

    /// An inherited Cargo target is kept only when it lies outside the repository.
    #[test]
    fn inherited_target_directory_must_be_external() {
        // Compare a dynamic absolute target with a relative target candidate.
        let args = cli(&[]);
        // Keep repository, cache, and CI target paths in one isolated absolute root.
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("example-repository");
        let cache = root.path().join("sagan-cache");
        let ci_target = root.path().join("sagan-lints-ci-target");

        let external = target_directory(&args, &repo, &cache, Some(&ci_target));
        // A relative target remains managed even when an external absolute target is accepted.
        let relative_target = repo.join("target");
        let relative = target_directory(&args, &repo, &cache, Some(&relative_target));

        assert_eq!(external.path, ci_target);
        assert!(!external.is_managed);
        assert!(relative.is_managed);
    }

    #[test]
    fn ordinary_runs_do_not_create_a_log_directory() {
        assert!(log_root(&cli(&[])).unwrap().is_none());
    }

    #[test]
    fn parent_directory_components_are_not_treated_as_external() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("example-repository");
        let outside = repo.join("../outside");
        assert!(!is_external_path(&outside, &repo));
    }

    /// Repository labels keep a portable byte set and name the root directory.
    #[test]
    fn repository_labels_are_filesystem_safe() {
        assert_eq!(repository_label(Path::new("/")), "repo");
        assert_eq!(
            repository_label(Path::new("/work/my repo.v2")),
            "my_repo_v2"
        );
    }

    /// Display quoting leaves safe arguments bare and single-quotes the rest.
    #[test]
    fn quotes_shell_unsafe_arguments() {
        assert_eq!(
            quote_argument(OsStr::new("--features=a,b")),
            "--features=a,b"
        );
        assert_eq!(quote_argument(OsStr::new("")), "''");
        assert_eq!(quote_argument(OsStr::new("it's")), r"'it'\''s'");
    }

    /// Cargo selection flags keep Cargo's documented order and the caller's escape hatches.
    #[test]
    fn selection_arguments_preserve_cargo_order() {
        let args = cli(&[
            "--manifest-path",
            "crates/a/Cargo.toml",
            "-p",
            "a",
            "-p",
            "b",
            "--exclude",
            "c",
            "--all-features",
            "--no-default-features",
            "--features",
            "x y",
            "--target",
            "wasm32-unknown-unknown",
            "--locked",
            "--frozen",
            "--offline",
            "--extra-cargo-arg=--lib",
        ]);

        assert_eq!(
            strings(&selection_args(&args, Path::new("/repo"))),
            [
                "--manifest-path",
                "/repo/crates/a/Cargo.toml",
                "--package",
                "a",
                "--package",
                "b",
                "--exclude",
                "c",
                "--all-features",
                "--no-default-features",
                "--features",
                "x y",
                "--target",
                "wasm32-unknown-unknown",
                "--locked",
                "--frozen",
                "--offline",
                "--lib",
            ]
        );
    }

    /// Workspace selection applies only without packages or a manifest.
    #[test]
    fn workspace_selection_yields_to_narrower_selection() {
        let repo = Path::new("/repo");

        assert_eq!(strings(&selection_args(&cli(&[]), repo)), ["--workspace"]);
        assert!(selection_args(&cli(&["--no-workspace"]), repo).is_empty());
        assert_eq!(
            strings(&selection_args(
                &cli(&["--manifest-path", "/elsewhere/Cargo.toml"]),
                repo
            )),
            ["--manifest-path", "/elsewhere/Cargo.toml"]
        );
    }

    /// Lint levels follow the public precedence through all caller overrides.
    ///
    /// Raw compiler arguments remain the final precedence layer.
    #[test]
    fn lint_levels_follow_cli_precedence() {
        // Build strict and relaxed argument sets for the two policy branches.
        let strict = lint_level_args(&cli(&["--rustc-lint", "custom_rustc"]));
        let relaxed = lint_level_args(&cli(&[
            "--no-deny-warnings",
            "--no-strict-rustc",
            "--aggressive-clippy",
            "--clippy-lint",
            "clippy::custom",
            "--allow-clippy-lint",
            "clippy::waived",
            "--extra-rustc-arg=--cap-lints=warn",
        ]));

        // Derive each precedence facet before checking the complete policy tuple.
        let has_strict_deny = strict.starts_with(&["-D".to_owned(), "warnings".to_owned()]);
        let has_custom_rustc = strict.windows(2).any(|pair| pair == ["-W", "custom_rustc"]);
        let has_no_deny_warnings = relaxed.first().is_none_or(|level| level != "-D");
        let has_no_custom_rustc = !relaxed.contains(&"custom_rustc".to_owned());
        let has_all_aggressive_lints = AGGRESSIVE_CLIPPY_LINTS
            .lines()
            .all(|lint| relaxed.contains(&lint.to_owned()));
        let ends_with_caller_overrides = relaxed.ends_with(
            &[
                "-W",
                "clippy::custom",
                "-A",
                "clippy::waived",
                "--cap-lints=warn",
            ]
            .map(String::from),
        );
        assert_eq!(
            (
                has_strict_deny,
                has_custom_rustc,
                has_no_deny_warnings,
                has_no_custom_rustc,
                has_all_aggressive_lints,
                ends_with_caller_overrides,
            ),
            (true, true, true, true, true, true)
        );
    }

    /// Repository-policy lints are allowed unless the caller opts in.
    #[test]
    fn private_lint_levels_allow_repository_policy_by_default() {
        let external = private_lint_args(&cli(&[]));
        let repository = private_lint_args(&cli(&["--include-repo-policy-lints"]));

        assert!(
            external
                .windows(2)
                .any(|pair| pair == ["-A", "unknown_lints"])
        );
        assert_eq!(repository, ["-D", "warnings"]);
    }

    /// The default selection runs strict Clippy and one aggregate private-lint check.
    #[test]
    fn default_phases_run_clippy_then_private_lints() {
        // Build both ordered phases with the embedded-driver environment.
        let (cache, phases) = phases(&["--fast", "--no-deps"], true);
        let names = phases
            .iter()
            .map(|phase| phase.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["clippy", "dylint"]);
        assert_default_clippy_phase(&cache, &phases[0]);
        assert_default_private_lint_phase(&phases[1]);
    }

    /// Verify strict Clippy's arguments, cache, and toolchain environment.
    fn assert_default_clippy_phase(cache: &tempfile::TempDir, clippy: &Phase) {
        // Compare the complete Clippy command contract in one structured assertion.
        let clippy_args = strings(&clippy.args)
            .into_iter()
            .take(5)
            .collect::<Vec<_>>();
        let clippy_config = variable(clippy, "CLIPPY_CONF_DIR").map(PathBuf::from);
        let clippy_debug = variable(clippy, "CARGO_PROFILE_TEST_DEBUG");
        assert_eq!(
            (clippy_args, clippy_config, clippy_debug,),
            (
                vec![
                    "clippy".to_owned(),
                    "--workspace".to_owned(),
                    "--no-deps".to_owned(),
                    "--message-format=json".to_owned(),
                    "--".to_owned(),
                ],
                Some(cache.path().join("clippy")),
                Some(OsStr::new("0")),
            )
        );
    }

    /// Verify the aggregate private-lint command and its driver environment.
    fn assert_default_private_lint_phase(dylint: &Phase) {
        // The private phase receives no dependencies and uses the embedded driver wrapper.
        let dylint_args = strings(&dylint.args);
        let no_deps = variable(dylint, "SAGAN_LINTS_DRIVER_NO_DEPS");
        let has_wrapper = variable(dylint, "RUSTC_WORKSPACE_WRAPPER").is_some();
        let has_toolchain_path = variable(dylint, "PATH")
            .is_some_and(|path| path.to_string_lossy().starts_with("/toolchain/bin"));
        assert_eq!(
            (dylint_args, no_deps, has_wrapper, has_toolchain_path,),
            (
                ["check", "--workspace", "--message-format=json"]
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<_>>(),
                Some(OsStr::new("1")),
                true,
                true,
            )
        );
    }

    /// Category selection keeps repeated categories, and wrappers prefix every Cargo phase.
    #[test]
    fn category_phases_preserve_order_and_cargo_wrapper() {
        // Build repeated category phases through a shell Cargo wrapper.
        let (_cache, phases) = phases(
            &[
                "--cargo-cmd",
                "nix develop -c cargo",
                "--use-repo-clippy-config",
                "--dylint-category",
                "style",
                "--dylint-category",
                "perf",
            ],
            false,
        );

        let names = phases
            .iter()
            .map(|phase| {
                (
                    phase.name.as_str(),
                    phase.program.to_string_lossy().into_owned(),
                )
            })
            .collect::<Vec<_>>();
        let wrapped_args = strings(&phases[1].args);
        let has_clippy_config = variable(&phases[0], "CLIPPY_CONF_DIR").is_some();
        let categories = variable(&phases[2], "SAGAN_LINTS_DRIVER_CATEGORIES");
        // Compare names, wrapper arguments, and policy environment together.
        assert_eq!(
            (names, wrapped_args, has_clippy_config, categories),
            (
                vec![
                    ("clippy", "nix".to_owned()),
                    ("dylint-style", "nix".to_owned()),
                    ("dylint-perf", "nix".to_owned()),
                ],
                vec![
                    "develop".to_owned(),
                    "-c".to_owned(),
                    "cargo".to_owned(),
                    "check".to_owned(),
                    "--workspace".to_owned(),
                    "--all-targets".to_owned(),
                ],
                false,
                Some(OsStr::new("perf")),
            )
        );
    }

    /// Listing runs the executable directly and never selects no-deps filtering.
    #[test]
    fn list_phases_reenter_the_executable() {
        let (_cache, phases) = phases(&["--list-private-lints", "--no-deps"], false);

        assert_eq!(phases.len(), 9);
        assert!(phases.iter().all(|phase| {
            strings(&phase.args) == ["rustc", "-W", "help"]
                && variable(phase, "SAGAN_LINTS_DRIVER_LIST").is_some()
                && variable(phase, "SAGAN_LINTS_DRIVER_NO_DEPS").is_none()
        }));
    }

    /// Skipping both suites selects no phases.
    #[test]
    fn skipping_every_suite_selects_no_phases() {
        assert!(
            phases(&["--skip-clippy", "--skip-dylint"], false)
                .1
                .is_empty()
        );
    }

    /// A phase-local PATH is extended, and an unjoinable entry leaves it unchanged.
    #[test]
    fn prepend_path_extends_phase_path() {
        // Verify normal joining and the unchanged value after an unjoinable path.
        let root = tempfile::tempdir().unwrap();
        let inherited = root.path().join("usr-bin");
        let toolchain = root.path().join("toolchain/bin");
        let bad_entry = root.path().join("bad:entry");
        let mut environment =
            BTreeMap::from([(OsString::from("PATH"), inherited.clone().into_os_string())]);

        prepend_path(&mut environment, &toolchain, Some(inherited.as_os_str()));
        prepend_path(&mut environment, &bad_entry, Some(inherited.as_os_str()));

        // The failed join must leave the valid toolchain and inherited entries intact.
        let expected_path = std::env::join_paths([toolchain, inherited]).unwrap();
        assert_eq!(environment.get(OsStr::new("PATH")), Some(&expected_path));
    }
}
