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
target_dir="$(realpath -m "$target_dir")"
report_dir="$target_dir/report"

if [[ "$target_dir" == "/" || "$target_dir" == "$repository_root" ||
	"$target_dir" == "$repository_root/target" ]]; then
	echo "COVERAGE_TARGET_DIR must name a dedicated coverage directory" >&2
	exit 2
fi

# Instrumented objects retain source mappings from the build that produced them.
# Remove the previous build before compiling so stale mappings cannot enter the report.
rm -rf "$target_dir/debug" "$target_dir/report" "$target_dir/profiles" \
	"$target_dir/profiles.txt" "$target_dir/coverage.profdata"
mkdir -p "$target_dir" "$report_dir"

# Build every crate, including the libraries dylint_testing builds, into one target directory.
export CARGO_TARGET_DIR="$target_dir"
export CARGO_INCREMENTAL=0
# Keep instrumented debug artifacts small by default while preserving overrides.
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-0}"
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

# Report against every instrumented executable and shared library. Reports include
# executable lines from production and test targets, including `cfg(test)` modules;
# they are not production-only. Example targets and test fixtures are excluded.
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

ignore='(/\.cargo/registry/|/rustc/|/nix/store/|/examples/|/ui/|/tests/fixtures/|/target/)'
common=(--instr-profile "$target_dir/coverage.profdata" --ignore-filename-regex "$ignore")

