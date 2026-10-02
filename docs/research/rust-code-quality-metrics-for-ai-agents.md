# Quantitative code-quality metrics for AI-generated Rust

Research date: 2026-08-19
Implementation update: 2026-08-20

## Recommendation

Use several small gates rather than one composite score. The strongest initial
set for this Rust lint suite is:

1. Keep the existing function-length, argument-count, type-complexity, file-size,
   and complicated-condition gates.
2. Use the implemented per-function Cyclomatic Complexity, source Cognitive
   Complexity, NPath, ABC, and exit-count gates.
3. Add `cargo-crap` after the coverage run represents the complete relevant test
   suite.
4. Reject cycles in module dependencies. Use the implemented fan-out and reachable public-surface gates. Calibrate their limits from accepted code.
5. Rank complexity-weighted churn and change coupling for review. Do not fail a
   build from their absolute values until the history window and generated-file
   policy are stable.
6. Track branch coverage, mutation score, new unsafe surface, and new lint
   exemptions independently.

The current Sagan runner enables `clippy::cognitive_complexity` with a threshold
of `15`. Clippy's current documentation says this lint does not calculate true
Cognitive Complexity and recommends `excessive_nesting` and `too_many_lines`
instead. Its default threshold is `25`. Keep the current `15` temporarily as a
secondary heuristic if policy stability matters, but do not label it a verified
Cognitive Complexity measurement. Replace that lint's role with a tested AST metric
before treating that number as a quality contract.

