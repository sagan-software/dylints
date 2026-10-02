# Interpreting CRAP scores

Research date: 2026-08-19

## Practical reference

The CRAP metric combines a function's cyclomatic complexity with its automated-test coverage:

```text
CRAP(m) = comp(m)^2 * (1 - cov(m)/100)^3 + comp(m)
```

The original authors used `30` as an initial threshold after examining their own code, open-source projects, and peer opinions. Current `cargo-crap` 0.4.3 keeps `30` as its default and flags a function only when its score is greater than the configured threshold. The available primary sources define no authoritative low, medium, and high bands.

Use these reference points:

- `1` is the theoretical minimum: complexity 1 with 100% coverage.
- `<= 30` is the conventional acceptable region under the current Rust tool's strict `> 30` gate.
- `> 30` is a prompt to inspect the function's complexity, coverage, and change context.
- A score is not a defect probability, time estimate, or calibrated risk multiplier.

The threshold is a triage heuristic. Savoia described the original numerical choices as experimental starting points. He said a single metric cannot fully assess code. He also warned against using CRAP as a proxy for general code quality or programmer skill. High coverage can come from weak tests. A complex function can sometimes be clearer than several extracted functions.

Sources: [Savoia's original formula and cautions](https://www.artima.com/weblogs/viewpost.jsp?thread=210575), [original CRAP4J threshold guidance](https://www.artima.com/weblogs/viewpost.jsp?thread=215899), [`cargo-crap` 0.4.3 documentation](https://github.com/minikin/cargo-crap/blob/v0.4.3/README.md).

## What `1260` means

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

The earlier experiment deliberately ran only three `rust-lints-web` tests. Its 0% result means those selected tests did not execute this function. It does not prove that the repository's complete test suite gives the function 0% coverage.

## Rust-specific interpretation

The original CRAP formula used Java method complexity and basis-path coverage. `cargo-crap` applies the same formula to Rust functions using its Rust complexity analysis and LCOV input. The Rust score is therefore an implementation of the original heuristic, not a direct measurement from the original Java tooling.

The scope of coverage must match the scope of analysis. In version 0.4.3, `cargo-crap` documents that a function can lack coverage data because instrumentation omitted it or because coverage covered only part of a workspace. Its default `pessimistic` policy treats missing coverage data as 0%; `optimistic` treats it as 100%; `skip` removes the function. The tool recommends comparing a current report with a baseline so teams can catch regressions even when an absolute threshold is not yet practical.

Source: [`cargo-crap` 0.4.3 coverage, missing-data, and baseline behavior](https://github.com/minikin/cargo-crap/blob/v0.4.3/README.md#the---missing-policy).

## Recommended use in this repository

1. Before treating an absolute score as representative, use the complete relevant test suite.
2. Before interpreting 0% coverage, check that analyzed Rust files match the LCOV files.
3. Investigate every score above 30, starting with functions that changed or are likely to change.
4. For complexity above 30, reduce complexity because coverage alone cannot satisfy the default gate.
5. Track score regressions against a stable baseline after establishing a representative coverage run.
