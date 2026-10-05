#![feature(rustc_private)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the synchronous benchmark harness owns compiler processes and workload files"
)]
#![allow(
    unused_crate_dependencies,
    reason = "the runner package embeds lint libraries that the benchmark harness loads dynamically"
)]

//! Criterion measurements of real compiler processes with independently loaded
//! lint passes.

extern crate rustc_driver as _;

mod support;

use std::{
    env,
    process::{Command, Stdio},
    time::Duration,
};

use self::support::{Lint, lints, workload};
use criterion::{Criterion, SamplingMode, criterion_group, criterion_main};
use dylint_support::rust_file_size_violation;
use tempfile::TempDir;

/// Execute one compiler check and reject failures instead of timing failed
/// compilations.
fn compile(command: &mut Command) {
    let status = command.status().expect("start pinned Dylint driver");
    if !status.success() {
        // Recover diagnostics only for failures; successful iterations keep terminal I/O excluded.
        let output = command
            .stderr(Stdio::piped())
            .output()
            .expect("capture compiler failure");
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        panic!("benchmark compiler failed: {status}\n{diagnostic}");
    }
}

/// Build one reusable command outside the measurement loop.
fn compiler(driver: &std::path::Path, source: &std::path::Path, libraries: &[&Lint]) -> Command {
    let mut command = Command::new(driver);
    let paths: Vec<_> = libraries.iter().map(|lint| &lint.library).collect();
    let _configured = command
        .current_dir(
            source
                .parent()
                .expect("source directory")
                .parent()
                .expect("workload directory"),
        )
        // A known empty configuration excludes repeated Cargo metadata discovery from analysis timings.
        .env("DYLINT_TOML", "")
        .env(
            "DYLINT_LIBS",
            serde_json::to_string(&paths).expect("library paths"),
        )
        .arg(source)
        .args([
            "--crate-name",
            "benchmark_workload",
            "--crate-type",
            "lib",
            "--edition=2024",
            "--emit=metadata",
        ])
        .arg("--out-dir")
        .arg(source.parent().expect("source directory"))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Resolve the pinned compiler and validate every discovered lint library.
fn inventory() -> (std::path::PathBuf, Vec<Lint>) {
    let metadata = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("Cargo metadata");
    assert!(metadata.status.success(), "Cargo metadata failed");
    let metadata = serde_json::from_slice(&metadata.stdout).expect("Cargo metadata JSON");
    let toolchain = env::var("RUSTUP_TOOLCHAIN").expect("run inside nix develop");
    let driver = std::path::PathBuf::from(
        env::var_os("DYLINT_DRIVER_PATH").expect("pinned driver directory"),
    )
    .join(&toolchain)
    .join("dylint-driver");
    assert!(driver.is_file(), "missing pinned Dylint driver");
    let mut lints = lints(&metadata, &toolchain);
    assert!(!lints.is_empty(), "no leaf lints found");
    prepare_libraries(&metadata, &mut lints);
    (driver, lints)
}

/// Build loadable leaf crates without category features contaminating their
/// artifact directory.
fn prepare_libraries(metadata: &serde_json::Value, lints: &mut [Lint]) {
    let directory = std::path::Path::new(
        metadata["target_directory"]
            .as_str()
            .expect("target directory"),
    )
    .join("lint-bench");
    let mut command = Command::new("cargo");
    let _configured = command
        .env_remove("CARGO_BUILD_BUILD_DIR")
        .args(["build", "--lib", "--target-dir"])
        .arg(&directory);
    for lint in lints.iter_mut() {
        let filename = lint.library.file_name().expect("lint library filename");
        lint.library = directory.join("debug").join(filename);
        let _configured = command.args(["--package", &lint.name]);
    }
    // Select only leaf packages so no category enables dylint_linting's constituent feature.
    let status = command.status().expect("build benchmark lint libraries");
    assert!(status.success(), "benchmark library build failed: {status}");
    assert!(
        lints.iter().all(|lint| lint.library.is_file()),
        "missing benchmark library"
    );
}

/// Register per-lint and overall compiler workloads from the validated
/// inventory.
fn lint_passes(criterion: &mut Criterion) {
    let (driver, lints) = inventory();
    let count = lints.len();
    println!(
        "Registered {count} lint-library benchmarks. Execution, triggering-case, and branch coverage are separate metrics."
    );
    let small = TempDir::new().expect("small workload directory");
    let small_source = workload(small.path(), 1, 32).expect("small compiler workload");
    let large = TempDir::new().expect("suite workload directory");
    let large_source = workload(large.path(), 8, 128).expect("suite compiler workload");
    let all: Vec<_> = lints.iter().collect();
    let mut group = criterion.benchmark_group("compiler");
    let _configured = group
        .sample_size(30)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5));
    for (name, libraries) in [("plain", &[][..]), ("all_lints", all.as_slice())] {
        let mut command = compiler(&driver, &large_source, libraries);
        let _configured = group.bench_function(name, |bencher| {
            bencher.iter(|| compile(&mut command));
        });
    }
    let _configured = group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(300));
    for lint in &lints {
        let mut command = compiler(&driver, &small_source, &[lint]);
        let _configured = group.bench_function(&lint.name, |bencher| {
            bencher.iter(|| compile(&mut command));
        });
    }
    let lint = lints
        .iter()
        .find(|lint| lint.name == "large_rust_file")
        .expect("file-size lint");
    let mut command = compiler(&driver, &small_source, &[lint]);
    let _configured = command.env_remove("DYLINT_TOML");
    let _configured = group.bench_function("config_discovery", |bencher| {
        bencher.iter(|| compile(&mut command));
    });
    group.finish();
}

