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
        # llvm-cov takes source files, so expand the directory to its Rust sources.
        while IFS= read -r -d '' source; do
            paths+=("$source")
        done < <(find "$(realpath "$2")" -name '*.rs' ! -path '*/ui/*' ! -path '*/fixtures/*' -print0)
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

# Instrumented objects retain source mappings from the build that produced them.
# Remove the previous build before compiling so stale mappings cannot enter the report.
rm -rf "$target_dir/debug" "$target_dir/report" "$target_dir/profiles" \
    "$target_dir/profiles.txt" "$target_dir/coverage.profdata"
mkdir -p "$target_dir" "$report_dir"

# Build every crate, including the libraries dylint_testing builds, into one target directory.
export CARGO_TARGET_DIR="$target_dir"
export CARGO_INCREMENTAL=0
export RUSTFLAGS="${RUSTFLAGS:-} -C instrument-coverage"
export LLVM_PROFILE_FILE="$target_dir/profiles/%p-%m.profraw"
mkdir -p "$target_dir/profiles"

# Run every selected test even after a failure, then report and fail at the end.
test_status=0
cargo test --no-fail-fast "${test_args[@]}" || test_status=$?

# Merge the profiles written by test binaries, the runner, and Dylint driver processes.
llvm_bin="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin"
find "$target_dir/profiles" -name '*.profraw' >"$target_dir/profiles.txt"
"$llvm_bin/llvm-profdata" merge -sparse --input-files="$target_dir/profiles.txt" -o "$target_dir/coverage.profdata"

# Report against every instrumented executable and shared library.
# llvm-cov reads its first positional argument as a binary, so only later ones use --object.
objects=()
while IFS= read -r -d '' object; do
    if ((${#objects[@]} == 0)); then
        objects+=("$object")
    else
        objects+=(--object "$object")
    fi
done < <(find "$target_dir/debug" -maxdepth 2 -type f \( -name '*.so' -o -perm -u+x \) \
    ! -name '*.d' ! -name 'build-script-*' -print0)

ignore='(/\.cargo/registry/|/rustc/|/nix/store/|/ui/|/tests/fixtures/|/target/)'
common=(--instr-profile "$target_dir/coverage.profdata" --ignore-filename-regex "$ignore")

# llvm-cov records #[path] modules with their lexical source paths, such as
# `lint/src/../../shared.rs`. Normalize those paths before filtering the report.
allowed_filenames_file=''
if ((${#paths[@]} > 0)); then
    selected_sources_file="$(mktemp "$target_dir/selected-sources.XXXXXX")"
    all_summary_file="$(mktemp "$target_dir/all-summary.XXXXXX.json")"
    allowed_filenames_file="$(mktemp "$target_dir/allowed-filenames.XXXXXX")"
    trap 'rm -f "$selected_sources_file" "$all_summary_file" "$allowed_filenames_file"' EXIT

    for source in "${paths[@]}"; do
        realpath -m "$source"
    done | sort -u >"$selected_sources_file"

    # Build the candidate filename set before applying the scoped filter. This
    # retains each lexical alias that resolves to a selected source file.
    "$llvm_bin/llvm-cov" export --summary-only "${common[@]}" "${objects[@]}" \
        >"$all_summary_file" 2>/dev/null
    while IFS= read -r filename; do
        filename_for_match="$filename"
        if [[ "$filename_for_match" != /* ]]; then
            filename_for_match="$repository_root/$filename_for_match"
        fi
        canonical_filename="$(realpath -m "$filename_for_match")"
        if grep -F -x -q "$canonical_filename" "$selected_sources_file"; then
            printf '%s\n' "$filename" >>"$allowed_filenames_file"
        fi
    done < <(jq -r '.data[0].files[].filename' "$all_summary_file")

    # llvm-cov has no include-filename option. Exclude every discovered source
    # file outside the selected set, including lexical aliases of #[path] files.
    excluded_regex=''
    while IFS= read -r filename; do
        if grep -F -x -q "$filename" "$allowed_filenames_file"; then
            continue
        fi
        escaped_filename="$(printf '%s' "$filename" | sed 's/[][\\.^$*+?(){}|]/\\&/g')"
        if [[ -n "$excluded_regex" ]]; then
            excluded_regex+='|'
        fi
        excluded_regex+="$escaped_filename"
    done < <(jq -r '.data[0].files[].filename' "$all_summary_file")
    if [[ -n "$excluded_regex" ]]; then
        scoped_ignore="$ignore|$excluded_regex"
        common=(--instr-profile "$target_dir/coverage.profdata" \
            --ignore-filename-regex "$scoped_ignore")
    fi
fi

"$llvm_bin/llvm-cov" report "${common[@]}" "${objects[@]}" >"$report_dir/summary.txt" 2>/dev/null
"$llvm_bin/llvm-cov" export --format=lcov "${common[@]}" "${objects[@]}" >"$report_dir/lcov.info" 2>/dev/null
"$llvm_bin/llvm-cov" export --summary-only "${common[@]}" "${objects[@]}" >"$report_dir/summary.json" 2>/dev/null

if [[ -n "$allowed_filenames_file" ]]; then
    filtered_summary_file="$(mktemp "$target_dir/filtered-summary.XXXXXX.json")"
    jq --rawfile allowed "$allowed_filenames_file" '
        ($allowed | split("\n") | map(select(length > 0))) as $allowed_files
        | .data[0].files |= map(select(.filename as $filename
            | any($allowed_files[]; . == $filename)))
        | ["branches", "functions", "instantiations", "lines", "mcdc", "regions"] as $metrics
        | reduce $metrics[] as $metric (.;
            ([(.data[0].files[]?.summary[$metric].count // 0)] | add) // 0
                as $count
            | ([(.data[0].files[]?.summary[$metric].covered // 0)] | add) // 0
                as $covered
            | .data[0].totals[$metric] = {
                count: $count,
                covered: $covered,
                notcovered: ($count - $covered),
                percent: (if $count == 0 then 0 else 100 * $covered / $count end)
            }
        )
    ' "$report_dir/summary.json" >"$filtered_summary_file"
    mv "$filtered_summary_file" "$report_dir/summary.json"
fi

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
