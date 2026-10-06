//! Instrument Dylint UI driver processes and publish canonical coverage
//! reports.

use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

use clap::Args;

use crate::{error::Error, lcov, paths, percentage::Percentage, process};

/// Coverage inputs validated before replacing generated artifacts.
#[derive(Debug, Args)]
pub(crate) struct Coverage {
    /// Minimum canonical executable-line coverage percentage.
    #[arg(long, default_value = "0")]
    min_lines: Percentage,
    /// Restrict the report to Rust sources within this directory; repeat as
    /// needed.
    #[arg(long = "path")]
    paths: Vec<PathBuf>,
    /// Dedicated directory for instrumented build outputs and reports.
    #[arg(long, default_value = "target/coverage")]
    target_dir: PathBuf,
    /// Cargo test selection after the separator.
    #[arg(last = true)]
    test_args: Vec<OsString>,
}

impl Coverage {
    /// Run selected tests, retain reports even on test failure, and enforce
    /// coverage.
    pub(crate) fn run(
        &self,
        root: &Path,
        target_override: Option<&OsStr>,
        rustflags: Option<OsString>,
    ) -> Result<(), Error> {
        // Validate selections before replacing coverage-owned outputs.
        let (selected, target) = self.prepare(root, target_override)?;
        // Preserve reports from unsuccessful tests before returning their status.
        let status = self.test(root, &target, rustflags)?;
        let percent = measured_report(&target, root, &selected)?;
        self.check_result(status, percent)
    }

    /// Resolve every input before replacing coverage-owned outputs.
    fn prepare(
        &self,
        root: &Path,
        target_override: Option<&OsStr>,
    ) -> Result<(BTreeSet<PathBuf>, PathBuf), Error> {
        // Reject an invalid selection before any generated output is replaced.
        let selected = paths::sources(&self.paths, root)?;
        let target = paths::coverage_target(
            target_override.map_or(self.target_dir.as_path(), Path::new),
            root,
        )?;
        // Replace only the paths that the coverage task owns.
        prepare(&target)?;
        Ok((selected, target))
    }

    /// Preserve test failure before applying the configured minimum.
    fn check_result(&self, status: std::process::ExitStatus, percent: f64) -> Result<(), Error> {
        // Failed tests remain a failure even when their partial profiles exceed the minimum.
        if !status.success() {
            return Err(Error::Process {
                program: "cargo test".to_owned(),
                status,
            });
        }
        // Compare the canonical rate only after the test outcome has passed.
        let minimum = self.min_lines.value();
        if percent < minimum {
            return Err(Error::Invalid(format!(
                "line coverage {percent:.2}% is below the {minimum}% minimum"
            )));
        }
        Ok(())
    }
    /// Instrument the test process and every child lint-library build.
    fn test(
        &self,
        root: &Path,
        target: &Path,
        rustflags: Option<OsString>,
    ) -> Result<std::process::ExitStatus, Error> {
        // Preserve linked variants without changing compiler commands inspected by UI tests.
        let linker = linker_variable()?;
        let wrapper = build_wrapper(root, target, &linker)?;
        // Default to every ordinary unit, UI and integration-test target.
        let test_args = if self.test_args.is_empty() {
            vec![
                "--workspace".into(),
                "--lib".into(),
                "--bins".into(),
                "--tests".into(),
            ]
        } else {
            self.test_args.clone()
        };
        // Preserve caller flags while adding the matching LLVM instrumentation.
        let mut flags = rustflags.unwrap_or_default();
        flags.push(" -C instrument-coverage");
        // The same environment instruments libraries built by dylint_testing's child Cargo.
        let status = instrumented_cargo(root, target, &wrapper, &linker)
            .args(["test", "--no-fail-fast"])
            .args(&test_args)
            .env("RUSTFLAGS", flags)
            .status()?;
        Ok(status)
    }
}

