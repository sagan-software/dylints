#![feature(rustc_private)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the synchronous benchmark harness owns compiler processes and workload files"
)]
#![expect(
    unused_crate_dependencies,
    reason = "the benchmark uses only dependencies needed by its compiler workload"
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

use self::support::{Lint, file_size_group, lints, workload};
use criterion::{Criterion, SamplingMode, criterion_group, criterion_main};
use dylint_support::rust_file_size_violation;
use tempfile::TempDir;

/// Select the compact workload used by the under-one-minute inventory sweep.
const FAST_SWEEP_ENV: &str = "SAGAN_BENCH_FAST_SWEEP";

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
        .env("CARGO_BUILD_BUILD_DIR", &directory)
        .args(["build", "--lib", "--target-dir"])
        .arg(&directory);
    for lint in lints.iter_mut() {
        let filename = lint.library.file_name().expect("lint library filename");
        lint.library = directory.join("debug").join(filename);
        let _configured = command.args(["--package", &lint.package_spec]);
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
    let fast_sweep = env::var_os(FAST_SWEEP_ENV).is_some();
    let group_name = if fast_sweep {
        "compiler_fast"
    } else {
        "compiler"
    };
    let (small_functions, large_modules, large_functions) =
        if fast_sweep { (1, 4, 8) } else { (32, 8, 128) };
    let small = TempDir::new().expect("small workload directory");
    let small_source = workload(small.path(), 1, small_functions).expect("small compiler workload");
    let large = TempDir::new().expect("suite workload directory");
    let large_source =
        workload(large.path(), large_modules, large_functions).expect("suite compiler workload");
    let all: Vec<_> = lints.iter().collect();
    let mut group = criterion.benchmark_group(group_name);
    let aggregate_measurement_time = if fast_sweep {
        Duration::from_millis(10)
    } else {
        Duration::from_secs(5)
    };
    let _configured = group
        .sample_size(30)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(aggregate_measurement_time);
    for (name, libraries) in [("plain", &[][..]), ("all_lints", all.as_slice())] {
        let mut command = compiler(&driver, &large_source, libraries);
        let _configured = group.bench_function(name, |bencher| {
            bencher.iter(|| compile(&mut command));
        });
    }
    let per_lint_measurement_time = if fast_sweep {
        Duration::from_millis(10)
    } else {
        Duration::from_millis(300)
    };
    let _configured = group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(per_lint_measurement_time);
    for lint in &lints {
        let mut command = compiler(&driver, &small_source, &[lint]);
        let _configured = group.bench_function(&lint.name, |bencher| {
            bencher.iter(|| compile(&mut command));
        });
    }
    let lint = lints
        .iter()
        .find(|lint| lint.name == "large-rust-file")
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
    let fast_sweep = env::var_os(FAST_SWEEP_ENV).is_some();
    let mut group = criterion.benchmark_group(file_size_group(fast_sweep));
    if fast_sweep {
        let _configured = group.measurement_time(Duration::from_millis(10));
    }
    for (name, source) in [
        (
            "support",
            include_str!("../../crates/dylint-support/src/lib.rs").to_owned(),
        ),
        (
            "aggregate",
            include_str!("../../crates/sagan-lints/src/lib.rs").to_owned(),
        ),
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

/// Keep quick-sweep analysis cheaper while retaining full resampling by default.
fn criterion_config() -> Criterion {
    let resamples = if env::var_os(FAST_SWEEP_ENV).is_some() {
        1_000
    } else {
        10_000
    };
    Criterion::default().nresamples(resamples)
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = lint_passes, file_size
}
criterion_main!(benches);
