#!/usr/bin/env bash
# Verify that coverage removes stale instrumented objects before reporting.
set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

fake_bin="$test_root/bin"
fake_sysroot="$test_root/sysroot"
coverage_target="$test_root/coverage"
mkdir -p "$fake_bin" "$fake_sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin" \
	"$coverage_target/debug"

printf 'stale object\n' >"$coverage_target/debug/stale-object"

cp "$repository_root/scripts/coverage-test-fixtures/cargo" "$fake_bin/cargo"
cp "$repository_root/scripts/coverage-test-fixtures/rustc" "$fake_bin/rustc"
cp "$repository_root/scripts/coverage-test-fixtures/llvm-profdata" \
	"$fake_sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"
cp "$repository_root/scripts/coverage-test-fixtures/llvm-cov" \
	"$fake_sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-cov"

chmod +x "$fake_bin/cargo" "$fake_bin/rustc" \
	"$fake_sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata" \
	"$fake_sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-cov"

export PATH="$fake_bin:$PATH"
export FAKE_CARGO_LOG="$test_root/cargo.log"
export FAKE_SYSROOT="$fake_sysroot"
export LLVM_COV_LOG="$test_root/llvm-cov.log"
export COVERAGE_TARGET_DIR="$coverage_target"
export FAKE_REPOSITORY_ROOT="$repository_root"

coverage_output="$test_root/coverage-output.log"
bash "$repository_root/scripts/coverage.sh" --min-lines 60.5 \
	--path "$repository_root/lints/complexity" \
	-- -p coverage-test --tests >"$coverage_output" 2>&1

if [[ -e "$coverage_target/debug/stale-object" ]]; then
	printf 'coverage retained a stale instrumented object\n' >&2
	exit 1
fi
if ! rg -q 'fresh-object' "$LLVM_COV_LOG"; then
	printf 'coverage did not report the fresh instrumented object\n' >&2
	exit 1
fi
if rg -q 'stale-object' "$LLVM_COV_LOG"; then
	printf 'coverage reported a stale instrumented object\n' >&2
	exit 1
fi
if ! rg -F -q 'src/runner' "$LLVM_COV_LOG"; then
	printf 'coverage did not exclude an unselected source file\n' >&2
	exit 1
fi
if rg -F -q 'example/src/../../chaining_lint_support.rs' "$LLVM_COV_LOG"; then
	printf 'coverage excluded a selected lexical source alias\n' >&2
	exit 1
fi
if ! rg -F -q '/examples/' "$LLVM_COV_LOG"; then
	printf 'coverage did not exclude example targets\n' >&2
	exit 1
fi
if [[ "$(jq '.data[0].files | length' "$coverage_target/report/summary.json")" != 2 ]]; then
	printf 'coverage did not retain both selected source mappings\n' >&2
	exit 1
fi
if jq -e --arg outside "$repository_root/src/runner.rs" \
	'any(.data[0].files[]; .filename == $outside)' \
	"$coverage_target/report/summary.json" >/dev/null; then
	printf 'coverage retained an unselected source mapping\n' >&2
	exit 1
fi
if ! rg -F -q 'coverage note: 1 lexical #[path] aliases remain separate LLVM mappings' \
	"$coverage_output"; then
	printf 'coverage did not report duplicated lexical mappings\n' >&2
	exit 1
fi
if ! rg -F -q 'RAW LLVM REPORT functions regions' "$coverage_target/report/summary.txt" ||
	! rg -F -q '44 functions have mismatched data' "$coverage_target/report/llvm-cov.stderr"; then
	printf 'coverage did not preserve raw LLVM report diagnostics\n' >&2
	exit 1
fi
if [[ "$(jq -r '.files | length' "$coverage_target/report/canonical_summary.json")" != 1 ]]; then
	printf 'canonical coverage did not merge lexical aliases\n' >&2
	exit 1
fi
canonical_count="$(jq -r '.totals.lines.count' "$coverage_target/report/canonical_summary.json")"
canonical_covered="$(jq -r '.totals.lines.covered' "$coverage_target/report/canonical_summary.json")"
if [[ "$canonical_count" != 3 || "$canonical_covered" != 2 ]]; then
	printf 'canonical coverage did not union disjoint alias hits\n' >&2
	exit 1
fi
if [[ "$(jq -c '.files[0].missing_lines' "$coverage_target/report/canonical_summary.json")" != '[30]' ]]; then
	printf 'canonical coverage did not retain the zero-hit line\n' >&2
	exit 1
fi
if jq -e --arg outside "$repository_root/src/runner.rs" \
	'any(.files[]; .filename == $outside)' \
	"$coverage_target/report/canonical_summary.json" >/dev/null; then
	printf 'canonical coverage retained an unselected source file\n' >&2
	exit 1