Sources:
[Clippy `cognitive_complexity`](https://rust-lang.github.io/rust-clippy/master/index.html#cognitive_complexity),
[Clippy configuration defaults](https://doc.rust-lang.org/nightly/clippy/lint_configuration.html),
[local runner policy](../../src/runner.rs), and
[local strict Clippy list](../../profiles/strict-clippy.lints).

## Implemented local profiles

The maintainability category now implements eleven Rust compiler lints. The
five additions from this update use these contracts:

- `abc_size` warns above vector magnitude `25`. It counts initialized bindings
  and assignments, calls, and source control-flow conditions. This Rust profile
  is not numerically interchangeable with analyzers that count every comparison.
- `many_exit_points` warns above `4`. It counts explicit `return` expressions
  and Rust `?` desugaring, but excludes normal fallthrough.
- `public_surface_size` warns above `25` reachable names in a reachable module.
  Direct public items and public re-export leaves count; associated items do not.
- `impl_method_count` warns above `20` source-authored inherent methods across
  all `impl` blocks for one local type. Trait and macro-generated methods do not.
- `field_usage_cohesion` reports only types with at least six measured receiver
  methods and four fields when two components each contain at least two methods
  and two fields. Shared fields and direct `self` calls connect methods.

The ABC, exit-count, and method-count limits follow the agent-feedback and Rust
profiles in the
[big-code-analysis threshold guide](https://dekobon.github.io/big-code-analysis/recipes/thresholds.html).
The public-surface limit is a local policy because its unit is a Rust module.

The project kept `field_usage_cohesion` only after its positive and negative fixtures
passed and calibration scans completed on `codex-claw`, `mdchat`, and `krustllm`
without candidates. The sampled code includes facades, adapters, builders, and
state types. This result found no false positives in the sample. It does not
prove that the graph profile is valid for every Rust design. Common LCOM
formulas have conflicting semantics and weak agreement with cohesion, so the
lint does not expose an LCOM score. Source:
[Al Dallal's comparison of LCOM variants](https://arxiv.org/abs/2012.12324).

## Terms in the quoted passage

### Cyclomatic Complexity

Cyclomatic Complexity measures the number of linearly independent paths through
a control-flow graph. McCabe defined it as `v(G) = E - N + 2P`, where `E` is the
edge count, `N` is the node count, and `P` is the number of connected components.
For one structured function, tools commonly calculate an equivalent form of one
plus the number of decisions.

It estimates the minimum basis-path testing burden. It does not count every
possible execution path, and it does not distinguish a flat decision table from
deep nesting. Counting details for `match`, guards, `?`, boolean operators,
closures, macros, and generated code differ between analyzers. Pin the analyzer
version and test its Rust grammar before making the value a gate.

McCabe described `10` as a reasonable upper bound, while warning that it was not
magical. A practical AI-agent policy is to fail new or changed functions above
`10`, baseline existing offenders, and require an explicit reviewed exemption.

Sources: [McCabe's 1976 paper](https://doi.org/10.1109/TSE.1976.233837) and
[the paper PDF](https://ics.uci.edu/~jajones/INF102-S18/readings/03_mccabe.pdf).

### Halstead metrics

Halstead treats a program as operators and operands:

- `n1` and `n2` are distinct operators and operands.
- `N1` and `N2` are total operators and operands.
- Vocabulary is `n = n1 + n2`.
- Length is `N = N1 + N2`.
- Volume is `V = N * log2(n)`.
- Difficulty is commonly `D = (n1 / 2) * (N2 / n2)`.
- Effort is `E = D * V`.

Time and predicted-defect formulas derive from those values. They are heuristic
projections, not observed time or defect counts. Operator classification is
language-specific, so Rust macros, method chains, turbofish syntax, pattern
matching, traits, and `?` require explicit analyzer rules.

Use Volume and Difficulty as warning and ranking signals. Do not block a change
from Halstead Effort, estimated time, or estimated bugs without local validation.
The values correlate strongly with code size, so they should not vote as if they
were independent evidence.

Sources: Halstead, *Elements of Software Science* (1977), and
[big-code-analysis metric definitions](https://dekobon.github.io/big-code-analysis/metrics.html#halstead).

### NPath Complexity

NPath counts acyclic execution paths through a function. It combines sequential
decisions multiplicatively, so it detects combinatorial path growth that
Cyclomatic Complexity can understate. A function with several independent
two-way decisions can have modest Cyclomatic Complexity and a large NPath value.

NPath is useful for testing burden and branch-combination risk. Its original
language rules have known edge cases, and no Rust compiler lint with mature support currently implements it. A Rust implementation must define `match`, guards, `if let`,
`while let`, `let else`, `?`, short-circuit operators, closures, and macro
expansions before its number is comparable across projects.

Use NPath as an advisory until those Rust semantics have fixtures. The Reddit
tool uses `400` as a default failure threshold, but that is a tool policy rather
than a research-backed universal limit.

Sources: [Nejmeh's 1988 paper](https://doi.org/10.1145/42372.42379) and
[ACPATH's analysis of NPath limitations](https://arxiv.org/abs/1610.07914).

### The CK suite

Chidamber and Kemerer defined six object-oriented design metrics:

- Weighted Methods per Class, or WMC, sums method weights. Cyclomatic
  Complexity is now a common weight.
- Depth of Inheritance Tree, or DIT, measures the longest path to a root class.
- Number of Children, or NOC, counts immediate subclasses.
- Coupling Between Object classes, or CBO, counts other classes coupled to a
  class.
- Response For a Class, or RFC, counts methods that can execute in response to a
  message to the class.
- Lack of Cohesion of Methods, or LCOM, measures how little methods share state.

WMC, coupling, response-set size, and cohesion have Rust analogues, but the implementation must choose the unit. A struct plus its inherent and trait `impl` blocks is one possible
unit. A module is often more useful because Rust permits free functions and
separates data from behavior. DIT and NOC do not transfer cleanly because trait
implementation and supertrait relationships are not class inheritance.

Do not import the CK suite unchanged. Start with module fan-out, type-level WMC,
method count, and public-method count. Treat any Rust CBO, RFC, or LCOM formula
as a local metric with a documented semantic contract.

Sources: [Chidamber and Kemerer's 1994 paper](https://doi.org/10.1109/32.295895) and
[big-code-analysis WMC documentation](https://dekobon.github.io/big-code-analysis/metrics.html#wmc).

### Distance from the Main Sequence

Robert Martin's package metrics use:

- Afferent coupling, `Ca`: incoming dependencies.
- Efferent coupling, `Ce`: outgoing dependencies.
- Instability, `I = Ce / (Ca + Ce)`.
- Abstractness, `A = abstract units / total units`.
- Normalized distance, `D' = |A + I - 1|`.

The Main Sequence is the line `A + I = 1`. A stable package with little
abstraction lies near the Zone of Pain. An unstable package with much abstraction
lies near the Zone of Uselessness. Distance alone loses that direction: both
corners can have the same `D'`. Report `A`, `I`, and `D'` together.

Rust needs a local definition of abstraction. Counting traits as abstract units
and structs or enums as concrete units is plausible but incomplete. Generic
APIs, sealed traits, trait objects, free functions, and internal modules affect
the meaning. Use this metric for workspace crates or intentional architectural
modules after defining those rules. Do not gate it before comparing the output
with known good and bad Rust designs.

Martin explicitly presented the metric as an experimental design standard and
warned against unconditional conformance.

Source: [Robert Martin, "Stability"](https://objectmentor.com/resources/articles/stability.pdf).

### Hotspot analysis

A hotspot combines static complexity with change frequency or churn. A complex
file that rarely changes may be costly but not urgent. A frequently changed file
that is simple may be healthy. A file with both properties is a high-value review
and refactoring target.

The history window, rename handling, generated files, bulk formatting commits,
and complexity measure change the ranking. Use a bounded window and exclude
generated and vendored code. Hotspots are prioritization data, not proof of a
defect.

Source: [Adam Tornhill's Code Maat](https://github.com/adamtornhill/code-maat).

### Cognitive Complexity

SonarSource designed Cognitive Complexity to approximate understandability. The
algorithm increments for breaks in linear flow, increments again for nesting,
and discounts some readable shorthand. A deeply nested branch scores more than
a flat sequence with the same Cyclomatic Complexity.

This makes it useful for review burden, but it is still a syntax heuristic. A
small score does not prove clear names, good domain boundaries, or simple data
flow. Rust analyzer behavior must cover `match`, guards, `?`, async blocks,
closures, labeled control flow, macros, and iterator chains.

Use a real implementation with `15` as an initial changed-function threshold.
Calibrate it against a corpus of accepted and rejected Rust functions. Do not use
Clippy's similarly named lint as the measurement source.

Sources:
[SonarSource's Cognitive Complexity paper](https://www.sonarsource.com/resources/cognitive-complexity/)
and
[Clippy's warning about its lint](https://rust-lang.github.io/rust-clippy/master/index.html#cognitive_complexity).

## Useful additions from the Reddit comments

The substantive comment thread adds three architecture metrics:

- Dependency cycle count. Production modules should have a dependency cycle count of `0`.
  `cargo modules dependencies --lib --acyclic` can fail when a Rust crate's
  internal dependency graph contains a cycle.
- Module fan-out. Count outgoing dependencies per module. A high value can
  identify an orchestration or god module, but generated adapters and facade
  modules need separate treatment. Start with regressions and percentile
  ranking.
- Change coupling. Count how often two files change in the same logical change.
  This can reveal an implicit dependency absent from the static graph. It differs
  from hotspots, which rank one entity by complexity and churn.

The comments also propose metric agreement. A module that has a cycle, high
fan-out, and unrelated change coupling is a stronger review target than a module
flagged by one heuristic.

Sources:
[the Reddit discussion](https://www.reddit.com/r/LLMDevs/comments/1stqeqn/how_40yearold_metrics_can_help_us_make_agentic/),
[`cargo-modules`](https://github.com/regexident/cargo-modules), and
[Code Maat logical coupling](https://github.com/adamtornhill/code-maat#mining-logical-coupling).

## Other metrics worth using for Rust

### CRAP

The Change Risk Anti-Patterns metric combines each function's Cyclomatic Complexity score
and coverage:

```text
CRAP(m) = comp(m)^2 * (1 - cov(m) / 100)^3 + comp(m)
```

This directly identifies untested complexity. Current `cargo-crap` defaults to a
threshold of `30`. The score depends on matching function coverage, so a partial
test run or missing LCOV entry can produce a misleading result. Use it only after
the coverage scope is representative. The existing CRAP research note in this
repository contains the detailed interpretation and local recommendation.

Sources: [`cargo-crap`](https://github.com/minikin/cargo-crap) and
[the local CRAP note](crap-metric.md).

### Branch coverage and mutation score

Line coverage proves that execution reached a line. Branch coverage shows which
control-flow outcomes ran. Mutation testing changes program behavior and checks
whether tests fail, so it measures assertion sensitivity that coverage cannot.

Use `cargo-llvm-cov` for line, region, and function coverage. Its branch coverage still requires nightly Rust, and its documentation calls the feature unstable. Use `cargo-mutants`
on changed packages or files in the fast loop, then run a broader scheduled job.
Track surviving mutants as a count and mutation score as a percentage. Never let
coverage compensate for surviving meaningful mutants.

Sources: [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) and
[`cargo-mutants`](https://github.com/sourcefrog/cargo-mutants).

### Maintainability Index and ABC

The Maintainability Index uses four inputs: Halstead Volume, Cyclomatic Complexity, lines of code, and sometimes comment density. It is useful for trends but double-counts
correlated size and complexity inputs. Multiple incompatible formulas exist.

ABC counts Assignments, Branches as calls, and Conditions. It detects long,
mechanical functions that can have low Cyclomatic Complexity. Use its three
components rather than only the vector magnitude, and compare values within the
same Rust analyzer version.

Use Maintainability Index as a report and regression signal. The local ABC lint
is a blocking gate only under the Rust profile and threshold documented above.

Source:
[big-code-analysis supported metrics](https://dekobon.github.io/big-code-analysis/metrics.html).

### Duplication and clone density

Track duplicated token or AST regions, clone count, and cloned-line percentage.
Duplication is useful for AI-generated handlers, adapters, and tests, where
plausible repetition can spread quickly. Exclude generated code and snapshots.
Require a minimum clone size so ordinary Rust boilerplate and derive-adjacent
patterns do not dominate the result.

Use clone density as a warning and reject new large clones in changed production
code. Validate any language-agnostic detector against macros and `impl` blocks.

### Rust safety surface

Track new unsafe blocks, unsafe functions, unsafe traits, unsafe impls, and unsafe
expressions. Also track undocumented unsafe blocks. `cargo-geiger` supplies
statistics for a crate and its dependencies, while Clippy can require safety
comments. These counts measure the audit surface. They do not measure actual unsafety.

Set the default change budget to zero new unsafe units. Allow additions only with
an explicit reviewed reason and focused safety tests.

Sources: [`cargo-geiger`](https://github.com/geiger-rs/cargo-geiger) and
[Clippy `undocumented_unsafe_blocks`](https://rust-lang.github.io/rust-clippy/master/index.html#undocumented_unsafe_blocks).

### Public API, dependency, and exemption growth

Count new public items, direct dependencies, duplicate dependency versions, lint
exemptions, baseline entries, and suppression markers. These are change budgets,
not universal quality scores. They are difficult for an agent to game silently
because each added unit is reviewable.

The current suite already rejects unreasoned Clippy allowances. Extend that
principle to every metrics exemption: require the metric name, reason, owner or
scope, and a condition for removal. A baseline should ratchet downward. Do not let an agent regenerate it automatically to make CI pass.

Rustdoc can report documented public-item and example percentages with
`-Z unstable-options --show-coverage`. Treat missing public documentation as a
binary lint gate when possible, since a percentage can hide which critical item
is missing.

Source:
[rustdoc coverage](https://doc.rust-lang.org/rustdoc/unstable-features.html#--show-coverage).

## Tool choices

### Best broad pilot: big-code-analysis

`big-code-analysis` is an active fork of Mozilla's `rust-code-analysis`. The `big-code-analysis` project published release `2.1.0` on 2026-08-07. Its `bca check` command supports per-function
thresholds, committed baselines, suppression auditing, CI exit codes, reports,
and Git-history metrics. It calculates Cyclomatic Complexity, Cognitive
Complexity, Halstead, ABC, Maintainability Index, LOC variants, argument and exit
counts, method and public-item counts, tokens, and WMC for Rust.

It is the closest current fit for an agent loop. Pin its version because grammar
updates and metric fixes can change values. Before adoption, add fixtures for the
Rust constructs named in this report and compare its results with manually
calculated examples.

Sources: [`big-code-analysis`](https://github.com/dekobon/big-code-analysis),
[baseline workflow](https://dekobon.github.io/big-code-analysis/recipes/baselines.html), and
[metric definitions](https://dekobon.github.io/big-code-analysis/metrics.html).

### Lower-level option: rust-code-analysis

Mozilla's `rust-code-analysis` supports Rust and exports Cyclomatic Complexity,
Cognitive Complexity, Halstead, Maintainability Index, ABC, LOC, argument,
exit, method, public-item, and WMC data. The latest `rust-code-analysis` release, `0.0.25`, dates from 2023-01-13. It is a metrics engine, so this repository would need to own threshold
policy, baselines, changed-function identity, output normalization, and CI
diagnostics.

Source: [`rust-code-analysis`](https://github.com/mozilla/rust-code-analysis).

### Original Reddit tool: slop

The post's `slop` tool reports that it supports Rust complexity, hotspots, package metrics, and class metrics, but not Rust dependency analysis. The repository dates from April 2026, and it had 38 stars when inspected. It is useful as a source
of ideas and fixtures. Its Rust CK and package projections need the semantic
validation described above before this lint suite relies on them.

Source: [`agent-slop-lint`](https://github.com/JordanGunn/agent-slop-lint).

## Proposed adoption order

1. Document that the current Clippy `cognitive_complexity = 15` gate is a
   secondary heuristic. Enable `clippy::excessive_nesting` with a locally tested
   nesting threshold and keep `too_many_lines = 50` in the consumer runner.
2. Pilot pinned `bca check` on Rust production paths. Start with Cyclomatic
   Complexity `10`, Cognitive Complexity `15`, and the existing function-length
   and argument limits. Write a reviewed baseline for existing offenders.
3. Add fixtures for Rust grammar forms: `match`, guards, `?`, `let else`, async blocks, closures, labeled flow, macro calls, and generated code. Do not promote the
   gate until each expected score is stable.
4. Add `cargo-crap` with threshold `30` after a representative coverage run.
   Fail new offenders and score regressions. Keep partial focused runs advisory.
5. Add `cargo modules dependencies --lib --acyclic`. Report module fan-out and
   public API growth without absolute failure thresholds for one calibration
   period.
6. Add bounded hotspot and change-coupling reports. Review the top-ranked changed
   entities rather than enforcing an unexplained scalar.
7. Add changed-package mutation testing, unsafe-surface budgets, and exemption
   counts. Make every new exemption a visible reviewed change.

This order gives agents fast deterministic feedback while keeping readability, test burden, architecture, history, test quality, and Rust safety as separate signals. A low value in one category must not cancel a failure in another.