# llvm-cov records #[path] modules with their lexical source paths, such as
# `lint/src/../../shared.rs`. Normalize those paths before filtering the report.
llvm_cov_diagnostics="$report_dir/llvm-cov.stderr"
: >"$llvm_cov_diagnostics"
allowed_filenames_file=''
selected_sources_file=''
lexical_alias_count=0
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
		>"$all_summary_file" 2>>"$llvm_cov_diagnostics"
	while IFS= read -r filename; do
		filename_for_match="$filename"
		if [[ "$filename_for_match" != /* ]]; then
			filename_for_match="$repository_root/$filename_for_match"
		fi
		canonical_filename="$(realpath -m "$filename_for_match")"
		if grep -F -x -q "$canonical_filename" "$selected_sources_file"; then
			printf '%s\n' "$filename" >>"$allowed_filenames_file"
			if [[ "$filename_for_match" != "$canonical_filename" ]]; then
				lexical_alias_count=$((lexical_alias_count + 1))
			fi
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
		common=(--instr-profile "$target_dir/coverage.profdata"
			--ignore-filename-regex "$scoped_ignore")
	fi
fi

{
	"$llvm_bin/llvm-cov" report "${common[@]}" "${objects[@]}" >"$report_dir/summary.txt"
	"$llvm_bin/llvm-cov" export --format=lcov "${common[@]}" "${objects[@]}" \
		>"$report_dir/lcov.info"
	"$llvm_bin/llvm-cov" export --summary-only "${common[@]}" "${objects[@]}" \
		>"$report_dir/summary.json"
} 2>>"$llvm_cov_diagnostics"

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

# Collapse duplicated lexical mappings to canonical source files for the line threshold.
# LCOV DA records are executable lines; the maximum hit count wins when aliases overlap.
canonical_lines_file="$report_dir/canonical-lines.tsv"
canonical_summary_file="$report_dir/canonical_summary.json"
canonical_gaps_file="$report_dir/canonical_gaps.txt"
declare -A canonical_hits=()
current_source=''
while IFS= read -r record || [[ -n "$record" ]]; do
	case "$record" in
	SF:*)
		raw_source="${record#SF:}"
		source_for_match="$raw_source"
		if [[ "$source_for_match" != /* ]]; then
			source_for_match="$repository_root/$source_for_match"
		fi
		current_source="$(realpath -m -- "$source_for_match")"
		if [[ -n "$selected_sources_file" ]] &&
			! grep -F -x -q "$current_source" "$selected_sources_file"; then
			current_source=''
		fi
		;;
	DA:*)
		# LLVM 22.1.8 llvm-cov export --format=lcov emits SF:<source> and DA:<line>,<count>[,<checksum>].
		# SF is normalized above; line coverage uses the first two DA fields and ignores an optional checksum.
		if [[ -z "$current_source" ]]; then
			continue
		fi
		data="${record#DA:}"
		line_number="${data%%,*}"
		hit_count="${data#*,}"
		hit_count="${hit_count%%,*}"
		if [[ "$line_number" =~ ^[0-9]+$ && "$hit_count" =~ ^[0-9]+$ ]]; then
			line_number=$((10#$line_number))
			key="$current_source"$'\034'"$line_number"
			previous="${canonical_hits[$key]:-}"
			if [[ -z "$previous" ]]; then
				canonical_hits["$key"]="$hit_count"
			elif ((10#$hit_count > 10#$previous)); then
				canonical_hits["$key"]="$hit_count"
			fi
		fi
		;;
	end_of_record)
		current_source=''
		;;
	esac
done <"$report_dir/lcov.info"

for key in "${!canonical_hits[@]}"; do
	canonical_source="${key%$'\034'*}"
	line_number="${key##*$'\034'}"
	printf '%s\t%s\t%s\n' "$canonical_source" "$line_number" "${canonical_hits[$key]}"
done | LC_ALL=C sort -t $'\t' -k1,1 -k2,2n >"$canonical_lines_file"

jq -Rn '
    def metric($count; $covered): {
        count: $count,
        covered: $covered,
        notcovered: ($count - $covered),
        percent: (if $count == 0 then 0 else 100 * $covered / $count end)
    };
    [inputs
        | select(length > 0)
        | split("\t")
        | {
            filename: .[0],
            line: (.[1] | tonumber),
            hits: (.[2] | tonumber)
        }
    ]
    | group_by(.filename)
    | map(
        . as $entries
        | ($entries | map(select(.hits > 0)) | length) as $covered
        | {
            filename: $entries[0].filename,
            lines: metric(($entries | length); $covered),
            missing_lines: ($entries | map(select(.hits == 0) | .line))
        }
    ) as $files
    | {
        scope: "canonical executable LCOV DA lines from production and test targets; cfg(test) modules included; examples and tests/fixtures excluded",
        files: $files,
        totals: {
            lines: metric(
                ($files | map(.lines.count) | add // 0);
                ($files | map(.lines.covered) | add // 0)
            )
        }
    }
' "$canonical_lines_file" >"$canonical_summary_file"
jq -r '.files[] as $file | $file.missing_lines[]? | "\($file.filename):\(.)"' \
	"$canonical_summary_file" >"$canonical_gaps_file"

# Print the least covered files and the totals, then enforce the threshold.
jq -r '.data[0].files[]
    | [.summary.lines.percent, .summary.lines.count - .summary.lines.covered, .filename]
    | @tsv' "$report_dir/summary.json" | sort -n | sed -n '1,40p'
raw_lines="$(jq -r '.data[0].totals.lines.percent' "$report_dir/summary.json")"
regions="$(jq -r '.data[0].totals.regions.percent' "$report_dir/summary.json")"
lines="$(jq -r '.totals.lines.percent' "$canonical_summary_file")"
printf 'total line coverage: %.2f%%\ntotal region coverage: %.2f%%\n' "$lines" "$regions"
printf 'raw LLVM line coverage: %.2f%%\nraw LLVM region coverage: %.2f%%\n' \
	"$raw_lines" "$regions"
if ((lexical_alias_count > 0)); then
	printf 'coverage note: %d lexical #[path] aliases remain separate LLVM mappings; totals count each compiled copy.\n' \
		"$lexical_alias_count"
fi
printf 'canonical gaps: %s\nraw LLVM diagnostics: %s\nreports: %s\n' \
	"$canonical_gaps_file" "$llvm_cov_diagnostics" "$report_dir"
if [[ -s "$llvm_cov_diagnostics" ]]; then
	cat "$llvm_cov_diagnostics" >&2
fi
if ((test_status != 0)); then
	echo "tests failed during the coverage run" >&2
	exit "$test_status"
fi
if awk -v actual="$lines" -v minimum="$min_lines" 'BEGIN { exit !(actual < minimum) }'; then
	printf 'line coverage %.2f%% is below the %s%% minimum\n' "$lines" "$min_lines" >&2
	exit 1
fi
