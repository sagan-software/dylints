#!/usr/bin/env bash
# Measure line and region coverage for the whole workspace.
#
# Dylint UI tests load each lint as a dynamic library inside a separate driver
# process, so `cargo llvm-cov`'s own report misses most lint code, and its rustc
# wrapper hides the `rustc` invocations that example-based UI tests parse. This
# script instead instruments every build through `RUSTFLAGS`, including the lint
# libraries that `dylint_testing` builds, then merges every profile and reports
# against every instrumented test binary and shared library.
#
# Usage: scripts/coverage.sh [--min-lines PERCENT] [--path DIR]... [-- CARGO_TEST_ARGS...]
# Without test arguments, it runs `cargo test --workspace --tests`. Pass package
# selections after `--` to measure part of the workspace, for example
# `-- -p ad_hoc_display --tests`, and `--path lints/style/ad_hoc_display` to
# limit the report to those directories. Doctests are not run: instrumented
# doctest builds cannot link the standard library twice.
# Environment: COVERAGE_TARGET_DIR (default: target/coverage).
set -euo pipefail

min_lines=0
test_args=(--workspace --tests)
paths=()
while (($# > 0)); do
    case "$1" in
    --min-lines)
        min_lines="$2"
        shift 2
        ;;
    --path)
        paths+=("$(realpath "$2")")
        shift 2
        ;;
    --)
        shift
        test_args=("$@")
        break
        ;;
    *)
        echo "usage: coverage.sh [--min-lines PERCENT] [--path DIR]... [-- CARGO_TEST_ARGS...]" >&2
        exit 2
        ;;
    esac
done

repository_root="$(git rev-parse --show-toplevel)"
target_dir="${COVERAGE_TARGET_DIR:-$repository_root/target/coverage}"
report_dir="$target_dir/report"
mkdir -p "$target_dir" "$report_dir"

# Build every crate, including the libraries dylint_testing builds, into one target directory.
export CARGO_TARGET_DIR="$target_dir"
export CARGO_INCREMENTAL=0
export RUSTFLAGS="${RUSTFLAGS:-} -C instrument-coverage"
export LLVM_PROFILE_FILE="$target_dir/profiles/%p-%m.profraw"
rm -rf "$target_dir/profiles"
mkdir -p "$target_dir/profiles"

# Run every selected test even after a failure, then report and fail at the end.
test_status=0
cargo test --no-fail-fast "${test_args[@]}" || test_status=$?

# Merge the profiles written by test binaries, the runner, and Dylint driver processes.
llvm_bin="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin"
find "$target_dir/profiles" -name '*.profraw' >"$target_dir/profiles.txt"
"$llvm_bin/llvm-profdata" merge -sparse --input-files="$target_dir/profiles.txt" -o "$target_dir/coverage.profdata"

# Report against every instrumented executable and shared library.
objects=()
while IFS= read -r -d '' object; do
    objects+=(--object "$object")
done < <(find "$target_dir/debug" -maxdepth 2 -type f \( -name '*.so' -o -perm -u+x \) \
    ! -name '*.d' ! -name 'build-script-*' -print0)

ignore='(/\.cargo/registry/|/rustc/|/nix/store/|/ui/|/tests/fixtures/|/target/)'
common=(--instr-profile "$target_dir/coverage.profdata" --ignore-filename-regex "$ignore")
"$llvm_bin/llvm-cov" report "${common[@]}" "${objects[@]}" "${paths[@]}" >"$report_dir/summary.txt" 2>/dev/null
"$llvm_bin/llvm-cov" export --format=lcov "${common[@]}" "${objects[@]}" "${paths[@]}" >"$report_dir/lcov.info" 2>/dev/null
"$llvm_bin/llvm-cov" export --summary-only "${common[@]}" "${objects[@]}" "${paths[@]}" >"$report_dir/summary.json" 2>/dev/null

# Print the least covered files and the totals, then enforce the threshold.
jq -r '.data[0].files[]
    | [.summary.lines.percent, .summary.lines.count - .summary.lines.covered, .filename]
    | @tsv' "$report_dir/summary.json" | sort -n | head -40
lines="$(jq -r '.data[0].totals.lines.percent' "$report_dir/summary.json")"
regions="$(jq -r '.data[0].totals.regions.percent' "$report_dir/summary.json")"
printf 'total line coverage: %.2f%%\ntotal region coverage: %.2f%%\n' "$lines" "$regions"
printf 'reports: %s\n' "$report_dir"
if ((test_status != 0)); then
    echo "tests failed during the coverage run" >&2
    exit "$test_status"
fi
if awk -v actual="$lines" -v minimum="$min_lines" 'BEGIN { exit !(actual < minimum) }'; then
    printf 'line coverage %.2f%% is below the %s%% minimum\n' "$lines" "$min_lines" >&2
    exit 1
fi