/// Keep instrumented builds, linked variants and profiles in one owned directory.
fn instrumented_cargo(root: &Path, target: &Path, wrapper: &Path, linker: &str) -> Command {
    // Override both Cargo directories so nested builds retain the same measurement scope.
    let mut command = Command::new("cargo");
    let _configured = command
        .current_dir(root)
        .env("CARGO_TARGET_DIR", target)
        .env("CARGO_BUILD_BUILD_DIR", target)
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env(linker, wrapper)
        .env("DYLINT_COVERAGE_OBJECTS", target.join("objects"))
        .env("LLVM_PROFILE_FILE", target.join("profiles/%p-%m.profraw"));
    command
}

/// Build one measured report and expose its exact output location.
fn measured_report(target: &Path, root: &Path, selected: &BTreeSet<PathBuf>) -> Result<f64, Error> {
    // Use LLVM tools from the compiler that generated these profiles.
    let tools = llvm_tools()?;
    merge(target, &tools)?;
    let percent = report(target, &tools, root, selected)?;
    // Keep the diagnostic tied to the generated artifact, including failing tests.
    let directory = target.join("report");
    let display = directory.display();
    writeln!(
        std::io::stdout().lock(),
        "canonical line coverage: {percent:.2}%\nreports: {display}"
    )?;
    Ok(percent)
}

/// Replace only coverage-owned build and report outputs in a validated
/// directory.
fn prepare(target: &Path) -> Result<(), Error> {
    // These paths belong exclusively to the validated coverage directory.
    for name in ["debug", "report", "profiles", "objects"] {
        let path = target.join(name);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
    }
    // Fresh report and profile directories retain evidence from the next test run.
    fs::create_dir_all(target.join("report"))?;
    fs::create_dir_all(target.join("profiles"))?;
    fs::create_dir_all(target.join("objects"))?;
    Ok(())
}

/// Build the wrapper outside instrumented outputs so nested Cargo builds
/// retain one executable.
fn build_wrapper(root: &Path, target: &Path, linker: &str) -> Result<PathBuf, Error> {
    let directory = target.join("wrapper");
    process::run(
        Command::new("cargo")
            .current_dir(root)
            .args([
                "build",
                "--package",
                "xtask",
                "--bin",
                "coverage-link",
                "--target-dir",
            ])
            .arg(&directory)
            .env("CARGO_BUILD_BUILD_DIR", &directory)
            .env("RUSTFLAGS", "")
            .env(linker, "dylint-link")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .env_remove("DYLINT_COVERAGE_OBJECTS"),
    )?;
    Ok(directory.join("debug/coverage-link"))
}

/// Locate LLVM tools from the same compiler used to instrument the workspace.
fn llvm_tools() -> Result<PathBuf, Error> {
    // Read both values from the active compiler to avoid mixing LLVM versions.
    let sysroot = process::output(Command::new("rustc").args(["--print", "sysroot"]))?;
    let host = compiler_host()?;
    let sysroot = String::from_utf8_lossy(&sysroot.stdout);
    Ok(Path::new(sysroot.trim())
        .join("lib/rustlib")
        .join(host)
        .join("bin"))
}

/// Read the active compiler host before selecting matching tools and linker configuration.
fn compiler_host() -> Result<String, Error> {
    let version = process::output(Command::new("rustc").arg("-vV"))?;
    String::from_utf8_lossy(&version.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
        .ok_or_else(|| Error::Invalid("rustc -vV has no host triple".to_owned()))
}

/// Cargo uses uppercase target names with hyphens replaced by underscores.
fn linker_variable() -> Result<String, Error> {
    let host = compiler_host()?.replace('-', "_").to_ascii_uppercase();
    Ok(format!("CARGO_TARGET_{host}_LINKER"))
}

/// Merge profiles from every test binary and dynamically loaded lint library.
fn merge(target: &Path, tools: &Path) -> Result<(), Error> {
    // Pass profile paths through an input file instead of a shell or argument expansion.
    let mut list = fs::File::create(target.join("profiles.txt"))?;
    // Only raw instrumentation profiles belong in the LLVM merge input.
    for entry in walkdir::WalkDir::new(target.join("profiles")) {
        let entry = entry?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "profraw")
        {
            let path = entry.path().display();
            writeln!(list, "{path}")?;
        }
    }
    // Merge every child-process profile into one sparse instrumentation profile.
    let list = target.join("profiles.txt");
    let display = list.display();
    process::run(
        Command::new(tools.join("llvm-profdata"))
            .args(["merge", "-sparse"])
            .arg(format!("--input-files={display}"))
            .arg("-o")
            .arg(target.join("coverage.profdata")),
    )
}

