# Interpreting metrics

The maintainability lints document their counting profiles and sources in the catalog. Cyclomatic complexity counts independent decisions. NPath estimates combinations of acyclic routes. Cognitive complexity weights nesting and control flow. ABC separates assignments, calls and conditions.

CRAP adds measured coverage to cyclomatic complexity. Compare values from the same analyzer version and source profile.

## Additional measurements

Halstead counts distinct and total operators and operands. Volume is `N × log₂(n)`, where `N` is the total count and `n` is the distinct vocabulary count. Difficulty is `(n₁ / 2) × (N₂ / n₂)`.
These dimensionless counts require a Rust-specific classification of macros, patterns, method calls and `?`.

Volume and difficulty can rank candidates after corpus calibration. Estimated time and defects are heuristic projections and must not become defect or duration claims. [Halstead metric definitions](https://dekobon.github.io/big-code-analysis/metrics.html#halstead).

The CK suite's inheritance depth and child count do not map directly to Rust traits. The existing type-method complexity, module fan-out and field-usage cohesion lints use explicit local Rust units. Package abstractness and main-sequence distance also need a validated Rust unit before implementation. [CK definitions](https://doi.org/10.1109/32.295895) and [Martin's experimental package metrics](https://objectmentor.com/resources/articles/stability.pdf).

Hotspots combine complexity with a bounded history window. Change coupling records files changed together. Choose rename, generated-file and bulk-formatting policies before comparing rankings. These history measurements belong in reports, because a compiler pass has no complete change history. [Code Maat](https://github.com/adamtornhill/code-maat).

Mutation tests measure whether assertions reject altered behavior. Line coverage records execution. Keep those results separate. Duplication detectors need a minimum clone size and Rust grammar fixtures. Unsafe and public API counts measure review surface, not correctness. [cargo-mutants](https://github.com/sourcefrog/cargo-mutants), [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov), and [cargo-geiger](https://github.com/geiger-rs/cargo-geiger).

## Adoption policy

Use the implemented lint profiles for deterministic compiler feedback. Before adding Halstead, package metrics, duplication or history gates, define their Rust semantics and test them against accepted and rejected code. Use reports for calibration. Require direct behavioral tests even when coverage reaches every changed line.
