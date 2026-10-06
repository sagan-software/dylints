# crap_score

## What it does

Warns when measured function complexity and executable-line coverage produce a CRAP score above the configured threshold.

## Why is this bad?

Uncovered decisions need tests and can increase change risk. The score ranks candidates for review; it does not predict defects.

## Known problems

Coverage must come from a representative run matching the analyzed source revision. Without configured coverage, this lint is inactive.

## Example

A function with complexity 6 and 0% measured coverage scores 42 and exceeds the default threshold 30.

## Use instead

Test the function's decisions or split independent responsibilities. Review assertions as well as coverage.

## Coverage profile and configuration

The formula is `complexity² × (1 − coverage_fraction)³ + complexity`. Complexity is a dimensionless decision count from the same HIR profile as `cyclomatic_complexity`. Coverage fraction is covered executable lines divided by measured executable lines. Multiplying this fraction by 100 converts it to the diagnostic's percentage. The default threshold is 30; equality passes.

The original Java metric used basis-path coverage. This Rust profile uses executable-line coverage and is not numerically interchangeable with that original profile. Line hits prove execution; they do not prove assertion quality.

Configure `[crap_score]` in `dylint.toml`: optional `coverage_path` is a local LCOV path; `threshold` is a finite nonnegative number. Relative report paths and `SF` paths resolve from the compiler working directory. Unknown configuration keys fail. Omitted `coverage_path` disables scoring.

An unreadable report or malformed `DA` record fails compilation. An unresolvable `SF` path fails compilation. An analyzed file absent from the
report or a function with no measured executable lines receives no score.
Threshold validation also runs when `coverage_path` is omitted.

The parser accepts LLVM 22.1.8 `SF:<path>` and `DA:<positive-line>,<unsigned-hits>[,<checksum>]` records. It ignores checksums and unrelated record kinds, including function and branch counters. `end_of_record` clears the active source. A `DA` record with extra fields fails.

Canonical filesystem paths merge source aliases; duplicate line records retain the maximum hit count. The callable's body start and end lines define an inclusive range in one file. Enclosing body ranges include nested closure lines. Functions sharing one source line share that line’s hit count.

Closures also have their own HIR body; macro-generated callables are excluded. Reports must match the analyzed source revision and relevant test scope; this lint cannot authenticate that match.

Sources: [original formula and cautions](https://www.artima.com/weblogs/viewpost.jsp?thread=210575), [original threshold guidance](https://www.artima.com/weblogs/viewpost.jsp?thread=215899), and [LLVM 22.1.8 LCOV exporter](https://github.com/llvm/llvm-project/blob/llvmorg-22.1.8/llvm/tools/llvm-cov/CoverageExporterLcov.cpp).

### A score of 1260

The observed function had cyclomatic complexity 35 and 0% coverage in the selected coverage run:

```text
CRAP = 35^2 * (1 - 0/100)^3 + 35
     = 1225 + 35
     = 1260
```

Cyclomatic complexity 35 means the analyzer found a large decision structure. Under the original definition, cyclomatic complexity is one plus the method's unique decisions. Exact counting differs between language analyzers. Compare the Rust value with other values from the same `cargo-crap` version.

The result is extreme because the formula squares uncovered complexity. `1260` is numerically 42 times the threshold. It does not mean 42 times as many defects, maintenance effort, or risk. The scale is nonlinear. The authors proposed it as an experimental ranking heuristic.

For the same complexity, coverage changes the score as follows:

| Coverage | CRAP |
| ---: | ---: |
| 0% | 1260.000 |
| 50% | 188.125 |
| 75% | 54.141 |
| 90% | 36.225 |
| 100% | 35.000 |

At 100% coverage, CRAP equals cyclomatic complexity. A function with complexity 35 therefore cannot reach 30 through coverage alone. It must reduce complexity to pass the absolute threshold.