fi
if ! rg -F -q "$repository_root/lints/complexity/chaining_lint_support.rs:30" \
	"$coverage_target/report/canonical_gaps.txt"; then
	printf 'canonical coverage did not write a readable gap\n' >&2
	exit 1
fi
if ! rg -F -q 'total line coverage: 66.67%' "$coverage_output" ||
	! rg -F -q 'raw LLVM line coverage: 50.00%' "$coverage_output"; then
	printf 'coverage did not report canonical and raw line totals\n' >&2
	exit 1
fi

threshold_output="$test_root/threshold-output.log"
if bash "$repository_root/scripts/coverage.sh" --min-lines 67 \
	--path "$repository_root/lints/complexity" \
	-- -p coverage-test --tests >"$threshold_output" 2>&1; then
	printf 'coverage accepted a threshold above canonical coverage\n' >&2
	exit 1
fi
if ! rg -F -q 'line coverage 66.67% is below the 67% minimum' "$threshold_output"; then
	printf 'coverage threshold did not use canonical lines\n' >&2
	exit 1
fi

coverage_target_no_path="$test_root/no-path-coverage"
mkdir -p "$coverage_target_no_path/debug"
printf 'stale object\n' >"$coverage_target_no_path/debug/stale-object"
export COVERAGE_TARGET_DIR="$coverage_target_no_path"
coverage_output_no_path="$test_root/no-path-coverage-output.log"
bash "$repository_root/scripts/coverage.sh" -- -p coverage-test --tests \
	>"$coverage_output_no_path" 2>&1
if ! rg -F -q 'total line coverage: 75.00%' "$coverage_output_no_path" ||
	! rg -F -q 'raw LLVM line coverage: 2.17%' "$coverage_output_no_path"; then
	printf 'coverage did not report canonical lines beside the large raw report\n' >&2
	exit 1
fi
if [[ -e "$coverage_target_no_path/debug/stale-object" ]]; then
	printf 'coverage retained a stale object in the unscoped run\n' >&2
	exit 1
fi

if COVERAGE_TARGET_DIR="$repository_root" bash "$repository_root/scripts/coverage.sh" \
	-- -p coverage-test --tests >"$test_root/unsafe-target.log" 2>&1; then
	printf 'coverage accepted the repository root as its target\n' >&2
	exit 1
fi
if ! rg -F -q 'must name a dedicated coverage directory' "$test_root/unsafe-target.log"; then
	printf 'coverage did not reject an unsafe target\n' >&2
	exit 1
fi

symlink_path="$test_root/symlink-path"
symlink_target="$test_root/symlink-coverage"
mkdir -p "$symlink_path"
ln -s "$repository_root/lints/complexity/chaining_lint_support.rs" \
	"$symlink_path/source.rs"
: >"$FAKE_CARGO_LOG"
: >"$LLVM_COV_LOG"
symlink_output="$test_root/symlink-output.log"
COVERAGE_TARGET_DIR="$symlink_target" bash "$repository_root/scripts/coverage.sh" \
	--min-lines 60.5 --path "$symlink_path" \
	-- -p coverage-test --tests >"$symlink_output" 2>&1
if [[ ! -s "$FAKE_CARGO_LOG" ]]; then
	printf 'symlink scope did not run the selected build\n' >&2
	exit 1
fi
if [[ "$(jq '.data[0].files | length' "$symlink_target/report/summary.json")" != 2 ]]; then
	printf 'symlink scope did not retain canonical and lexical mappings\n' >&2
	exit 1
fi
if ! rg -F -q 'total line coverage: 66.67%' "$symlink_output"; then
	printf 'symlink scope did not report the selected canonical source\n' >&2
	exit 1
fi

sqlx_facade="$repository_root/lints/crates/sqlx/src/lib.rs"
sqlx_fixture="$repository_root/lints/crates/sqlx/fixture/src/lib.rs"
sqlx_report_target="$test_root/sqlx-report-target"
sqlx_report_output="$test_root/sqlx-report-output.log"
FAKE_SQLX_SCOPE=1 COVERAGE_TARGET_DIR="$sqlx_report_target" \
	bash "$repository_root/scripts/coverage.sh" -- -p coverage-test --tests \
	>"$sqlx_report_output" 2>&1
if jq -e --arg fixture "$sqlx_fixture" \
	'any(.data[0].files[]; .filename == $fixture)' \
	"$sqlx_report_target/report/summary.json" >/dev/null; then
	printf 'coverage report retained the auxiliary SQLx fixture\n' >&2
	exit 1