/// Select executables and shared libraries without including linker scripts.
fn objects(target: &Path) -> Result<Vec<PathBuf>, Error> {
    // Limit discovery to Cargo binaries and immediate dependency artifacts.
    let mut result = Vec::new();
    for entry in walkdir::WalkDir::new(target.join("debug"))
        .max_depth(2)
        .into_iter()
        .chain(walkdir::WalkDir::new(target.join("objects")).max_depth(1))
    {
        let entry = entry?;
        if is_reportable(&entry)? {
            result.push(entry.into_path());
        }
    }
    // Stable object order keeps generated commands and reports reproducible.
    result.sort();
    if result.is_empty() {
        return Err(Error::Invalid(
            "coverage build produced no reportable objects".to_owned(),
        ));
    }
    Ok(result)
}

/// Exclude Cargo linker scripts and build helpers before inspecting
/// permissions.
fn is_reportable(entry: &walkdir::DirEntry) -> Result<bool, Error> {
    let name = entry.file_name().to_string_lossy();
    Ok(entry.file_type().is_file()
        && !name.starts_with("build-script-")
        && !name.ends_with(".d")
        && (name.ends_with(std::env::consts::DLL_SUFFIX) || is_executable(entry)?))
}

/// Identify executables on the supported Unix platforms.
fn is_executable(entry: &walkdir::DirEntry) -> Result<bool, Error> {
    use std::os::unix::fs::PermissionsExt as _;
    Ok(entry.metadata()?.permissions().mode() & 0o111 != 0)
}

/// Construct an LLVM report command with one positional object and explicit
/// extras.
fn llvm_command(tools: &Path, operation: &str, target: &Path, objects: &[PathBuf]) -> Command {
    // Exclude generated files and intentional counterexample fixtures from every report.
    let mut command = Command::new(tools.join("llvm-cov"));
    let _configured = command.arg(operation).arg("--instr-profile").arg(target.join("coverage.profdata"))
        .arg("--ignore-filename-regex").arg("(/\\.cargo/registry/|/\\.rustup(-dylint)?/|/rustc/|/nix/store/|/examples/|/ui/|/fixtures/|/fixture/|/target/)");
    // LLVM accepts one positional object followed by explicitly named additional objects.
    if let Some((first, rest)) = objects.split_first() {
        let _configured = command.arg(first);
        for object in rest {
            let _configured = command.arg("--object").arg(object);
        }
    }
    command
}

/// Preserve LLVM warnings separately from report data.
fn capture(command: &mut Command, target: &Path) -> Result<Vec<u8>, Error> {
    // Save stderr independently so LLVM warnings never corrupt exported data.
    let result = command.output()?;
    let mut diagnostics = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target.join("report/llvm-cov.stderr"))?;
    diagnostics.write_all(&result.stderr)?;
    // Propagate tool failure after preserving its diagnostics.
    if !result.status.success() {
        return Err(Error::Process {
            program: "llvm-cov".to_owned(),
            status: result.status,
        });
    }
    Ok(result.stdout)
}

/// Emit raw LLVM reports, canonical summary and browsable HTML from one
/// profile.
fn report(
    target: &Path,
    tools: &Path,
    root: &Path,
    selected: &BTreeSet<PathBuf>,
) -> Result<f64, Error> {
    // Derive all report formats from the same objects and merged profile.
    let objects = objects(target)?;
    let summary = read_summary(target, tools, &objects)?;
    scoped_reports(target, tools, root, selected, &objects, summary)
}

