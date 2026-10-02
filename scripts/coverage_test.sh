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

bash "$repository_root/scripts/coverage.sh" --path "$repository_root/lints/complexity" \
    -- -p coverage-test --tests

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