fi
if ! jq -e --arg facade "$sqlx_facade" \
	'any(.data[0].files[]; .filename == $facade)' \
	"$sqlx_report_target/report/summary.json" >/dev/null; then
	printf 'coverage report excluded the SQLx production facade\n' >&2
	exit 1
fi
if jq -e --arg fixture "$sqlx_fixture" \
	'any(.files[]; .filename == $fixture)' \
	"$sqlx_report_target/report/canonical_summary.json" >/dev/null; then
	printf 'canonical coverage retained the auxiliary SQLx fixture\n' >&2
	exit 1
fi
if ! jq -e --arg facade "$sqlx_facade" \
	'any(.files[]; .filename == $facade)' \
	"$sqlx_report_target/report/canonical_summary.json" >/dev/null; then
	printf 'canonical coverage excluded the SQLx production facade\n' >&2
	exit 1
fi
if ! rg -F -q '/lints/crates/sqlx/fixture/' "$LLVM_COV_LOG"; then
	printf 'coverage did not pass the exact SQLx fixture exclusion\n' >&2
	exit 1
fi
if ! rg -F -q 'lints/crates/sqlx/fixture excluded' \
	"$sqlx_report_target/report/canonical_summary.json"; then
	printf 'canonical coverage scope did not name the SQLx fixture exclusion\n' >&2
	exit 1
fi

sqlx_path_target="$test_root/sqlx-path-target"
sqlx_path_output="$test_root/sqlx-path-output.log"
FAKE_SQLX_SCOPE=1 COVERAGE_TARGET_DIR="$sqlx_path_target" \
	bash "$repository_root/scripts/coverage.sh" --min-lines 100 \
	--path "$repository_root/lints/crates/sqlx" -- -p coverage-test --tests \
	>"$sqlx_path_output" 2>&1
if [[ "$(jq '.data[0].files | length' "$sqlx_path_target/report/summary.json")" != 1 ]]; then
	printf 'SQLx path scope retained an excluded fixture or lost the facade\n' >&2
	exit 1
fi
if ! jq -e --arg facade "$sqlx_facade" \
	'any(.data[0].files[]; .filename == $facade)' \
	"$sqlx_path_target/report/summary.json" >/dev/null; then
	printf 'SQLx path scope did not retain the production facade\n' >&2
	exit 1
fi
if ! rg -F -q 'total line coverage: 100.00%' "$sqlx_path_output"; then
	printf 'SQLx path scope did not report the retained production facade\n' >&2
	exit 1
fi

assert_cli_rejects_before_run() {
	local name="$1"
	local expected="$2"
	shift 2
	local coverage_target="$test_root/$name-target"
	local output="$test_root/$name-output.log"
	mkdir -p "$coverage_target/debug"
	printf 'sentinel\n' >"$coverage_target/debug/sentinel"
	: >"$FAKE_CARGO_LOG"
	if COVERAGE_TARGET_DIR="$coverage_target" bash "$repository_root/scripts/coverage.sh" \
		"$@" >"$output" 2>&1; then
		printf '%s accepted invalid arguments\n' "$name" >&2
		exit 1
	fi
	if [[ -s "$FAKE_CARGO_LOG" ]]; then
		printf '%s ran Cargo before rejecting arguments\n' "$name" >&2
		exit 1
	fi
	if [[ ! -e "$coverage_target/debug/sentinel" ]]; then
		printf '%s cleaned the target before rejecting arguments\n' "$name" >&2
		exit 1
	fi
	if ! rg -F -q -- "$expected" "$output"; then
		printf '%s emitted an unexpected diagnostic\n' "$name" >&2
		cat "$output" >&2
		exit 1
	fi
}

assert_cli_rejects_before_run invalid-min-lines \
	'--min-lines must be a decimal percentage from 0 through 100' \
	--min-lines 0junk
assert_cli_rejects_before_run out-of-range-min-lines \
	'--min-lines must be a decimal percentage from 0 through 100' \
	--min-lines 100.1
assert_cli_rejects_before_run missing-min-lines \
	'--min-lines requires a value' \
	--min-lines
assert_cli_rejects_before_run missing-path \
	'--path must name an existing directory' \
	--path "$test_root/does-not-exist"
empty_path="$test_root/empty-path"
mkdir -p "$empty_path"
assert_cli_rejects_before_run empty-path \
	'--path must contain at least one selected Rust source file' \
	--path "$empty_path"
assert_cli_rejects_before_run tests-fixtures-path \
	'--path must contain at least one selected Rust source file' \
	--path "$repository_root/tests/fixtures"
assert_cli_rejects_before_run sqlx-fixture-path \
	'--path must contain at least one selected Rust source file' \
	--path "$repository_root/lints/crates/sqlx/fixture"
assert_cli_rejects_before_run missing-path-value \
	'--path requires a value' \
	--path