/// Read LLVM's raw mapping inventory before applying source selection.
fn read_summary(target: &Path, tools: &Path, objects: &[PathBuf]) -> Result<LlvmReport, Error> {
    let raw = capture(
        llvm_command(tools, "export", target, objects).arg("--summary-only"),
        target,
    )?;
    Ok(serde_json::from_slice(&raw)?)
}

/// Apply one scope to canonical totals and every browsable LLVM report.
fn scoped_reports(
    target: &Path,
    tools: &Path,
    root: &Path,
    selected: &BTreeSet<PathBuf>,
    objects: &[PathBuf],
    mut summary: LlvmReport,
) -> Result<f64, Error> {
    // Resolve lexical aliases before excluding unselected filenames in every LLVM output.
    let ignore = scoped_ignore(&summary, root, selected)?;
    let command = |operation| {
        let mut command = llvm_command(tools, operation, target, objects);
        if let Some(ignore) = &ignore {
            let _configured = command.arg("--ignore-filename-regex").arg(ignore);
        }
        command
    };
    // Canonical totals merge source aliases; raw LLVM totals retain every mapping.
    let percent = canonical_report(target, tools, root, selected, objects, ignore.as_deref())?;
    raw_reports(
        target,
        root,
        selected,
        &mut summary,
        &mut command("report"),
        &mut command("show"),
    )?;
    Ok(percent)
}

/// Retain filtered raw totals and their matching text and HTML artifacts.
fn raw_reports(
    target: &Path,
    root: &Path,
    selected: &BTreeSet<PathBuf>,
    summary: &mut LlvmReport,
    text: &mut Command,
    html: &mut Command,
) -> Result<(), Error> {
    filter_summary(summary, root, selected)?;
    write_json(&target.join("report/summary.json"), summary)?;
    // The same exclusion expression governs both browsable formats.
    llvm_reports(target, text, html)
}

/// Serialize one report artifact without mixing tool diagnostics into its
/// bytes.
fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), Error> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes)?;
    Ok(())
}

/// Normalize executable-line identities and write the canonical coverage
/// artifacts.
fn canonical_report(
    target: &Path,
    tools: &Path,
    root: &Path,
    selected: &BTreeSet<PathBuf>,
    objects: &[PathBuf],
    ignore: Option<&str>,
) -> Result<f64, Error> {
    let lines = object_lines(target, tools, root, selected, objects, ignore)?;
    // Stable source and line order exposes the exact untested production lines.
    canonical_artifacts(&lines, &target.join("report"))?;
    Ok(lcov::metric(&lines).percent)
}

/// Export binary variants independently before merging canonical executable-line hits.
fn object_lines(
    target: &Path,
    tools: &Path,
    root: &Path,
    selected: &BTreeSet<PathBuf>,
    objects: &[PathBuf],
    ignore: Option<&str>,
) -> Result<lcov::Lines, Error> {
    // LLVM deduplicates function mappings across objects; export variants separately first.
    let mut lines = lcov::Lines::new();
    let mut raw_file = fs::File::create(target.join("report/lcov.info"))?;
    for object in objects {
        let mut command = llvm_command(tools, "export", target, std::slice::from_ref(object));
        if let Some(ignore) = ignore {
            let _configured = command.arg("--ignore-filename-regex").arg(ignore);
        }
        // Preserve each raw export before canonicalizing its source paths.
        let raw = capture(command.arg("--format=lcov"), target)?;
        raw_file.write_all(&raw)?;
        // A line reached by any retained compiler variant remains covered.
        for (identity, hits) in lcov::parse(&String::from_utf8_lossy(&raw), root, selected)? {
            let previous = lines.entry(identity).or_default();
            *previous = (*previous).max(hits);
        }
    }
    Ok(lines)
}

/// Write canonical totals and their exact measured line inventory.
fn canonical_artifacts(lines: &lcov::Lines, report: &Path) -> Result<(), Error> {
    write_json(
        &report.join("canonical_summary.json"),
        &lcov::summary(lines),
    )?;
    write_lines(lines, report)
}

