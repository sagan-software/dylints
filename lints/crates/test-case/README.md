# test-case

Lints for the `test_case` and `test_matrix` attributes of the `test-case`
crate. They are written against test-case 3.3.1 and cover descriptions,
`ignore` and `panics` modifiers, output matchers, matrix inputs, async tests,
and leftovers from the test-case 2.0 migration.

## Lints

- [`test-case-async-without-test-harness`](test-case-async-without-test-harness)
- [`test-case-constant-match-guard`](test-case-constant-match-guard)
- [`test-case-empty-description`](test-case-empty-description)
- [`test-case-empty-ignore-reason`](test-case-empty-ignore-reason)
- [`test-case-empty-matrix`](test-case-empty-matrix)
- [`test-case-empty-panic-message`](test-case-empty-panic-message)
- [`test-case-ignore-without-reason`](test-case-ignore-without-reason)
- [`test-case-ignored-matrix`](test-case-ignored-matrix)
- [`test-case-inconclusive-modifier`](test-case-inconclusive-modifier)
- [`test-case-large-suite`](test-case-large-suite)
- [`test-case-legacy-inconclusive-description`](test-case-legacy-inconclusive-description)
- [`test-case-nonpositive-almost-precision`](test-case-nonpositive-almost-precision)
- [`test-case-panics-without-message`](test-case-panics-without-message)
- [`test-case-single-case-matrix`](test-case-single-case-matrix)
- [`test-case-singleton-contains-in-order`](test-case-singleton-contains-in-order)
- [`test-case-singleton-matrix-dimension`](test-case-singleton-matrix-dimension)
- [`test-case-unnamed-test-case`](test-case-unnamed-test-case)
- [`test-case-whitespace-ignore-reason`](test-case-whitespace-ignore-reason)
- [`test-case-whitespace-panic-message`](test-case-whitespace-panic-message)
- [`test-case-wildcard-match`](test-case-wildcard-match)
- [`test-case-with-function-path`](test-case-with-function-path)