/// Isolate the shared file-size policy on actual repository files and threshold
/// edges.
fn file_size(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("file_size");
    for (name, source) in [
        ("runner", include_str!("../src/runner.rs").to_owned()),
        ("driver", include_str!("../src/driver.rs").to_owned()),
        ("below_production_limit", "const _: () = ();\n".repeat(1499)),
        ("at_production_limit", "const _: () = ();\n".repeat(1500)),
        ("at_total_limit", "const _: () = ();\n".repeat(2000)),
        ("empty", String::new()),
    ] {
        let _configured = group.bench_function(name, |bencher| {
            bencher.iter(|| rust_file_size_violation(std::hint::black_box(&source)));
        });
    }
    group.finish();
}

/// Prepare workspace-only cleanup in the dedicated repository benchmark cache.
fn repository_clean(repo: &std::path::Path, target: &std::path::Path) -> Command {
    let output = Command::new("cargo")
        .current_dir(repo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("repository metadata");
    assert!(output.status.success());
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("metadata JSON");
    let mut clean = Command::new("cargo");
    let _configured = clean
        .env_remove("CARGO_BUILD_BUILD_DIR")
        .current_dir(repo)
        .args(["clean", "--target-dir"])
        .arg(target)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for package in metadata
        .get("packages")
        .expect("packages")
        .as_array()
        .expect("packages")
    {
        let package_id = package.get("id").expect("id").as_str().expect("package ID");
        let _configured = clean.args(["--package", package_id]);
    }
    clean
}

/// Measure fresh workspace checks with warm dependencies through the bundled
/// runner.
fn repository(criterion: &mut Criterion) {
    let Some(binary) = env::var_os("SAGAN_BENCH_RUNNER") else {
        return;
    };
    let repo = env::var_os("SAGAN_BENCH_REPOSITORY").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        std::path::PathBuf::from,
    );
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/repository-bench");
    let mut clean = repository_clean(&repo, &target);
    let mut group = criterion.benchmark_group("repository");
    let _configured = group
        .sample_size(10)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_secs(1));
    let _configured = group.bench_function("lint", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                // Remove only workspace artifacts in this benchmark's cache; retain external dependencies.
                compile(&mut clean);
                let mut command = Command::new(&binary);
                let _configured = command
                    .env_remove("CARGO_BUILD_BUILD_DIR")
                    .args([
                        "--heartbeat-seconds",
                        "0",
                        "--use-repo-clippy-config",
                        "--no-all-targets",
                        "--target-dir",
                    ])
                    .arg(&target)
                    .arg("--repo")
                    .arg(&repo)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let started = std::time::Instant::now();
                compile(&mut command);
                // Match self-lint's second pass without including UI example targets.
                let _configured = command.arg("--extra-cargo-arg=--tests");
                compile(&mut command);
                elapsed += started.elapsed();
            }
            elapsed
        });
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().nresamples(10_000);
    targets = lint_passes, file_size, repository
}
criterion_main!(benches);