/// Export LLVM text and HTML using the same selected mapping scope.
fn llvm_reports(target: &Path, text: &mut Command, html: &mut Command) -> Result<(), Error> {
    // Retain text totals alongside the same profile used by HTML.
    let report = target.join("report");
    fs::write(report.join("summary.txt"), capture(text, target)?)?;
    // LLVM creates the browsable HTML tree inside the retained report directory.
    let display = report.display();
    let _configured = html
        .args(["--format=html", "--show-line-counts-or-regions"])
        .arg(format!("--output-dir={display}"));
    let _html = capture(html, target)?;
    Ok(())
}

/// Write stable canonical line identities and exact zero-hit gaps.
fn write_lines(lines: &lcov::Lines, report: &Path) -> Result<(), Error> {
    // Separate all measured lines from zero-hit gaps while preserving canonical order.
    let mut table = fs::File::create(report.join("canonical-lines.tsv"))?;
    let mut gaps = fs::File::create(report.join("canonical_gaps.txt"))?;
    // Every gap refers to the same source identity used by the threshold.
    for ((file, line), hits) in lines {
        // Tabular and gap records use the same canonical path spelling.
        let display = file.display();
        writeln!(table, "{display}\t{line}\t{hits}")?;
        if *hits == 0 {
            writeln!(gaps, "{display}:{line}")?;
        }
    }
    Ok(())
}

/// Escape an exact filename for LLVM's regular-expression exclusion interface.
fn escape_filename(filename: &str) -> String {
    // Escape only regex metacharacters; preserve the complete filename spelling.
    let mut result = String::with_capacity(filename.len());
    for character in filename.chars() {
        if "[]\\.^$*+?(){}|".contains(character) {
            result.push('\\');
        }
        // The escaped filename still matches the original literal character.
        result.push(character);
    }
    result
}

/// LLVM report envelope, retaining unrelated export metadata.
#[derive(serde::Deserialize, serde::Serialize)]
struct LlvmReport {
    /// One mapping set per instrumented profile.
    data: Vec<LlvmMapping>,
    /// LLVM export version and type fields.
    #[serde(flatten)]
    metadata: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Source mappings and summary totals for one instrumented profile.
#[derive(serde::Deserialize, serde::Serialize)]
struct LlvmMapping {
    /// Raw lexical filenames remain separate mappings.
    files: Vec<LlvmFile>,
    /// LLVM counters by metric name.
    totals: std::collections::BTreeMap<String, lcov::Metric>,
    /// Fields outside the summary profile are retained unchanged.
    #[serde(flatten)]
    metadata: std::collections::BTreeMap<String, serde_json::Value>,
}

/// One lexical source mapping and its LLVM metric counters.
#[derive(serde::Deserialize, serde::Serialize)]
struct LlvmFile {
    /// LLVM's original filename spelling.
    filename: PathBuf,
    /// Exact metric counts with approximate percentage projections.
    summary: std::collections::BTreeMap<String, lcov::Metric>,
    /// Additional mapping fields survive scope filtering.
    #[serde(flatten)]
    metadata: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Keep every lexical alias of a selected source and exclude every other
/// mapping.
fn scoped_ignore(
    summary: &LlvmReport,
    root: &Path,
    selected: &BTreeSet<PathBuf>,
) -> Result<Option<String>, Error> {
    if selected.is_empty() {
        return Ok(None);
    }
    // A scoped report requires an actual LLVM mapping envelope.
    let mapping = summary
        .data
        .first()
        .ok_or_else(|| Error::Invalid("LLVM summary has no mapping data".to_owned()))?;
    // Resolve each lexical alias before deciding whether it belongs to the scope.
    let mut excluded = Vec::new();
    for file in &mapping.files {
        if !selected.contains(&paths::canonical(&file.filename, root)?) {
            let filename = file
                .filename
                .to_str()
                .ok_or_else(|| Error::Invalid("LLVM filename is not UTF-8".to_owned()))?;
            excluded.push(escape_filename(filename));
        }
    }
    // Add exact escaped exclusions without weakening the shared fixture exclusions.
    let base =
        "(/\\.cargo/registry/|/rustc/|/nix/store/|/examples/|/ui/|/fixtures/|/fixture/|/target/)";
    Ok((!excluded.is_empty()).then(|| {
        let excluded = excluded.join("|");
        format!("{base}|{excluded}")
    }))
}

/// Filter raw summary mappings while preserving aliases and recomputing raw
/// totals.
fn filter_summary(
    summary: &mut LlvmReport,
    root: &Path,
    selected: &BTreeSet<PathBuf>,
) -> Result<(), Error> {
    if selected.is_empty() {
        return Ok(());
    }
    let mapping = summary
        .data
        .first_mut()
        .ok_or_else(|| Error::Invalid("LLVM summary has no mapping data".to_owned()))?;
    // Preserve every selected lexical mapping, including aliases of one source file.
    let mut retained = Vec::new();
    for file in mapping.files.drain(..) {
        if selected.contains(&paths::canonical(&file.filename, root)?) {
            retained.push(file);
        }
    }
    // Recompute raw mapping totals after removing unselected mappings.
    mapping.files = retained;
    for name in [
        "branches",
        "functions",
        "instantiations",
        "lines",
        "mcdc",
        "regions",
    ] {
        // Counts remain exact integers; Metric derives the approximate percentage.
        let count = mapping
            .files
            .iter()
            .filter_map(|file| file.summary.get(name))
            .map(|metric| metric.count)
            .sum();
        let covered = mapping
            .files
            .iter()
            .filter_map(|file| file.summary.get(name))
            .map(|metric| metric.covered)
            .sum();
        let _old = mapping
            .totals
            .insert(name.to_owned(), lcov::Metric::new(count, covered));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LlvmFile, LlvmMapping, LlvmReport, filter_summary, scoped_ignore};
    use std::{
        collections::BTreeMap, ffi::OsString, os::unix::ffi::OsStringExt as _, path::PathBuf,
    };

    /// Build one raw lexical mapping for scope-boundary tests.
    fn mapping(filename: PathBuf) -> LlvmReport {
        LlvmReport {
            data: vec![LlvmMapping {
                files: vec![LlvmFile {
                    filename,
                    summary: BTreeMap::new(),
                    metadata: BTreeMap::new(),
                }],
                totals: BTreeMap::new(),
                metadata: BTreeMap::new(),
            }],
            metadata: BTreeMap::new(),
        }
    }

    /// Empty mapping envelopes cannot support scoped reports.
    #[test]
    fn scoped_empty_mapping_fails() {
        // A real selection must not become an empty successful LLVM report.
        let root = tempfile::tempdir().expect("mapping fixture");
        let selected = std::iter::once(root.path().join("selected.rs")).collect();
        let mut report = LlvmReport {
            data: Vec::new(),
            metadata: BTreeMap::new(),
        };
        assert!(scoped_ignore(&report, root.path(), &selected).is_err());
        assert!(filter_summary(&mut report, root.path(), &selected).is_err());
    }

    /// LLVM exclusion expressions reject filenames that cannot be UTF-8 text.
    #[test]
    fn scoped_non_utf8_filename_fails() {
        // Filesystem identity can be valid while the LLVM text boundary rejects it.
        let root = tempfile::tempdir().expect("mapping fixture");
        let selected = std::iter::once(root.path().join("selected.rs")).collect();
        let report = mapping(root.path().join(OsString::from_vec(vec![0xff])));
        assert!(scoped_ignore(&report, root.path(), &selected).is_err());
    }

    /// Fully selected mappings need no additional filename exclusion.
    #[test]
    fn scoped_selected_mapping_has_no_exclusion() {
        // The selected lexical path resolves to the report's canonical identity.
        let root = tempfile::tempdir().expect("mapping fixture");
        let source = root.path().join("selected.rs");
        let selected = std::iter::once(source.clone()).collect();
        let report = mapping(source);
        assert!(
            scoped_ignore(&report, root.path(), &selected)
                .expect("selected mappings")
                .is_none()
        );
    }
}
